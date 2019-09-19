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

use std::sync::{Arc, RwLock, Weak};
use crate::application::*; 
use serde_json::Value;
use crate::node::*;
use crate::utils;
use crate::metric::*;
use crate::alert::*;
use crate::event::*;
use crate::landing::Landing;
use std::collections::HashMap;
use tokio::sync::mpsc;
use std::error::Error;
use std::time::{Instant, Duration};
use tokio::timer::delay;
use crate::eval::*;
use futures::{StreamExt, Stream, Sink, SinkExt};
use simple_redis;



#[derive(Clone)]
pub struct WatcherDispatcher {
    metric_tx: mpsc::UnboundedSender<Metric>,
    event_tx: mpsc::UnboundedSender<Event>,
    alert_tx: mpsc::UnboundedSender<Alert>,
}

impl WatcherDispatcher {
    pub fn new(landing: &Landing) -> WatcherDispatcher {
        let (metric_tx, mut metric_rx) = mpsc::unbounded_channel();
        let (event_tx, mut event_rx) = mpsc::unbounded_channel();
        let (alert_tx, mut alert_rx) = mpsc::unbounded_channel();
        let redis_publishing = !landing.redis_publish.is_none();
        macro_rules! dispatch {
            ($publish: expr, $rx: expr, $chan: expr, $desc: expr, $type: ty) => {
                {
                    let publish = $publish.clone();
                    tokio::spawn(async move {
                        if !redis_publishing {
                            loop {
                                match $rx.next().await {
                                    Some(msg) => { info!("New {}: {:?}", $desc, msg);},
                                    _ => unreachable!(),
                                }
                            }
                        } else {
                            let mut redis = simple_redis::create(&publish).unwrap();
                            loop {
                                match $rx.next().await {
                                    Some(msg) => { 
                                        let data: Value = (msg as $type).into();
                                        let string = data.to_string();
                                        match redis.publish($chan, &string) {
                                            Ok(_) => { info!("Published {}: {}", $desc, &string); },
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
        
        dispatch!(&redis_publish, metric_rx, "NightsWatchMetric", "metric", Metric);
        dispatch!(&redis_publish, event_rx, "NightsWatchEvent", "event", Event);
        dispatch!(&redis_publish, alert_rx, "NightsWatchAlert", "alert", Alert);
                    
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


