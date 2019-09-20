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

use std::sync::{Arc, Weak, RwLock};
use crate::watcher::Watcher;
use ws::{listen, Handler, Sender, Result, Message, CloseCode, Error, Handshake};
use std::collections::HashMap;
use std::fmt;
use serde_json::{self, Value};

use crate::event::Event;
use crate::alert::Alert;
use crate::raven::RavenMessage;

use crate::utils::{self, AsyncRes};

struct MaesterSession<'a> {
    id: u64,
    out: Sender,
    watcher: &'a Arc<Watcher>,
    maester: &'a Maester,
}

impl<'a> Handler for MaesterSession<'a> {
    fn on_open(&mut self, _: Handshake) -> Result<()> {
        info!("New maester client just got connected!");
        self.maester.on_connected(&self.out);
        Ok(())
    }

    fn on_message(&mut self, msg: Message) -> Result<()> {
        // Echo the message back
        info!("Got data from maester client: {:?}", msg);
        // self.out.send(msg)
        match msg {
            Message::Text(ref data) => {
                let msg: RavenMessage = data.into();
                match msg {
                    RavenMessage::TakeSnapshot => {
                        self.watcher.take_snapshot();
                    },
                    RavenMessage::LoadSnapshot => {
                        self.watcher.load_snapshot_from_dispatcher();
                    },
                    _ => {}
                }
            },
            _ => { warn!("Received unrecognized bytes from maester client: {:?}", &msg); },
        }
        Ok(())
    }

    fn on_close(&mut self, code: CloseCode, reason: &str) {
        self.maester.on_disconnected(&self.id);
        match code {
            CloseCode::Normal => warn!("The maester client is done with the connection."),
            CloseCode::Away   => warn!("The maester client is leaving the site."),
            CloseCode::Abnormal => warn!(
                "Closing handshake failed! Unable to obtain closing status from the maester client."),
            _ => warn!("The maester client encountered an error: {}", reason),
        }
    }

    fn on_error(&mut self, err: Error) {
        error!("The maester server encountered an error: {:?}", err);
    }
}

struct MaesterState {
    watcher: Option<Arc<Watcher>>,
    sessions: HashMap<u64, Sender>,
    id: u64,  // Session unique id, should start with 1, we would assume 0 as invalid
}

impl MaesterState {
    pub fn register_session(&mut self, sender: &Sender) -> u64 {
        let id = self.id;
        self.sessions.insert(id, sender.clone());
        self.id += 1;
        id
    }

    pub fn deregister_session(&mut self, id: &u64) {
        if *id > 0 {
            self.sessions.remove(id);
        }
    }
}

pub struct Maester {
    state: Arc<RwLock<MaesterState>>,
}

impl Maester {
    pub fn new() -> Arc<Self> {
        let maester = Self {
            state: Arc::new(RwLock::new(MaesterState {
                watcher: None,
                sessions: HashMap::new(),
                id: 1
            })),
        };

        Arc::new(maester)
    }

    pub fn add_watcher(&self, watcher: &Arc<Watcher>) {
        let mut state = self.state.write().unwrap();
        state.watcher = Some(watcher.clone());
    }

    // Receive messages from watcher to dispatch to maester
    pub fn on_received<T: fmt::Debug>(&self, data: &T) {
        info!("To dispatch metric to maester: {:?}", data);
    }

    pub fn on_alert(&self, alert: &Alert) {
        self.broadcast(&RavenMessage::NewAlert { data: alert }.to_json());
    }

    pub fn on_event(&self, event: &Event) {
        self.broadcast(&RavenMessage::NewEvent { data: event }.to_json());
    }

    fn broadcast(&self, data: &str) {
        let state = self.state.read().unwrap();
        for sender in state.sessions.values() {
            let _ = sender.send(data);
        }
    }

    pub fn on_connected(&self, sender: &Sender) -> u64 {
        let mut state = self.state.write().unwrap();
        state.register_session(sender)
    }

    pub fn on_disconnected(&self, id: &u64) {
        let mut state = self.state.write().unwrap();
        state.deregister_session(id);
    }

    pub async fn setup(&self) -> AsyncRes {
        // We fetch the watcher first to share within the thread.
        let watcher = {
            let state = self.state.read().unwrap();
            state.watcher.as_ref().unwrap().clone()
        };

        let maester_listen_bind = {
            let landing = watcher.landing.read().unwrap();
            landing.maester_listen_bind.clone()
        };

        info!("Start to listen for maester clients at: {}", &maester_listen_bind);
        listen(&maester_listen_bind, |out| {
            MaesterSession { out, watcher: &watcher, id: 0, maester: self }
        }).unwrap();
        Ok(())
    }
}


