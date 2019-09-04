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
use tokio_net::process;
// Sample configuration
//
// nightfort: 127.0.0.1:6000
// targets:
//  1:
//    check_script:
//    paths:
//      - .app1.service1
//      - .app2.service2
//    name: pod1
//    interval: 10
//    extra:
//      display_name: ""
//      description: ""
//      ...
//  2:
//  ...
//    
//
type Messenger = mpsc::UnboundedSender<Dracarys>;

pub struct State {
    last_check: u64,
    health_status: u8,
    health_history: VecDeque<u8>,
}

pub struct Target {
    id: u16,
    check_script: String,
    paths: Vec<String>,
    name: String,
    interval: u64,
    extra: Value,

    state: Arc<Mutex<State>>,
}

impl Target {
    pub async fn check_health(&self, health_status: &mut u8) -> AsyncRes {
        *health_status = 100;
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
        if let Some(targets) = raw["targets"].as_object() {
            for (id, info) in targets.iter() {
                let target_id: u16;
                match id.parse::<u16>() {
                    Ok(id) => {
                        target_id = id;
                    },
                    Err(_) => {
                        error!("Invalid target id: {}", id);
                        continue;
                    },
                }
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
                let state = State {
                    last_check: 0,
                    health_status: 0,
                    health_history: VecDeque::new(),
                };
                let target = Target {
                    id: target_id,
                    check_script: info.get_str("check_script", ""),
                    name: info.get_str("name", "new-leaf-node"),
                    interval: info.get_u64("interval", 10),
                    extra: info["extra"].clone(),
                    paths,
                    state: Arc::new(Mutex::new(state)),
                };
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
