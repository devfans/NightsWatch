/*  MIT License

Copyright (c) 2019 Stefan Liu - NightsWatch

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
*/

use std::collections::{VecDeque, HashMap};
use serde_json::Value;
use crate::utils::{JsonParser, AsyncRes};
use crate::knight::*;
use crate::dracarys::{Dracarys, DracarysFramer};
use tokio::sync::mpsc;
use std::sync::{Arc, Mutex};
use crate::utils;
use std::time::{Duration, Instant};
use tokio::process::Command;
// Sample configuration
//
// nightfort: 127.0.0.1:6000
// targets:
//  - watch:
//      program:
//      args: []
//      type:
//    default_health: 0
//    paths:
//      - .app1.service1
//      - .app2.service2
//    name: pod1
//    interval: 10
//    extra:
//      display_name: ""
//      description: ""
//      ...
//
//  - watch:
//  ...
//    
//
type Messenger = mpsc::UnboundedSender<Dracarys>;

pub struct State {
    last_check: u64,
    health_status: u8,
    health_history: VecDeque<u8>,
}

pub enum TargetCheckType {
    WatchExit,
    WatchOutput,
    WatchExitAndMetric,
    WatchMetric,
}

pub struct Target {
    id: u16,
    check_prog: String,
    check_args: Vec<String>,
    check_type: TargetCheckType,
    relative_metric_path: bool,
    default_health: u8,
    paths: Vec<String>,
    name: String,
    interval: u64,
    extra: Value,

    state: Arc<Mutex<State>>,
}

impl Target {
    pub async fn check_health(&self, health_status: &mut u8, metrics: &mut Vec<(String, String)>) -> AsyncRes {
        let mut success = false;
        *health_status = self.default_health;

        if self.check_prog.len() < 1 { return Ok(()); }

        let mut bin = Command::new(self.check_prog.clone());
        let cmd = bin.args(&self.check_args);

        let mut check_metrics = false;
        let mut output_status = false;
        let mut check_output = false;
        let mut check_exit = false;
        match self.check_type {
            TargetCheckType::WatchOutput => {
                check_output = true;
                output_status = true;
            },
            TargetCheckType::WatchExit => {
                check_exit = true;
            },
            TargetCheckType::WatchExitAndMetric => {
                check_exit = true;
                check_output = true;
                check_metrics = true;
            },
            TargetCheckType::WatchMetric => {
                check_metrics = true;
                check_output = true;
            }
        }

        if check_output {
            match cmd.output().await {
                Ok(res) => {
                    success = true;
                    // Check health status from exit code
                    if check_exit {
                        *health_status = res.status.code().unwrap_or(self.default_health as i32) as u8;
                    }

                    // Check health status from first 100 bytes of the stdout
                    if output_status {
                        let mut max = 100;
                        if res.stdout.len() < max { max = res.stdout.len() }
                        let slice = &res.stdout[..max];
                        match String::from_utf8(slice.to_vec()) {
                            Ok(output_str) => {
                                let output = output_str.trim();
                                // info!("checking output {}", String::from_utf8(res.stdout).unwrap());
                                match output.parse::<u8>() {
                                    Ok(health) => { *health_status = health; }
                                    _ => error!("Failed to parse health status from output: {}", output),
                                }

                                // info!("{}", output);
                            },
                            Err(e) => { error!("Failed to convert check output to string, {}", e); }
                        }
                    }

                    // Collect Metrics from stdoutput
                    if check_metrics {
                        match String::from_utf8(res.stdout) {
                            Ok(output_str) => {
                                for line in output_str.split("\n") {
                                    let tokens: Vec<&str> = line.split(",").collect();
                                    if tokens.len() > 1 {
                                        metrics.push((format!(".{}", tokens[0].trim()), tokens[1].trim().to_string()));
                                    }
                                }
                            },
                            Err(e) => { error!("Failed to convert check output to string, {}", e); }
                        }
                    }

                },
                _ => {}
            }
        } else if check_exit {
            if let Ok(child) = cmd.spawn() {
                match child.await {
                    Ok(status) => {
                        success = true;
                        *health_status = status.code().unwrap_or(self.default_health as i32) as u8;
                    },
                    _ => {}
                }
            }
        }
        if !success {
            error!("Failed to run check command! {}, {:?}", self.check_prog, self.check_args);
        }
        Ok(())
    }
}


pub struct Map {
    pub nightfort: String,
    pub map: HashMap<u16, Arc<Target>>,
}

