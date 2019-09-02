
use std::sync::{Arc, RwLock};
use crate::application::*; 
use serde_json::Value;
use crate::node::*;
use crate::utils;
use crate::metric::*;
use crate::alert::*;
use crate::event::*;
use std::collections::HashMap;
use tokio::sync::mpsc;
use std::error::Error;
use std::time::{Instant, Duration};
use tokio::timer::Delay;

pub struct WatcherState {
    tick: u64,
    ticking: bool,
    tick_init: bool,

    last_tick_start: u128,
    last_tick_end: u128,
    
    interval: u64,
}

#[derive(Clone)]
pub struct WatcherDispatcher {
    metric_tx: mpsc::UnboundedSender<Metric>,
    event_tx: mpsc::UnboundedSender<Event>,
    alert_tx: mpsc::UnboundedSender<Alert>,
}

impl WatcherDispatcher {
    pub fn new() -> WatcherDispatcher {
        let (metric_tx, metric_rx) = mpsc::unbounded_channel();
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let (alert_tx, alert_rx) = mpsc::unbounded_channel();
        WatcherDispatcher {
            metric_tx,
            event_tx,
            alert_tx,
        }
    }

    pub fn send_alert(&self, alert: Alert) {
        let mut sender = self.alert_tx.clone();
        let _ = sender.try_send(alert);
    }

    pub fn send_event(&self, event: Event) {
        let mut sender = self.event_tx.clone();
        let _ = sender.try_send(event);
    }

    pub fn send_metric(&self, metric: Metric) {
        let mut sender = self.metric_tx.clone();
        let _ = sender.try_send(metric);
    }
}

#[derive(Clone)]
pub struct Watcher {
    app_map: Arc<RwLock<HashMap<String, Arc<Application>>>>,
    store: Arc<Store>,
    state: Arc<RwLock<WatcherState>>,
    dispatcher: WatcherDispatcher,
}

impl Watcher {
    pub fn new() -> Watcher {
        let state = WatcherState {
            tick: 0,
            ticking: false,
            tick_init: true,
            last_tick_start: 0,
            last_tick_end: 0,
            interval: 10,

        };
        Watcher {
            app_map: Arc::new(RwLock::new(HashMap::new())),
            state: Arc::new(RwLock::new(state)),
            store: StoreProto::new(),
            dispatcher: WatcherDispatcher::new(),
        }
    }

    pub fn add_application(&mut self, raw: &Value) {
        let app = ApplicationProto::new(self.store.clone());
        let mut state = app.write().unwrap();
        state.parse(&raw);
        let mut apps = self.app_map.write().unwrap();
        let app_name = state.read_name();
        apps.insert(app_name, app.clone());
    }

    pub async fn start(&mut self) -> Result<(), Box<dyn Error>> {
        let interval: u64;
        let mut last_tick: u64;
        {
            let state = self.state.read().unwrap();
            interval = state.interval * 1000;
        }
        loop {
            {
                let state = self.state.read().unwrap();
                last_tick = state.last_tick_start as u64;
            }
            let sleep_ms = (last_tick + interval) as i64 - utils::now() as i64 * 1000;
            if sleep_ms > 0 {
                Delay::new(Instant::now() + Duration::from_millis(sleep_ms as u64)).await;
            }
            self.tick();
        }
        Ok(())
    }

    pub fn tick(&mut self) {
        info!("Watcher starts to stare at white walkers");
        let tick: u64;
        {
            let mut state = self.state.write().unwrap();
            state.tick += 1;
            state.tick_init = false;
            tick = state.tick;
            state.last_tick_start = utils::now_ms();
            state.ticking = true;
        }
        let apps = self.app_map.read().unwrap();
        for app in apps.values() {
            let mut state = app.write().unwrap();
            state.tick(tick);
        }
        {
            let mut state = self.state.write().unwrap();
            state.last_tick_end = utils::now_ms();
            state.ticking = false;
        }
    }
}



