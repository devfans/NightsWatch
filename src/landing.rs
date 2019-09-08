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
