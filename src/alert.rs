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
use crate::eval::NodeHealth;
use serde_json::Value;

#[derive(Debug)]
#[derive(Clone)]
pub struct Alert {
    pub name: String,
    pub application: String,
    pub source: Weak<Node>,
    pub path: String,
    pub timestamp: u64,
    pub severity: u8,
    pub status: u8,
    pub description: String,
}

impl From<&Alert> for Value {
    fn from(a: &Alert) -> Value {
        json!({
            "name": a.name,
            "application": a.application,
            "path": a.path,
            "time": a.timestamp,
            "severity": a.severity,
            "status": a.status,
            "description": a.description 
        })
    }
}

