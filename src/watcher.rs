
use std::sync::Arc;
use crate::application::*; 
use serde_json::Value;
use crate::node;
use crate::utils;

pub struct Watcher {
    applications: Vec<Application>,
    tick: u64,
    ticking: bool,
    init: bool,

    last_tick: u64,

    nodes: Vec<Arc<node::Node>>,
}

impl Watcher {
    pub fn new() -> Watcher {
        Watcher {
            applications: Vec::new(),
            tick: 0,
            ticking: false,
            init: true,
            last_tick: 0,
            nodes: Vec::new(),
        }
    }

    pub fn add_application(&mut self, raw: Value) {
        let mut app = Application::new();
        app.parse(raw, &self.nodes);
    }

    pub fn start(&mut self) {
    }
}



