use std::sync::Weak;

pub struct Alert {
    name: String,
    application: String,
    source: Weak<Node>,
    path: String,
    timestamp: u64,
    severity: u8,
    description: String,
}
