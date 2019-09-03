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
use std::collections::HashMap;
use crate::dracarys::{Dracarys, DracarysFramer};
use futures::{StreamExt};
use serde_json::{self, json};

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
                // Try to lock paths first before create a leaf node
                let watcher = self.watcher.upgrade().unwrap();
                let locker = watcher.new_locker(paths);
                let mut locked = false;
                locker.try_get_locked(&mut locked, 1000).await?;
                if locked {
                    // Create leaf node and link to parents
                    let raw = match serde_json::from_str(extra) {
                        Ok(raw) => raw,
                        Err(_) => json!({}),
                    };
                    watcher.allocate_ranger(name, paths, &raw);
                    locker.unlock().await?;
                } else {
                    warn!("Failed to lock requested node paths: {:?}", msg);
                }
            },
            Dracarys::Report { id, health_status } => {
                if let Some(node) = self.hands.get(&id) {
                    if let Some(state) = node.upgrade() {
                        let mut state = state.write().unwrap();
                        state.health_status = health_status;
                        state.health_last_check = utils::now();
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
                    error!("Error on this raven: {}, error: {:?}", addr, e);
                }
            });
        }
    }

    pub async fn process(watcher: Weak<Watcher>, stream: TcpStream, _addr: SocketAddr) -> AsyncRes {
        // > Find parents
        // > Create leaf node
        // > Poll
        let mut handler = ColdHands::new(watcher);
        let mut stream = Framed::new(stream, DracarysFramer::new());

        loop {
            match stream.next().await {
                Some(Ok(msg)) => {
                    info!("Nightfort rx: {:?}", msg);
                    handler.process(msg).await?;
                },
                _ => {
                    return Ok(());
                }
            }
        }
    }
}
