
use std::sync::{Arc, RwLock};
use crate::application::*; 
use serde_json::Value;
use crate::node::*;
use crate::utils;

pub struct Watcher {
    applications: Arc<RwLock<Vec<Arc<Application>>>>,
    tick: u64,
    ticking: bool,
    init: bool,

    last_tick_start: u64,
    last_tick_end: u64,

    store: Arc<Store>,
}

impl Watcher {
    pub fn new() -> Watcher {
        Watcher {
            applications: Arc::new(RwLock::new(Vec::new())),
            tick: 0,
            ticking: false,
            init: true,
            last_tick_start: 0,
            last_tick_end: 0,
            store: StoreProto::new(),
        }
    }

    pub fn add_application(&mut self, raw: &Value) {
        let app = ApplicationProto::new(self.store.clone());
        let mut state = app.write().unwrap();
        state.parse(&raw);
        let mut apps = self.applications.write().unwrap();
        apps.push(app.clone());
    }

    pub fn start(&mut self) {
    }

    pub fn tick(&mut self) {
        self.tick += 1;
        self.last_tick_start = utils::now();
        let apps = self.applications.read().unwrap();
        for app in apps.iter() {
            let mut state = app.write().unwrap();
            state.tick(self.tick);
        }
    }
}



