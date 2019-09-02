use std::sync::Weak;
use crate::node::*;

pub enum EventType {
    HealthDowngrade,
    HealthUpgrade,
    NodeJoin,
    NodeLeft,
}

pub struct Event {
    event_type: EventType,
    desc: String,
    source: Weak<Node>,
    application: String,
    timestamp: u64,
    path: String,
}

