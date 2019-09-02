#![feature(async_await)]
extern crate serde_json;
#[macro_use] extern crate log;
extern crate env_logger;

use std::env;
use std::fs;
use std::error::Error;

mod application;

mod watcher;
mod node;
mod metric;
mod event;
mod utils;
mod alert;
use watcher::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
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
    watcher.start().await;
    Ok(())
}
