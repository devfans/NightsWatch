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

use std::sync::{Weak, Arc, RwLock};
use serde_json::Value;
use serde_json::map::Map;
use crate::node::*;
use crate::utils::{self, *};
use log::{info};
use std::collections::{HashMap, VecDeque};
use crate::eval::*;
use crate::dispatcher::*;
use crate::alert::Alert;

pub type Application = RwLock<ApplicationProto>;

pub struct ApplicationProto {
    health_alert_threshold: u8,
    root: Weak<Node>,
    nodes_init: bool,  // Flag to re-draw the achitecture of the app
    nodes: Arc<RwLock<NodeQ>>,
    nodes_by_depth: Arc<RwLock<HashMap<usize, Vec<Weak<Node>>>>>,
    depth: usize,
    last_tick: u64,
    store: Arc<Store>,
    dispatcher: WatcherDispatcher
}


impl ApplicationProto {
    pub fn new(store: Arc<Store>, dispatcher: WatcherDispatcher) -> Arc<Application> {
        Arc::new(RwLock::new(ApplicationProto {
            health_alert_threshold: 10,
            root: Weak::new(),
            nodes_init: true,
            nodes: Arc::new(RwLock::new(Vec::new())),
            nodes_by_depth: Arc::new(RwLock::new(HashMap::new())),
            last_tick: 0,
            depth: 0,
            store,
            dispatcher,
        }))
    }

    pub fn sig_init(&mut self) {
        self.nodes_init = true;
    }

    pub fn tick(&mut self, tick: u64, eval: &mut EvalEngineProto) {
        if self.nodes_init {
            self.init_nodes();
            self.nodes_init = false;
        }

        self.run_tick(tick, eval);
    }

    pub fn read_name(&self) -> String {
        let root = self.root.clone();
        root.upgrade().unwrap().read().unwrap().name.clone()
    }

    fn run_tick(&mut self, tick: u64, eval: &mut EvalEngineProto) {
        let app_name = self.read_name();
        self.last_tick = tick;
        let nodes = self.nodes.read().unwrap();
        for item in nodes.iter().rev() {
            if let Some(node) = item.upgrade() {
                let mut node = node.write().unwrap();
                
                // Try update script if exists
                if let Some(ref script) = node.health_check_eval_override {
                    let success = eval.add_script(script, node.id);
                    if success {
                        info!("Successfully updated health check script!");
                        node.health_check_eval = Some(script.clone());
                        node.health_check_eval_change = utils::now();
                    } else {
                        error!("Failed to update health check script!");
                    }
                    node.health_check_eval_override = None;
                }

                if node.health_check_tick > tick {
                    panic!("Unexpected check tick of node");
                } else if node.health_check_tick < tick {
                    node.health_check_tick = tick;

                    eval.node.from_node(&node);
                    if !node.health_check_eval.is_none() {
                        info!("Evaluating health status for node {} with health script", node.id);
                        eval.eval();
                    } else {
                        if let NodeType::Leaf = node.node_type {
                            if utils::now() > node.health_last_report + node.health_report_threshold as u64 {
                                eval.node.alert = true;
                                eval.node.health = 0;
                            } else {
                                eval.node.health = node.health_status
                            }
                        }
                        if eval.node.health <= node.health_alert_threshold || eval.node.health <= self.health_alert_threshold {
                            eval.node.alert = true;
                        }
                    }
                    node.health_status = eval.node.health;
                    node.health_last_check = utils::now();
                    
                    let app_meta = node.get_app_meta(&app_name).unwrap().clone();
                    info!("App:{} node {} status evaluated as {}", app_name, app_meta.path.read(), node.health_status);

                    if node.alert_enabled && eval.node.alert {
                        self.dispatcher.send_alert(Alert {
                            name: format!("Health Alert for node: {}", node.display_name.clone()),
                            application: app_name.clone(),
                            source: item.clone(),
                            path: app_meta.path.read(),
                            timestamp: utils::now(),
                            severity: eval.node.severity,
                            description: node.alert_description.clone(),
                        });
                    }

                    if node.metric_enabled && (tick as u32) % node.metric_interval == 0 {
                        // self.dispatcher.send_metric(Metric::new(&app_meta.path.read(), node.health_status));
                        self.dispatcher.send_metric((&app_meta.path.read(), node.health_status).into());
                    }
                }

            }
        }
        /*
        let nodes_by_depth = self.nodes_by_depth.read().unwrap();
        for i in (1..self.depth + 1).rev() {
            let items = nodes_by_depth.get(&i).unwrap();
            for item in items {
                if let Some(node) = item.upgrade() {
                    let mut node = node.write().unwrap();
                    node.tick(tick, &app_name);
                }
            }
        }
        */

    }

