use std::sync::{Arc, Weak, RwLock, Mutex};

use serde_json::Value;
use crate::utils::*;
use crate::utils::{self, AsyncRes};
use std::collections::{HashMap, HashSet};
use std::iter::FromIterator;
use tokio::timer::delay;
use std::time::{Instant, Duration};

// use log::{warn, info};

pub enum NodeType {
    Application,
    Node,
    Leaf,
}

pub enum HealthCheckType {
    Timer,
    Event,
}

pub type Node = RwLock<NodeProto>;
pub type NodeStore = HashMap<u64, Arc<Node>>;
pub type NodeQ = Vec<Weak<Node>>;
pub type NodeIndexStore = HashMap<String, u64>;
pub type NodePathLocker = Mutex<NodePathLockerProto>;

pub struct NodePathLockerProto {
    locks: HashSet<String>,
}

impl NodePathLockerProto {
    pub fn new() -> Arc<NodePathLocker> {
        Arc::new(Mutex::new(NodePathLockerProto {
            locks: HashSet::new(),
        }))
    }

    #[allow(dead_code)]
    pub fn lock_path(&mut self, path: &String, locked: &mut bool) {
        *locked = self.locks.insert(path.clone()); 
    }

    pub fn lock_paths(&mut self, paths: &HashSet<String>, locked: &mut bool) {
        let mut locked_paths: HashSet<&String>= HashSet::new();
        let mut success = true;
        for path in paths.iter() {
            if self.locks.insert(path.clone()) {
                locked_paths.insert(path);
            } else {
                success = false;
                break;
            }
        }
        *locked = success;
        if !success {
            for path in locked_paths.iter() {
                let _ = self.locks.remove(path.clone());
            }
        }
    }
    
    pub fn unlock_paths(&mut self, paths: &HashSet<String>) {
        for path in paths.iter() {
            let _ = self.locks.remove(path);
        }
    }
}

pub struct NodeHodor {
    paths: HashSet<String>,
    locker: Arc<NodePathLocker>,
}

impl NodeHodor {
    pub fn new(paths: &Vec<String>, locker: Arc<NodePathLocker>) -> NodeHodor {
        let paths:  HashSet<String> = HashSet::from_iter(paths.iter().cloned());
        NodeHodor {
            paths,
            locker,
        }
    }

    pub async fn try_lock(&self, locked: &mut bool) -> AsyncRes {
        let mut locker = self.locker.lock().unwrap();
        locker.lock_paths(&self.paths, locked);
        Ok(())
    } 

    pub async fn try_get_locked(&self, locked: &mut bool, sleep_ms: u64) -> AsyncRes {
        self.try_lock(locked).await?;
        if !*locked {
            delay(Instant::now() + Duration::from_millis(sleep_ms as u64)).await;
        }
        self.try_lock(locked).await?;
        Ok(())
    }

    pub async fn unlock(&self) -> AsyncRes {
        let mut locker = self.locker.lock().unwrap();
        locker.unlock_paths(&self.paths);
        Ok(())
    }
}

pub struct StoreProto {
    id: u64,
    store: NodeStore,
    index: NodeIndexStore,
}

impl StoreProto {
    pub fn new() -> Arc<Store> {
        Arc::new(RwLock::new(StoreProto {
            id: 0,
            store: HashMap::new(),
            index: HashMap::new(),
        }))
    }
}

pub type Store = RwLock<StoreProto>;

pub struct InitNodeQ {
    pub app_meta: AppMeta,
    pub nodes: NodeQ,
}

#[derive(Clone)]
pub struct AppMeta {
    pub path: NodePath,
}

impl AppMeta {
    pub fn new() -> AppMeta {
        AppMeta {
            path: NodePath::new_path(),
        }
    }

    pub fn parse_app_name(path: &String) -> Option<String> {
        if !path.starts_with('.') { return None }
        let tokens: Vec<&str> = path.split('.').collect();
        if tokens.len() > 1 && tokens[1].len() > 0 {
            return Some(tokens[1].to_string());
        }
        None
    }
}

#[derive(Clone)]
pub struct NodePath {
    path: String,
    depth: usize,
}

impl NodePath {
    pub fn new_path() -> NodePath {
        NodePath {
            path: String::new(),
            depth: 0,
        }
    }

    pub fn append(&mut self, name: &String) {
        self.path.push_str(&(".".to_owned() + name));
        self.depth += 1;
    }

    pub fn new(root: String) -> NodePath {
        NodePath {
            path: root,
            depth: 1,
        }
    }

    pub fn read(&self) -> String {
        self.path.clone()
    }

    pub fn read_depth(&self) -> usize {
        self.depth
    }
}

pub type AppMetaMap = HashMap<String, AppMeta>;

pub struct NodeProto {
    pub id: u64,
    pub node_type: NodeType,
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub node_created: u64,

    pub metric_enabled: bool,

    pub parents: Vec<Weak<Node>>,
    pub children: Vec<Weak<Node>>,
    
