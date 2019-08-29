use std::sync::{Weak, Arc};
use serde_json;
use crate::node;

pub struct Application {
    notification_level: u8,
    root: Weak<node::Node>,
    nodes_init: bool,
    nodes: Vec<Weak<node::Node>>,
}

impl Application {
    pub fn new() -> Application {
        Application {
            notification_level: 1,
            root: Weak::new(),
            nodes_init: true,
            nodes: Vec::new(),
        }
    }

    pub fn parse(&mut self, raw: serde_json::Value, store: &Vec<Arc<node::Node>>) {
    }
}

