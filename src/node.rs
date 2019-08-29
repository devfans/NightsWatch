use std::sync::{Arc, Weak, RwLock};
use serde_json;
use crate::utils;

pub enum NodeType {
    Application,
    Node,
    Leaf,
}

pub enum HealthCheckType {
    Timer,
    Event,
}

pub struct NodeProto {
    node_type: NodeType,
    name: String,
    display_name: String,
    description: String,
    node_created: u64,

    metric_enabled: bool,

    parents: Vec<Weak<RwLock<NodeProto>>>,
    children: Vec<Weak<RwLock<NodeProto>>>,
    
    alert_enabled: bool,
    alert_description: String,
    alert_severity_eval: Option<String>,

    health_status: u8,
    health_check_eval: Option<String>,
    health_check_type: HealthCheckType,
    health_event_enabled: bool,
    health_check_init: bool,
    health_check_source: Weak<RwLock<NodeProto>>,

    health_check_tick: u64,
    health_last_check: u64,
    health_last_report: u64,
    health_last_change: u64,
}

impl NodeProto {
    pub fn new() -> Arc<RwLock<NodeProto>> {
        Arc::new(RwLock::new(NodeProto {
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
        }))
    }
    pub fn parse_node(raw: serde_json::Value) -> Arc<RwLock<NodeProto>> {
        return NodeProto::new();
    }

    pub fn parse_app_node(raw: serde_json::Value) -> Arc<RwLock<NodeProto>> {
        return NodeProto::new();
    }
}

pub type Node = RwLock<NodeProto>;