    pub alert_enabled: bool,
    pub alert_description: String,
    pub alert_severity_eval: Option<String>,

    pub health_status: u8,
    pub health_check_eval: Option<String>,
    pub health_check_type: HealthCheckType,
    pub health_event_enabled: bool,
    health_check_init: bool,
    health_check_source: Weak<Node>,

    pub health_check_tick: u64,
    pub health_last_check: u64,
    pub health_last_report: u64,
    pub health_last_change: u64,

    pub app_meta_map: AppMetaMap,
}

impl NodeProto {
    pub fn add_parent(&mut self, node: Weak<Node>) {
        self.parents.push(node);
    }

    pub fn add_child(&mut self, node: Weak<Node>) {
        self.children.push(node);
    }

    pub fn calculate_health(&mut self) {
        // default script to check health status of the node
        let mut count: u32 = 0;
        let mut amount: u32 = 0;
        for node in self.children.iter() {
            if let Some(child_node) = node.upgrade() {
                let child = child_node.read().unwrap();
                count += 1;
                amount += child.health_status as u32;
            }
        }
        self.health_status = 255;
        if count > 0 { self.health_status = (amount / count) as u8; }
        self.health_last_check = utils::now();
    }

    pub fn tick(&mut self, tick: u64, app_name: &String) {
        if self.health_check_tick > tick {
            panic!("Unexpected check tick of node");
        } else if self.health_check_tick < tick {
            self.health_check_tick = tick;
            self.calculate_health();
            let app_meta = self.get_app_meta(app_name).unwrap();
            info!("App:{} node {} status evaluated as {}", app_name, app_meta.path.read(), self.health_status);
        }
    }

    pub fn get_app_meta(&mut self, app_name: &String) -> Option<&AppMeta> {
        self.app_meta_map.get(app_name)
    }
}

pub trait StoreOps {
    fn new_node(&self) -> Arc<Node>;
    fn add_node(&self, raw: &Value, name: String) -> Arc<Node>;
    fn add_app_node(&self, raw: &Value) -> Arc<Node>;
    fn add_leaf_node(&self, name: &String, raw: &Value) -> Arc<Node>;
    fn update_index(&self, name: &String, index: u64);
    fn get_weak_node(&self, path: &String) -> Option<Weak<Node>>;
}

impl StoreOps for Arc<Store> {
    fn new_node(&self) -> Arc<Node> {
        let mut node = NodeProto {
            id: 0,
            node_type: NodeType::Node,
            name: String::new(),
            display_name: String::new(),
            description: String::new(),
            node_created: utils::now(),
            metric_enabled: true,

            parents: Vec::new(),
            children: Vec::new(),

            alert_enabled: true,
            alert_description: String::new(),
            alert_severity_eval: None,
            health_status: 0,
            health_check_eval: None,
            health_check_type: HealthCheckType::Timer,
            health_event_enabled: true,
            health_check_init: true,
            health_check_source: Weak::new(),

            health_check_tick: 0,
            health_last_check: 0,
            health_last_report: 0,
            health_last_change: 0,
            app_meta_map: HashMap::new(),
        };
        let mut store = self.write().unwrap();
        let id = store.id;
        node.id = id;
        store.id += 1;
        let new_node = Arc::new(RwLock::new(node));
        store.store.insert(id, new_node.clone());
        new_node
    }
    fn add_node(&self, raw: &Value, name: String) -> Arc<Node> {
        let node = self.new_node();
        {
            let mut state = node.write().unwrap();
            state.name = name;
            state.display_name = raw.get_str("display_name", "new node");
            state.description = raw.get_str("description", "");
            state.alert_enabled = raw.get_bool("alert_enabled", true);
            state.alert_description = raw.get_str("alert_description", "");
            state.health_event_enabled = raw.get_bool("health_event_enabled", true);
            
            state.metric_enabled = raw.get_bool("metric_enabled", true);
            state.node_type = NodeType::Node;
        }
        node
    }

    fn add_leaf_node(&self, name: &String, raw: &Value) -> Arc<Node> {
        let node = self.add_node(raw, name.clone());
        {
            let mut state = node.write().unwrap();
            state.node_type = NodeType::Leaf;
        }
        node
    }
    fn add_app_node(&self, raw: &serde_json::Value) -> Arc<Node> {
        let name = raw.get_str("name", "new_application");
        let node = self.add_node(raw, name);
        {
            let mut state = node.write().unwrap();
            state.node_type = NodeType::Application;
        }
        node
    }

    fn update_index(&self, name: &String, index: u64) {
        let mut state = self.write().unwrap();
        state.index.insert(name.clone(), index);
    }

    fn get_weak_node(&self, path: &String) -> Option<Weak<Node>> {
        let state = self.read().unwrap();
        if let Some(id) = state.index.get(path) {
            if let Some(node) = state.store.get(&id) {
                return Some(Arc::downgrade(&node.clone()));
            }
        }
        None
    }
}

