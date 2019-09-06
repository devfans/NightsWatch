use tokio::{codec, sync::mpsc, timer::delay, net::TcpStream, codec::{FramedRead, FramedWrite, Framed} };
use std::{io, marker, net::{SocketAddr, ToSocketAddrs}, time::{Duration, Instant}, marker::Unpin, sync::{Arc, Mutex}};
use crate::utils::AsyncRes;
use futures::{StreamExt, Stream, Sink, SinkExt};


pub trait Wine<MessageType> {
    type Stream: Stream<Item=MessageType> + Send + Unpin;
    type Framer: codec::Encoder<Item=MessageType, Error=io::Error> + codec::Decoder<Item=MessageType, Error=io::Error> + Send + Unpin;
    fn wake_up(&self) -> Self::Stream;
    fn take_nap(&self);
    fn drink(&self, message: MessageType);
    fn get_framer(&self) -> Self::Framer;
}

pub struct Knight<MessageType: 'static + Send, WineProvider: Wine<MessageType>> {
    target: String,
    wine: WineProvider,
    ph: marker::PhantomData<&'static MessageType>,
}

impl<MessageType: Send + Sync, WineProvider: 'static + Wine<MessageType> + Send + Sync> Knight<MessageType, WineProvider> {
    pub fn new(target: &String, wine: WineProvider) -> Knight<MessageType, WineProvider> {
        Knight {
            target: target.clone(),
            wine,
            ph: marker::PhantomData,
        }
    }

    pub async fn drink_wine(self) -> AsyncRes {
        let us = Arc::new(self);
        loop {
            let mut success = false;
            let mut target: Option<SocketAddr> = None;
            us.parse_target(&mut target).await?;
            if let Some(addr) = target {
                info!("Connecting to remote target: {}", addr);
                match TcpStream::connect(&addr).await {
                    Ok(stream) => {
                        let (r, w) = stream.split();
                        let mut messenger = us.wine.wake_up();
                        // let mut stream = Framed::new(stream, self.wine.get_framer());
                        let mut tx = FramedWrite::new(w, us.wine.get_framer());
                        let mut rx = FramedRead::new(r, us.wine.get_framer());
                        success = true;
                        // self.rx.forward(tx);
                        // tx.send_all(self.tx);
                        // self.rx.forward(stream);
                        info!("Connected to {}", addr);
                        let handler = us.clone();
                        tokio::spawn( async move {
                            loop {
                                match rx.next().await {
                                    Some(Ok(msg)) => { handler.wine.drink(msg); },
                                    _ => { 
                                        warn!("Potential disconnection from remote side");
                                        break;
                                    },
                                }
                            }
                        });
                        /* match tx.send_all(&mut updates).await {
                            _ => { warn!("Potential disconnection"); },
                        }
                        */
                        loop {
                            match messenger.next().await {
                                Some(msg) => {
                                    match tx.send(msg).await {
                                        Ok(_) => {
                                            // info!("Message sent");
                                        }
                                        Err(e) => {
                                            error!("Connection broken! error: {:?}", e);
                                            break;
                                        },
                                    }
                                },
                                _ => { panic!("Unexpected message null"); },
                            }
                        }
                        us.wine.take_nap();
                    },
                    Err(e) => {
                        error!("Failed to connecto to {}, error: {:?}", addr, e);
                    },
                }
            }

            if !success {
                warn!("Faild to parse target address: {}", us.target);
            } else {
                warn!("Disconnected from target address, will reconnect again after one second");
            }
            delay(Instant::now() + Duration::from_secs(1)).await;
        }
    }

    async fn parse_target(&self, addr: &mut Option<SocketAddr>) -> AsyncRes {
        match self.target.to_socket_addrs() {
            Ok(mut addrs) => {
                *addr = addrs.next();
            },
            _ => {}
        }
        Ok(())
    }
}
