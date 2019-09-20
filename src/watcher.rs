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


use std::sync::{Arc, RwLock, Weak};
use crate::application::*; 
use serde_json::{map::Map, Value};
use crate::node::*;
use crate::utils;
use crate::landing::Landing;
use std::collections::HashMap;
use std::error::Error;
use std::time::{Instant, Duration};
use tokio::timer::delay;
use crate::eval::*;
use crate::dispatcher::*;
use futures::{StreamExt, Stream, Sink, SinkExt};

pub struct WatcherState {
    tick: u64,
    ticking: bool,
    tick_init: bool,

    last_tick_start: u128,
    last_tick_end: u128,
    
    interval: u64,
}

#[derive(Clone)]
pub struct Watcher {
    app_map: Arc<RwLock<HashMap<String, Arc<Application>>>>,
    store: Arc<Store>,
    state: Arc<RwLock<WatcherState>>,
    pub dispatcher: WatcherDispatcher,
    pub locker: Arc<NodePathLocker>,
    pub landing: Arc<RwLock<Landing>>,
}

impl Watcher {
    pub fn new(landing: Landing) -> Watcher {
        let state = WatcherState {
            tick: 0,
            ticking: false,
            tick_init: true,
            last_tick_start: 0,
            last_tick_end: 0,
            interval: 10,

        };
        Watcher {
            app_map: Arc::new(RwLock::new(HashMap::new())),
            state: Arc::new(RwLock::new(state)),
            store: StoreProto::new(),
            dispatcher: WatcherDispatcher::new(&landing),
            locker: NodePathLockerProto::new(),
            landing: Arc::new(RwLock::new(landing)),
        }
    }
    
    pub fn new_locker(&self, paths: &Vec<String>) -> NodeHodor {
        NodeHodor::new(paths, self.locker.clone())
    }

    pub fn add_application(&mut self, raw: &Value) {
        let app = ApplicationProto::new(self.store.clone(), self.dispatcher.clone());
        let mut state = app.write().unwrap();
        state.parse(&raw);
        let mut apps = self.app_map.write().unwrap();
        let app_name = state.read_name();
        apps.insert(app_name, app.clone());
    }

    pub fn sig_app_init(&self, app: &String) {
        let apps = self.app_map.read().unwrap();
        if let Some(app) = apps.get(app) {
            let mut state = app.write().unwrap();
            state.sig_init();
        }
    }

    pub fn locate_node (&self, path: &String) -> Option<Weak<Node>> {
        self.store.get_weak_node(path)
    }

    pub fn locate_node_with_paths (&self, paths: &Vec<String>) -> Option<Weak<Node>> {
        for path in paths.iter() {
            let res = self.locate_node(path);
            if !res.is_none() { return res; }
        }
        None
    }

    pub fn allocate_ranger(&self, name: &String, paths: &Vec<String>, raw: &Value) -> Option<Weak<Node>> {
        // Create new leaf node
        // Link to parents
        // Activate application init_nodes
        let node = self.store.add_leaf_node(name, raw);
        let ranger = Arc::downgrade(&node);
        let mut leaf = node.write().unwrap();
        for path in paths.iter() {
            if let Some(app) = AppMeta::parse_app_name(path) {
                self.sig_app_init(&app);
                if let Some(parent) = self.store.get_weak_node(path) {
                    let parent_node = parent.upgrade().unwrap();
                    let mut state = parent_node.write().unwrap();
                    state.add_child(Arc::downgrade(&node));
                    leaf.add_parent(parent);
                    return Some(ranger);
                } else {
                    warn!("Failed to find parent: {}", path);
                }

            } else {
                warn!("Failed to parse app name from path: {}", path);
            }
        }
        None
    }

    pub async fn start(&mut self) -> Result<(), Box<dyn Error>> {
        let mut engine = EvalEngineProto::new_engine();
        let interval: u64;
        let mut last_tick: u64;
        {
            let state = self.state.read().unwrap();
            interval = state.interval * 1000;
        }
        loop {
            {
                let state = self.state.read().unwrap();
                last_tick = state.last_tick_start as u64;
            }
            let sleep_ms = (last_tick + interval) as i64 - utils::now() as i64 * 1000;
            if sleep_ms > 0 {
                delay(Instant::now() + Duration::from_millis(sleep_ms as u64)).await;
            }
            self.tick(&mut engine);
            // info!("{}", self.dump().to_string());
        }
    }

    pub fn dump(&self) -> Value {
        let mut snapshot = Snapshot::new();
        let apps = self.app_map.read().unwrap();
        for app in apps.values() {
            let app = app.read().unwrap();
            app.dump(&mut snapshot);
        }
        snapshot.dump()
    }

    pub fn load(&mut self, raw: &mut Value) {
        let mut snapshot = Snapshot::new();
        snapshot.load(raw);

        let mut apps = snapshot.deserialize(&self.store, &self.dispatcher);
        for app in apps.drain(..) {
            let mut apps = self.app_map.write().unwrap();
            let app_name = app.read_name();
            apps.insert(app_name, Arc::new(RwLock::new(app)));
        }
        info!("Loaded snapshot!");
    }

    pub fn load_snapshot_from_dispatcher(&mut self) {
        match self.dispatcher.command_get_str("LINDEX", vec![REDIS_KEY_SNAPSHOTS, "0"]) {
            Err(e) => {
                error!("Failed to load snapshot form redis: {:?}", e);
            },
            Ok(ref data) => {
                match serde_json::from_str::<Value>(data) {
                    Ok(ref mut raw) => { self.load(raw); },
                    Err(_) => { error!("Failed to fetch valid snapshot data, got {}", data); },
                }
            }
        }
    }

    pub fn take_snapshot(&self) {
        self.dispatcher.send_snapshot(&self.dump().to_string());
    }

    pub fn tick(&mut self, eval: &mut EvalEngineProto) {
        info!("Watcher starts to stare at white walkers");
        let tick: u64;
        {
            let mut state = self.state.write().unwrap();
            state.tick += 1;
            state.tick_init = false;
            tick = state.tick;
            state.last_tick_start = utils::now_ms();
            state.ticking = true;
        }
        let apps = self.app_map.read().unwrap();
        for app in apps.values() {
            let mut state = app.write().unwrap();
            state.tick(tick, eval);
        }
        {
            let mut state = self.state.write().unwrap();
            state.last_tick_end = utils::now_ms();
            state.ticking = false;
        }
    }
}



