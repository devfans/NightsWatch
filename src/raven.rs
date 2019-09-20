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

use crate::event::Event;
use crate::alert::Alert;
use serde_json::{self, Value};

pub enum RavenMessage<'a> {
    TakeSnapshot,
    LoadSnapshot,
    NewAlert {
        data: &'a Alert,
    },
    NewEvent {
        data: &'a Event,
    },
    None,
}

impl<'a> From<&RavenMessage<'a>> for Value {
    fn from(m: &RavenMessage) -> Value {
        match m {
            RavenMessage::TakeSnapshot => {
                json!({"method": "take_snapshot"})
            },
            RavenMessage::LoadSnapshot => {
                json!({"method": "load_snapshot"})
            },
            RavenMessage::NewAlert { data } => {
                let data: Value = (*data).into();
                json!({
                    "method": "new_alert",
                    "data": data
                })
            },
            RavenMessage::NewEvent { data } => {
                let data: Value = (*data).into();
                json!({
                    "method": "new_event",
                    "data": data
                })
            },
            _ => unimplemented!()
        }
    }
}

impl<'a> From<&String> for RavenMessage<'a> {
    fn from(m: &String) -> RavenMessage<'a> {
        match serde_json::from_str::<Value>(m) {
            Ok(value) => {
                match value["method"].as_str() {
                    Some(method) => {
                        if method == "take_snapshot" {
                            RavenMessage::TakeSnapshot
                        } else {
                            error!("Unknown rave message method: {}", method);
                            RavenMessage::None
                        }
                    },
                    None => {
                        error!("Invalid raven message method: {}", m);
                        RavenMessage::None
                    }
                }
            },
            Err(_) => {
                error!("Invalid raw raven message: {}", m);
                RavenMessage::None
            }
        }
    }
}

