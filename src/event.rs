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

use std::sync::Weak;
use crate::node::*;
use serde_json::Value;

#[derive(Debug)]
#[derive(Clone)]
pub enum EventType {
    HealthDowngrade,
    HealthUpgrade,
    NodeJoin,
    NodeLeft,
}

#[derive(Debug)]
#[derive(Clone)]
pub struct Event {
    event_type: EventType,
    desc: String,
    source: Weak<Node>,
    application: String,
    timestamp: u64,
    path: String,
}

impl From<&Event> for Value {
    fn from(e: &Event) -> Value {
        json!({
            "type": e.event_type.to_string(),
            "description": e.desc,
            "application": e.application,
            "time": e.timestamp,
            "path": e.path
        })
    }
}

