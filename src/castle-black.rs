extern crate serde_json;
use std::fs;
use std::env;

mod application;

mod watcher;
mod node;
mod utils;
use watcher::*;


fn main() {
    let mut conf_path = None;

    for arg in env::args().skip(1) {
        if arg.starts_with("-c") {
            conf_path = Some(arg.split_at(14).1.to_string());
        }
    }

    let conf = if let Some(path) = conf_path {
        path
    } else {
        "./config.json".to_string()
    };
    println!("Loading configuration from {}", conf);

    let conf_file = fs::File::open(conf).expect("Failed to read config file");
    let config = serde_json::from_reader(conf_file).expect("Failed to parse config file");

    let mut watcher = Watcher::new();

    watcher.add_application(&config);
    watcher.start();
}
