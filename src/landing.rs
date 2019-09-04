
use serde::{Deserialize, Serialize};
use serde_json;
use serde_json::Value;

#[derive(Serialize, Deserialize)]
pub struct Landing {
    pub nightfort_listen_bind: String,
    pub watcher_tick_interval: usize,
}


impl Landing {
    pub fn new() -> Landing {
        Landing {
            nightfort_listen_bind: "0.0.0.0:6000".to_string(),
            watcher_tick_interval: 10,
        }
    }

    pub fn dump(&self) -> serde_json::Result<String> {
        let string = serde_json::to_string(self)?;
        Ok(string)
    }
    
    pub fn parse(&mut self, raw:& Value) {
        if let Some(watcher_tick_interval) = raw["watcher_tick_interval"].as_u64() {
            self.watcher_tick_interval = watcher_tick_interval as usize;
        }
        if let Some(nightfort_listen_bind) = raw["nightfort_listen_bind"].as_str() {
            self.nightfort_listen_bind = nightfort_listen_bind.to_string();
        }
    }
}