    fn init_nodes(&mut self) {
        let nodes = self.nodes.clone();
        let nodes_by_depth = self.nodes_by_depth.clone();
        if let Some(node) = self.root.upgrade() {
            let mut nodes = nodes.write().unwrap();
            let mut nodes_by_depth = nodes_by_depth.write().unwrap();
            
            // Flush nodes queue first
            nodes.clear();
            nodes_by_depth.clear();
            let app_name: String;
            let app_display_name: String;
            let mut app_meta: AppMeta;
            {
                let mut app = node.write().unwrap();
                info!("Drawing tree architecture of application {}", app.display_name);
                app_display_name = app.display_name.clone();
                app_meta = AppMeta::new();
                app_name = app.name.clone();
                app_meta.path.append(&app.name);
                self.depth = app_meta.path.read_depth();
                app.app_meta_map.insert(app_name.clone(), app_meta.clone());
                self.store.update_index(&app_meta.path.read(), app.id);
                nodes.push(self.root.clone());
                let entry = nodes_by_depth.entry(self.depth).or_insert(Vec::new());
                entry.push(self.root.clone());
                
            }
            let mut tasks: VecDeque<InitNodeQ> = VecDeque::new();
            tasks.push_back(InitNodeQ {
                app_meta: app_meta,
                nodes: node.read().unwrap().children.clone(),
            });

            loop {
                let task = tasks.pop_front();
                if task.is_none() {
                    break;
                }
                let task = task.unwrap();

                for child in task.nodes.iter() {
                    if let Some(node) = child.upgrade() {
                        nodes.push(child.clone());
                        let mut kid_app_meta = task.app_meta.clone();
                        let mut kid = node.write().unwrap();
                        kid_app_meta.path.append(&kid.name);
                        self.depth = kid_app_meta.path.read_depth();
                        kid.app_meta_map.insert(app_name.clone(), kid_app_meta.clone());
                        self.store.update_index(&kid_app_meta.path.read(), kid.id);
                        let entry = nodes_by_depth.entry(self.depth).or_insert(Vec::new());
                        entry.push(child.clone());

                        tasks.push_back(InitNodeQ { 
                            app_meta: kid_app_meta.clone(),
                            nodes: kid.children.clone(),
                        });
                    }
                }
            }

            info!("Finished drawing tree architecture of application {}", app_display_name);
        }

    }

    // Sample application tree
    // app:
    //   children:
    //     node1:
    //       children:
    //          node3:
    //             children:
    //     node2: 
    //       children

    pub fn parse(&mut self, raw:& Value) {
        let root = self.store.add_app_node(&raw);
        if let Some(health_alert_threshold) = raw["health_alert_threshold"].as_u64() {
            self.health_alert_threshold = health_alert_threshold as u8;
        }
        self.root = Arc::downgrade(&root);
        if let Some(children) = raw["children"].as_object() {
            if !children.is_empty() {
                ApplicationProto::parse_children(&root, &children, &mut self.store);
            }
        }
    }
    
    pub fn parse_children(parent_node: &Arc<Node>, children: & JsonMap, store: &mut Arc<Store>) {
        for (name, raw) in children.iter() {
            let mut node = store.add_node(&raw, name.clone());
            if let Some(sub_children) = raw["children"].as_object() {
                if ! sub_children.is_empty() {
                    ApplicationProto::parse_children(&mut node, sub_children, store);
                }
            }
            let mut parent = parent_node.write().unwrap();
            parent.add_child(Arc::downgrade(&node));
            let mut child = node.write().unwrap();
            child.add_parent(Arc::downgrade(parent_node));
            info!("linking parent {} with child {}", parent.name, child.name);
        }
    }

    pub fn dump(&self, snapshot: &mut Snapshot) {
        snapshot.insert_app(&self);
        let nodes = self.nodes.read().unwrap();
        for node in nodes.iter() {
            snapshot.insert_node(node);
        }
    }

    pub fn serialize(&self) -> Value {
        let root = self.root.upgrade().unwrap().read().unwrap().id;
        json!({
            "health_alert_threshold": self.health_alert_threshold,
            "depth": self.depth,
            "root": root
        })
    }
    
