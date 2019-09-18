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

use std::sync::{Arc, Weak};
use crate::watcher::*;
use crate::utils::{self, AsyncRes};
use tokio::{
    self,
    codec::Framed,
    net::{TcpListener, TcpStream},
};
use std::net::SocketAddr;
use crate::node::*;
use crate::metric::Metric;
use std::collections::HashMap;
use crate::dracarys::{Dracarys, DracarysFramer};
use futures::{StreamExt};
use serde_json::{self, json};

use std::time::{Duration, Instant};
use tokio::timer::delay;

struct ColdHands {
    hands: HashMap<u16, Weak<Node>>,
    watcher: Weak<Watcher>,
}

impl ColdHands {
    pub fn new(watcher: Weak<Watcher>) -> ColdHands {
        ColdHands {
            hands: HashMap::new(),
            watcher: watcher,
        }
    }
    pub async fn process(&mut self, msg: Dracarys) -> AsyncRes {
        match msg {
            Dracarys::Target { id, ref paths, ref name, ref extra } => {
                let watcher = self.watcher.upgrade().unwrap();
                // Check if any parent exist first
                if watcher.locate_node_with_paths(paths).is_none() {
                    error!("No parent exists for this ranger: {:?}", msg);
                    return Ok(());
                }
                let mut locked = false;
                let mut failed_path: String = String::new();
                let mut lock_paths = Vec::new();
                for path in paths.iter() {
                    lock_paths.push(path.clone() + "." + name);
                }
                let mut leaf: Option<Weak<Node>> = None;
                // Try locate the node with paths first to void locking
                leaf = watcher.locate_node_with_paths(&lock_paths);
                if leaf.is_none() {
                    // Try to lock paths first before create a leaf node
                    let locker = watcher.new_locker(&lock_paths);
                    locker.try_lock(&mut locked, &mut failed_path).await?;
                    if locked {
                        // Try locate again
                        leaf = watcher.locate_node_with_paths(&lock_paths);
                        if leaf.is_none() {
                            // Create leaf node and link to parents
                            let raw = match serde_json::from_str(extra) {
                                Ok(raw) => raw,
                                Err(_) => json!({}),
                            };
                            leaf = watcher.allocate_ranger(name, paths, &raw);
                        }
                        locker.unlock().await?;
                    } else if !failed_path.is_empty() {
                        delay(Instant::now() + Duration::from_millis(200)).await;
                        leaf = watcher.locate_node(&failed_path);
                    }
                }
                if let Some(ranger) = leaf {
                    self.hands.insert(id, ranger);
                    info!("Successfully allocated/found the ranger for {:?}", msg);
                } else {
                    warn!("Failed to Find or allocate the ranger");
                }
            },
            Dracarys::Report { id, health_status } => {
                if let Some(node) = self.hands.get(&id) {
                    if let Some(state) = node.upgrade() {
                        let mut state = state.write().unwrap();
                        state.health_status = health_status;
                        state.health_last_report = utils::now();
                        info!("Successfully updated health status for ranger with id {}", id);
                    } else {
                        warn!("Failed to get the target node for report {:?}", msg);
                    }
                } else {
                    warn!("Ranger tell false tales: {:?}", msg);
                }
            },
            Dracarys::Message { id, ref data } => {
                info!("Message from id: {} ranger: {}", id, data);
            },
            Dracarys::Metric { ref metrics, .. } => {
                let watcher = self.watcher.upgrade().unwrap();
                for m in metrics.iter() {
                    watcher.dispatcher.send_metric((&m.0, &m.1, &m.2).into());
                }
                /*
                if let Some(node) = self.hands.get(&id) {
                    if let Some(state) = node.upgrade() {
                        let node = state.read().unwrap();
                        
                    }
                } else {
                    warn!("Ranger tell false tales: {:?}", msg);
                }
                */
            }
        }
        Ok(())
    }
}

pub struct Nightfort {
    watcher: Weak<Watcher>,
    listen_bind: String,
}

impl Nightfort {
    pub fn new(watcher: &Arc<Watcher>) -> Nightfort {
        let watcher = Arc::downgrade(watcher);
        Nightfort {
            watcher,
            listen_bind: "0.0.0.0:6000".to_string(),
        }
    }

    pub async fn setup(&mut self) -> AsyncRes {
        // read conf
        {
            let watcher = self.watcher.upgrade().unwrap();
            let landing = watcher.landing.read().unwrap();
            self.listen_bind = landing.nightfort_listen_bind.clone();
        }

        // listen and bind
        // let addr = self.listen_bind.to_socket_addrs().unwrap().next().unwrap();
        let addr: SocketAddr  = self.listen_bind.parse().unwrap();
        let mut listener = TcpListener::bind(&addr).await?;
        info!("Nightfort listening on {}", addr);
        loop {
            let (stream, addr) = listener.accept().await?;
            let watcher = self.watcher.clone();
            tokio::spawn(async move {
                if let Err(e) = Nightfort::process(watcher, stream, addr).await {
                    error!("Error on this ranger: {}, error: {:?}", addr, e);
                }
            });
        }
    }

    pub async fn process(watcher: Weak<Watcher>, stream: TcpStream, addr: SocketAddr) -> AsyncRes {
        let mut handler = ColdHands::new(watcher);
        let mut stream = Framed::new(stream, DracarysFramer::new());
        info!("New Ranger connected from: {}", addr);

        loop {
            match stream.next().await {
                Some(Ok(msg)) => {
                    // info!("Nightfort rx: {:?}", msg);
                    handler.process(msg).await?;
                },
                Some(Err(e)) => {
                    error!("Nightfor met error: {:?}", e);
                    return Ok(());
                },
                None => { 
                    warn!("We lost connection with this ranger from {}", addr);
                    return Ok(());
                }
            }
        }
    }
}
