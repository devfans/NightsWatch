#![warn(rust_2018_idioms)]

extern crate serde;
extern crate serde_json;
#[macro_use] extern crate log;
extern crate env_logger;
extern crate bytes;
extern crate tokio_timer;
extern crate futures_core;
extern crate futures_sink;
extern crate futures_util;

use std::env;
use std::fs;

mod application;

mod landing;
mod watcher;
mod node;
mod metric;
mod event;
mod utils;
mod alert;
mod dracarys;
mod maester;
mod nightfort;

use utils::*;

use watcher::*;
use nightfort::Nightfort;
use log::{info, warn};
use std::sync::Arc;

#[tokio::main]
async fn main() -> AsyncRes {
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
    info!("Loading configuration from {}", conf);

    let conf_file = fs::File::open(conf).expect("Failed to read config file");
    let config = serde_json::from_reader(conf_file).expect("Failed to parse config file");

    let mut watcher = Watcher::new();
    watcher.add_application(&config);
    {
        let mut landing = watcher.landing.write().unwrap();
        landing.parse(&config);
    }


    let global_watcher = Arc::new(watcher.clone());
    let mut nightfort = Nightfort::new(&global_watcher);
    tokio::spawn( async move { let _ = nightfort.setup().await; } );

    watcher.start().await?;
    warn!("Watcher is being destroyed!!!");
    Ok(())
}
