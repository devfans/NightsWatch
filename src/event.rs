use std::sync::Weak;

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