impl Map {
    pub fn new(raw: &Value) -> Map {
        let mut map = HashMap::new(); 
        let nightfort = raw.get_str("nightfort", "127.0.0.1:6000");
        if let Some(targets) = raw["targets"].as_array() {
            for (index, info) in targets.iter().enumerate() {
                // target id
                let target_id = index as u16;

                // target path
                let mut paths = Vec::new();
                if let Some(paths_info) = info["paths"].as_array() {
                    for path in paths_info.iter() {
                        if let Some(path) = path.as_str() {
                            paths.push(path.to_string());
                        }
                    }
                }
                if paths.len() < 1 {
                    error!("At least one invalid parent path should be specified for leaf node");
                    continue;
                }

                // target stats
                let state = State {
                    last_check: 0,
                    health_status: 0,
                    health_history: VecDeque::new(),
                };

                // target body
                let mut target = Target {
                    id: target_id,
                    check_prog: String::new(),
                    check_type: TargetCheckType::WatchOutput,
                    relative_metric_path: info.get_bool("relative_metric_path", true),
                    check_args: Vec::new(),
                    name: info.get_str("name", "new-leaf-node"),
                    interval: info.get_u64("interval", 10),
                    extra: info["extra"].clone(),
                    paths,
                    default_health: info.get_u64("default_health", 0) as u8,
                    state: Arc::new(Mutex::new(state)),
                };

                // target check
                if info["watch"].is_object() {
                    target.check_prog = info["watch"].get_str("prog", "");

                    let check_type = info["watch"].get_str("type", "");
                    if check_type == "watch_exit" {
                        // Only check exit code as health status
                        target.check_type = TargetCheckType::WatchExit;
                    } else if check_type == "watch_metrics" {
                        // Only check output as metrics
                        target.check_type = TargetCheckType::WatchMetric;
                    } else if check_type == "watch_exit_and_metrics" {
                        // Check exit code as health status and collect metrics from output
                        target.check_type = TargetCheckType::WatchExitAndMetric;
                    }
                    if let Some(args) = info["watch"]["args"].as_array() {
                        for arg in args.iter() {
                            if let Some(arg) = arg.as_str() {
                                target.check_args.push(arg.to_string());
                            }
                        }
                    }
                }

                // insert target into map
                map.insert(target_id, Arc::new(target));
            }
        }
        Map {
            nightfort,
            map,
        }
    }
}

pub struct Ranger {
    map: Map,
}

impl Ranger {
    fn start_watch(&self, messenger: Messenger) {
        for target in self.map.map.values() {
            Self::watch_target(target.clone(), messenger.clone());
        }
    }

    pub fn watch_target(target: Arc<Target>, mut messenger: Messenger) {
        tokio::spawn(async move {
            // Send target info
            let _ = messenger.send(Dracarys::Target {
                id: target.id,
                name: target.name.clone(),
                paths: target.paths.clone(),
                extra: target.extra.to_string(),
            });
            let mut last_check: u64 = 0;
            let mut health_status: u8 = 0;
            let interval = target.interval;
            let check_health_status = match target.check_type {
                TargetCheckType::WatchMetric => false,
                _ => true
            };
            loop {
                let sleep_s = (last_check + interval) as i64 - utils::now() as i64;
                if sleep_s > 0 {
                    sleep!(1000 * sleep_s as u64);
                }
                last_check = utils::now();
                let mut metrics = Vec::new();
                match target.check_health(&mut health_status, &mut metrics).await {
                    Ok(_) => {
                        if check_health_status {
                            {
                                let mut state = target.state.lock().unwrap();
                                state.last_check = last_check;
                                state.health_status = health_status;
                                state.health_history.push_back(health_status);
                                if state.health_history.len() > 50 {
                                    state.health_history.pop_front();
                                }
                            }
                            // Send report
                            let _ = messenger.send(Dracarys::Report {
                                id: target.id,
                                health_status,
                            });
                        }
        
                        if metrics.len() > 0 {
                            let now = utils::now();
                            let mut data = Vec::new();
                            for m in metrics.iter() {
                                data.push((m.0.clone(), m.1.clone(), now));
                            }
                            let _ = messenger.send(Dracarys::Metric {
                                id: target.id,
                                relative: target.relative_metric_path,
                                metrics: data,
                            });
                        }
                    },
                    Err(e) => {
                        error!("Failed to check target health for failed script execution, error: {:?}", e);
                    },
                }
            }

        });
    }
}

impl Wine<Dracarys> for Arc<Ranger> {
    type Stream = mpsc::UnboundedReceiver<Dracarys>;
    type Framer = DracarysFramer;
    fn get_framer(&self) -> Self::Framer {
        DracarysFramer::new()
    }

    fn drink(&self, msg: Dracarys) {
        info!("Received message for the ranger: {:?}", msg);
    }

    fn take_nap(&self) {
        info!("Ranger lost connection for the moment!");
    }

    fn wake_up(&self) -> Self::Stream {
        info!("Ranger gets connected with Nightfort");
        let (tx, rx) = mpsc::unbounded_channel();
        // self.messenger = Some(tx);
        self.start_watch(tx);
        rx
    }
}

impl Ranger {
    pub fn new(map: &Value) -> Ranger {
        Ranger {
            map: Map::new(map),
        }
    }

    pub async fn start(self) -> AsyncRes {
        let ranger = Arc::new(self);
        // tokio::spawn(async move {
        Knight::new(&ranger.map.nightfort, ranger.clone()).drink_wine().await.unwrap();
        // });
        Ok(())
    }
}

