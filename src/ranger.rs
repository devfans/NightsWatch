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
use tokio::timer::delay;
use std::time::{Duration, Instant};
use tokio_net::process::Command;
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
}

pub struct Target {
    id: u16,
    check_prog: String,
    check_args: Vec<String>,
    check_type: TargetCheckType,
    default_health: u8,
    paths: Vec<String>,
    name: String,
    interval: u64,
    extra: Value,

    state: Arc<Mutex<State>>,
}

impl Target {
    pub async fn check_health(&self, health_status: &mut u8) -> AsyncRes {
        let mut success = false;
        *health_status = self.default_health;

        if self.check_prog.len() < 1 { return Ok(()); }

        let mut bin = Command::new(self.check_prog.clone());
        let cmd = bin.args(&self.check_args);

        if let TargetCheckType::WatchOutput = self.check_type {
            match cmd.output().await {
                Ok(res) => {
                    success = true;
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

                            info!("{}", output);
                        },
                        Err(e) => { error!("Failed to convert check output to string, {}", e); }
                    }
                },
                _ => {}
            }
        } else {
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
                    if info["watch"].get_str("type", "") == "watch_exit" {
                        target.check_type = TargetCheckType::WatchExit;
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
            let _ = messenger.try_send(Dracarys::Target {
                id: target.id,
                name: target.name.clone(),
                paths: target.paths.clone(),
                extra: target.extra.to_string(),
            });
            let mut last_check: u64 = 0;
            let mut health_status: u8 = 0;
            let interval = target.interval;
            loop {
                let sleep_s = (last_check + interval) as i64 - utils::now() as i64;
                if sleep_s > 0 {
                    delay(Instant::now() + Duration::from_millis(1000 * sleep_s as u64)).await;
                }
                last_check = utils::now();
                match target.check_health(&mut health_status).await {
                    Ok(_) => {
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
                        let _ = messenger.try_send(Dracarys::Report {
                            id: target.id,
                            health_status,
                        });
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
        info!("Ranger lost connection for now!");
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

