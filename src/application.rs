use std::sync::{Weak, Arc, RwLock};
use serde_json::Value;
use crate::node::*;
use crate::utils::*;

pub type Application = RwLock<ApplicationProto>;

pub struct ApplicationProto {
    notification_level: u8,
    root: Weak<Node>,
    nodes_init: bool,
    nodes: NodeQ,
    last_tick: u64,
}

impl ApplicationProto {
    pub fn new() -> Arc<Application> {
        Arc::new(RwLock::new(ApplicationProto {
            notification_level: 1,
            root: Weak::new(),
            nodes_init: true,
            nodes: Vec::new(),
            last_tick: 0,
        }))
    }

    pub fn tick(&mut self, tick: u64) {
        self.last_tick = tick;
        if self.nodes_init {
            self.init_nodes();
            self.nodes_init = false;
        }
    }

    pub fn init_nodes(&mut self) {}


    // Sample application tree
    // app:
    //   children:
    //     node1:
    //       children:
    //          node3:
    //             children:
    //     node2: 
    //       children

    pub fn parse(&mut self, raw:& Value, mut store: Arc<Store>) {
        let root = store.add_app_node(&raw);
        if let Some(notification_level) = raw["notification_level"].as_u64() {
            self.notification_level = notification_level as u8;
        }
        self.root = Arc::downgrade(&root);
        if let Some(children) = raw["children"].as_object() {
            if !children.is_empty() {
                ApplicationProto::parse_children(&root, &children, &mut store);
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
            println!("linking parent {} with child {}", parent.name, child.name);
        }
    }

}

    
