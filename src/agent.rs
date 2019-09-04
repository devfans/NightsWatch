use serde_json;
#[macro_use] extern crate log;
use env_logger;

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
mod knight;
mod ranger;

use utils::*;

use ranger::Ranger;


#[tokio::main]
async fn main() -> AsyncRes {
    env_logger::init();
    let mut conf_path = None;

    for arg in env::args().skip(1) {
        if arg.starts_with("-c") {
            conf_path = Some(arg.split_at(3).1.to_string());
        }
    }

    let conf = if let Some(path) = conf_path {
        path
    } else {
        "./config.json".to_string()
    };
    info!("Loading configuration from {}", conf);

    let conf_file = fs::File::open(conf).expect("Failed to read config file");
    let map = serde_json::from_reader(conf_file).expect("Failed to parse config file");
    let ranger = Ranger::new(&map);
    ranger.start().await?;
    warn!("This ranger is being destroyed!!!");
    Ok(())
}