    pub fn deserialize(
        raw: &Value,
        store: &Arc<Store>,
        dispatcher: &WatcherDispatcher,
        nodes: &Map<String, Value>,
        state: &mut HashMap<String, u64>,
        kids_map: &mut HashMap<String, Vec<String>>
        ) -> ApplicationProto {

        let mut tasks = VecDeque::new();
        let mut link_tasks = VecDeque::new();
        let mut app = Self {
            health_alert_threshold: raw.get_u64("health_alarm_threshold", 10) as u8,
            root: Weak::new(),
            nodes_init: true,
            nodes: Arc::new(RwLock::new(Vec::new())),
            nodes_by_depth: Arc::new(RwLock::new(HashMap::new())),
            last_tick: 0,
            depth: 0,
            store: store.clone(),
            dispatcher: dispatcher.clone(),
        };

        let root_id: String;
        if let Some(id) = raw["root"].as_u64() {
            root_id = id.to_string();
        } else {
            error!("Invalid root node id for application!");
            return app;
        }

        // Load all application nodes first
        tasks.push_back(root_id.clone());
        loop {
            let task = tasks.pop_front();
            if task.is_none() {
                break;
            }
            let node_id = task.unwrap();
            if state.get(&node_id).is_none() {
                let data = &raw[&node_id];
                let node = store.deserialize_node(data);
                let node = node.read().unwrap();
                state.insert(node_id.clone(), node.id);
                if let Some(children) = data["children"].as_array() {
                    for child in children.iter() {
                        if let Some(kid) = child.as_str() {
                            tasks.push_back(kid.to_string());
                            let entry = kids_map.entry(node_id.clone()).or_insert(Vec::new());
                            entry.push(kid.to_string());
                        }
                    }
                }
            }
        }
        tasks.clear();

        // Set application root node
        {
            let root_id = state.get(&root_id).unwrap();
            let node = store.get_node(root_id).unwrap();
            app.root = Arc::downgrade(&node);
            link_tasks.push_back(node);
        }

        // Try to link the nodes
        loop {
            let task = link_tasks.pop_front();
            if task.is_none() {
                break;
            }

            let parent_node = task.unwrap();
            let parent_id = parent_node.read().unwrap().id;
            if let Some(kids) = kids_map.get(&parent_id.to_string()) {
                for kid in kids.iter() {
                    // Cast to real child node id
                    if let Some(kid_id) = state.get(kid) {
                        // Get child node
                        if let Some(node) = store.get_node(kid_id) {
                            let mut parent = parent_node.write().unwrap();
                            parent.add_child(Arc::downgrade(&node));
                            let mut child = node.write().unwrap();
                            child.add_parent(Arc::downgrade(&parent_node));
                            info!("linking parent {} with child {}", parent.name, child.name);

                            // Append child to task queue
                            link_tasks.push_back(node.clone());
                        } else {
                            warn!("Node was not found during linking application, node id {}", kid_id);
                        }
                    } else {
                        warn!("Can not find node real id during linking application, node id {}", kid);
                    }
                }
            }
        }
        app
    }



}

/// Snapshot serves as node store for Serialize/Deserialize purpose
pub struct Snapshot {
    nodes: Map<String, Value>,
    applications: Map<String, Value>
}

impl Snapshot {
    pub fn new() -> Self {
        Self {
            nodes: Map::new(),
            applications: Map::new(),
        }
    }

    pub fn insert_node(&mut self, node: &Weak<Node>) {
        if let Some(node) = node.upgrade() {
            let node = node.read().unwrap();
            let id = node.id.to_string();
            if self.nodes.get(&id).is_none() {
                self.nodes.insert(id, node.serialize());
            }
        }
    }

    pub fn insert_app(&mut self, app: &ApplicationProto) {
        let name = app.read_name();
        if self.applications.get(&name).is_none() {
            self.applications.insert(name, app.serialize());
            self.insert_node(&app.root);
        }
    }

    pub fn dump(&self) -> Value {
        json!({
            "nodes": self.nodes,
            "applications": self.applications,
            "date": utils::now()
        })
    }

    pub fn load(&mut self, data: &Value) {
        if let Some(apps) = data["applications"].as_object() {
            self.applications = apps.clone();
        }
        if let Some(nodes) = data["nodes"].as_object() {
            self.nodes = nodes.clone();
        }
        let tsp = match data["date"].as_str() {
            Some(date) => format!(" created at {}", date.to_string()),
            None => String::new(),
        };
        info!("Loading snapshot {}.", tsp);
    }

    pub fn deserialize(&self, store: &Arc<Store>, dispatcher: &WatcherDispatcher) -> Vec<ApplicationProto> {
        let mut state = HashMap::new();
        let mut kids_map = HashMap::new();
        let mut apps = Vec::new();
        for app_data in self.applications.values() {
            let app = ApplicationProto::deserialize(app_data, store, dispatcher, &self.nodes, &mut state, &mut kids_map);
            if let Some(_) = app.root.upgrade() {
                info!("Successfully load application {} from snapshot!", &app.read_name());
                apps.push(app);
            } else {
                error!("Invalid application detected during deserializing snapshot!");
            }
        }
        apps
    }
}

   
