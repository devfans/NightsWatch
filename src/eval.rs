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

use rhai::{Engine, RegisterFn };
use std::sync::{RwLock, Arc, Mutex};
use crate::node::*;
use std::collections::HashMap;
use std::fmt::Display;

fn showit<T: Display>(x: &mut T) -> () {
    println!("{}", x)
}

#[derive(Debug)]
#[derive(Clone)]
pub struct NodeHealth {
    id: u64,
    kids: HashMap<String, u8>,
    last_report: u64,
    last_status: u8,
    last_check: u64,
    pub avg_health: u8,  // For default health calculation
    pub alert: bool,
    pub severity: u8,
    pub health: u8,
}

impl NodeHealth {
    pub fn new() -> Self {
        Self {
            id: 0,
            kids: HashMap::new(),
            last_report: 0,
            last_status: 0,
            last_check: 0,
            avg_health: 0,
            alert: false,
            severity: 0,
            health: 0,
        }
    }

    pub fn from_node(&mut self, node: &NodeProto) {
        self.id = node.id;
        self.last_report = node.health_last_report;
        self.last_check = node.health_last_check;
        self.last_status = node.health_status;
        self.alert = false;
        self.severity = 0;
        self.health = 0;
        self.kids.clear();
        self.avg_health = 255;
        let kids_count = node.children.len() as u32;
        let mut kids_total = 0;
        for kid in node.children.iter() {
            let node = kid.upgrade().unwrap();
            let node = node.read().unwrap();
            self.kids.insert(node.name.clone(), node.health_status);
            kids_total += node.health_status as u32;
        }
        if kids_count > 0 { self.avg_health = (kids_total / kids_count) as u8; }
        self.health = self.avg_health;
    }

    pub fn apply(&self, node: &Arc<Node>) {
        unimplemented!()
    }

    pub fn get_kid_health(&mut self, kid_name: String) -> i64 {
        if let Some(health) = self.kids.get(&kid_name) {
            return health.clone() as i64
        }
        0
    }

    pub fn get_last_status(&mut self) -> i64 {
        self.last_status as i64
    }

    pub fn get_last_check(&mut self) -> i64 {
        self.last_check as i64
    }

    pub fn get_last_report(&mut self) -> i64 {
        self.last_report as i64
    }

    pub fn set_alert(&mut self, alert: bool) {
        self.alert = alert;
    }

    pub fn set_severity(&mut self, severity: i64) {
        self.severity = severity as u8;
    }

    pub fn set_health(&mut self, health: i64) {
        // info!("Setting health as {}", health);
        self.health = health as u8;
    }

    pub fn dump(&mut self) -> u32 {
        self.health as u32 + ((self.severity as u32) << 8) + ((self.alert as u32) << 16)
    }

}

pub struct EvalEngineProto {
    engine: Engine,
    pub node: NodeHealth,
}

impl EvalEngineProto {
    pub fn new() -> Arc<EvalEngine> {
        let engine = EvalEngineProto::new_engine();
        Arc::new(RwLock::new(engine))
    }

    pub fn new_engine() -> Self {
        let mut engine = Engine::new();
        engine.register_type::<NodeHealth>();
        engine.register_fn("new_node", NodeHealth::new);
        engine.register_fn("kid", NodeHealth::get_kid_health);
        engine.register_fn("dump", NodeHealth::dump);
        engine.register_get("last_check", NodeHealth::get_last_check);
        engine.register_get("last_report", NodeHealth::get_last_report);
        engine.register_get("last_status", NodeHealth::get_last_status);
        engine.register_set("alert", NodeHealth::set_alert);
        engine.register_set("severity", NodeHealth::set_severity);
        engine.register_set("health", NodeHealth::set_health);

        // Debugging
        engine.register_fn("print", showit as fn(x: &mut i32) -> ());
        engine.register_fn("print", showit as fn(x: &mut i64) -> ());
        engine.register_fn("print", showit as fn(x: &mut u32) -> ());
        engine.register_fn("print", showit as fn(x: &mut u64) -> ());
        engine.register_fn("print", showit as fn(x: &mut f32) -> ());
        engine.register_fn("print", showit as fn(x: &mut f64) -> ());
        engine.register_fn("print", showit as fn(x: &mut bool) -> ());
        engine.register_fn("print", showit as fn(x: &mut String) -> ());

        let node = NodeHealth::new();
        EvalEngineProto { engine, node }
    }

    pub fn eval_health(&self, node: &mut NodeHealth) {
        self.engine.call_fn::<&str, (&mut NodeHealth, ), ()>(&format!("fun{}", node.id), (node,)).unwrap();
    }

    pub fn eval(&mut self) {
        let ret = self.engine.call_fn::<&str, (&mut NodeHealth, ), u32>(&format!("fun{}", &self.node.id), (&mut self.node,)).unwrap();
        self.node.alert = (ret & 0x10000) > 0;
        self.node.severity = ((ret >> 8) & 0xff) as u8; 
        self.node.health = (ret & 0xff) as u8;
        info!("{:?}", self.node);
    }

    pub fn add_script(&mut self, raw_script: &str, node_id: u64) -> bool {
        let full_script = format!("fn fun{}(node) {{ {} \n return node.dump(); }}", node_id, raw_script);

        info!("Registering health script: {}", full_script);
        let add_script = self.engine.eval::<()>(&full_script);
        if !add_script.is_ok() {
            error!("Failed to register health script!");
            return false;
        }
        // Test the script
        let mut node = NodeHealth::new();
        match self.engine.call_fn::<&str, (&mut NodeHealth, ), u32>(&format!("fun{}", node_id), (&mut node,)) {
            Ok(_) => true,
            Err(e) => {
                error!("{:?}", e);
                // Override the script if it's invalid
                self.engine.eval::<()>(&format!("fn fun{}(node) {{ return node.dump(); }}", node_id)).unwrap();
                false
            }
        }
    }
}

pub type EvalEngine = RwLock<EvalEngineProto>;



