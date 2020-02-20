/*  MIT License

Copyright (c) 2019 Stefan Liu - NightsWatch

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
*/

use serde_json::Value;
use crate::metric::*;
use crate::alert::*;
use crate::event::*;
use crate::landing::Landing;
use tokio::sync::mpsc;
use futures::StreamExt;
use simple_redis;
use std::sync::{Arc, Mutex};
use crate::maester::Maester;


pub const REDIS_KEY_METRICS:   &'static str = "NigthsWatchMetrics";
pub const REDIS_KEY_EVENTS:    &'static str = "NigthsWatchEvents";
pub const REDIS_KEY_ALERTS:    &'static str = "NigthsWatchAlerts";
pub const REDIS_KEY_SNAPSHOTS: &'static str = "NigthsWatchSnapshots";


pub type CommandError = simple_redis::types::RedisError;

#[derive(Clone)]
pub struct WatcherDispatcher {
    metric_tx: mpsc::UnboundedSender<Metric>,
    event_tx: mpsc::UnboundedSender<Event>,
    alert_tx: mpsc::UnboundedSender<Alert>,
    snapshot_tx: mpsc::UnboundedSender<String>,
    maester: Arc<Maester>,
    redis_client: Option<Arc<Mutex<simple_redis::client::Client>>>,
}

impl WatcherDispatcher {
    pub fn new(landing: &Landing, maester: &Arc<Maester>) -> WatcherDispatcher {
        let (metric_tx, mut metric_rx) = mpsc::unbounded_channel();
        let (event_tx, mut event_rx) = mpsc::unbounded_channel();
        let (alert_tx, mut alert_rx) = mpsc::unbounded_channel();
        let (snapshot_tx, mut snapshot_rx) = mpsc::unbounded_channel();
        let redis_publishing = !landing.redis_publish.is_none();

        let mut dispatcher = WatcherDispatcher {
            metric_tx,
            event_tx,
            alert_tx,
            snapshot_tx,
            maester: maester.clone(),
            redis_client: None,
        };

        if redis_publishing {
            dispatcher.redis_client = Some(Arc::new(Mutex::new(simple_redis::create(&landing.redis_publish.as_ref().unwrap().to_string()).unwrap())));
        }

        macro_rules! dispatch {
            ($publish: expr, $rx: expr, $chan: expr, $desc: expr, $type: ty, $handle: expr) => {
                {
                    let publish = $publish.clone();
                    tokio::spawn(async move {
                        if !redis_publishing {
                            loop {
                                match $rx.next().await {
                                    Some(ref msg) => { 
                                        info!("New {}: {:?}", $desc, msg);
                                        $handle(msg);
                                    },
                                    _ => unreachable!(),
                                }
                            }
                        } else {
                            let mut redis = simple_redis::create(&publish).unwrap();
                            loop {
                                match $rx.next().await {
                                    Some(ref msg) => { 
                                        let data: Value = (msg as &$type).into();
                                        let string = data.to_string();
                                        match redis.publish($chan, &string) {
                                            Ok(_) => { 
                                                info!("Published {}: {}", $desc, &string);
                                                // maester_handler.on_received::<$type>(msg);
                                                $handle(msg);
                                            },
                                            Err(e) => { error!("Failed to publish {} {} error: {:?}", $desc, &string, e); }
                                        }
                                    },
                                    _ => unreachable!(),
                                }
                            }
                        }
                    });
                }
            }
        }

        let redis_publish = match landing.redis_publish {
            Some(ref string) => string.clone(),
            None => String::new(),
        };
        
        let alert_handler = maester.clone();
        let event_handler = maester.clone();
        dispatch!(&redis_publish, metric_rx, REDIS_KEY_METRICS, "metric", Metric, |_| {});
        dispatch!(&redis_publish, event_rx, REDIS_KEY_EVENTS, "event", Event, |msg| event_handler.on_event(msg));
        dispatch!(&redis_publish, alert_rx, REDIS_KEY_ALERTS, "alert", Alert, |msg| alert_handler.on_alert(msg));

        tokio::spawn(async move {
            if !redis_publishing {
                loop {
                    match snapshot_rx.next().await {
                        Some(msg) => info!("New snapshot {}", msg),
                        _ => unreachable!(),
                    }
                }
            } else {
                let mut redis = simple_redis::create(&redis_publish).unwrap();
                loop {
                    match snapshot_rx.next().await {
                        Some(ref msg) => {
                            match redis.run_command_empty_response("LPUSH", vec![REDIS_KEY_SNAPSHOTS, msg]) {
                                Ok(_) => {
                                    info!("Saved snapshot into redis!");
                                    info!("{}", msg);
                                    // Trim for saving storage
                                    let _ = redis.run_command_empty_response("LTRIM", vec![REDIS_KEY_SNAPSHOTS, "0", "10"]);
                                },
                                Err(e) => { error!("Failed to save snapshot to redis for the moment error {:?} snapshot {}", e, msg); }
                            }
                        },
                        _ => unreachable!(),
                    }
                }
            }
        });

        dispatcher
    }

    pub fn send_alert(&self, alert: Alert) {
        let sender = self.alert_tx.clone();
        let _ = sender.send(alert);
    }

    pub fn send_event(&self, event: Event) {
        let sender = self.event_tx.clone();
        let _ = sender.send(event);
    }

    pub fn send_metric(&self, metric: Metric) {
        let sender = self.metric_tx.clone();
        let _ = sender.send(metric);
    }

    pub fn send_snapshot(&self, snapshot: &String) {
        let sender = self.snapshot_tx.clone();
        let _ = sender.send(snapshot.to_string());
    }

    pub fn command_get_str(&self, command: &str, args: Vec<&str>) -> Result<String, CommandError> {
        match self.redis_client {
            Some(ref client) => {
                client.lock().unwrap().run_command::<String>(command, args)
            },
            None => Err(CommandError { info: simple_redis::types::ErrorInfo::Description("No redis store available") })
        }
    }
}


