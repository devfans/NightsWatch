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
use crate::node::*;
use crate::utils::*;
use log::{info};
use std::collections::{HashMap, VecDeque};

pub type Application = RwLock<ApplicationProto>;

pub struct ApplicationProto {
    notification_level: u8,
    root: Weak<Node>,
    nodes_init: bool,  // Flag to re-draw the achitecture of the app
    nodes: Arc<RwLock<NodeQ>>,
    nodes_by_depth: Arc<RwLock<HashMap<usize, Vec<Weak<Node>>>>>,
    depth: usize,
    last_tick: u64,
    store: Arc<Store>,
}


impl ApplicationProto {
    pub fn new(store: Arc<Store>) -> Arc<Application> {
        Arc::new(RwLock::new(ApplicationProto {
            notification_level: 1,
            root: Weak::new(),
            nodes_init: true,
            nodes: Arc::new(RwLock::new(Vec::new())),
            nodes_by_depth: Arc::new(RwLock::new(HashMap::new())),
            last_tick: 0,
            depth: 0,
            store,
        }))
    }

    pub fn sig_init(&mut self) {
        self.nodes_init = true;
    }

    pub fn tick(&mut self, tick: u64) {
        if self.nodes_init {
            self.init_nodes();
            self.nodes_init = false;
        }

        self.run_tick(tick);
    }

    pub fn read_name(&self) -> String {
        let root = self.root.clone();
        root.upgrade().unwrap().read().unwrap().name.clone()
    }

    fn run_tick(&mut self, tick: u64) {
        let app_name = self.read_name();
        self.last_tick = tick;
        let nodes = self.nodes.read().unwrap();
        for item in nodes.iter().rev() {
            if let Some(node) = item.upgrade() {
                let mut node = node.write().unwrap();
                node.tick(tick, &app_name);
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
        if let Some(notification_level) = raw["notification_level"].as_u64() {
            self.notification_level = notification_level as u8;
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

}

    
