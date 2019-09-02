use crate::utils;
use log::info;
use std::fmt::Debug;

#[derive(Debug)]
pub struct Metric {
    path: String,
    timestamp: u64,
    value: f64,
    data: String,
}

#[derive(Clone)]
pub struct MetricChannel {
}

impl MetricChannel {

    pub fn new() -> MetricChannel {
        MetricChannel {
        }
    }

    pub fn record<T: ToString>(&self, name: &String, value: T) {
        let m = Metric {
            path: name.clone(),
            timestamp: utils::now(),
            data: value.to_string(),
            value: 0.0,
        };
        info!("Metric: {:?}", m);
    }
}



