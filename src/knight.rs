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

use tokio::{net::TcpStream };
use tokio_util::codec::{ self, FramedRead, FramedWrite };
use std::{io, marker, net::{SocketAddr, ToSocketAddrs}, marker::Unpin, sync::Arc };
use crate::utils::AsyncRes;
use futures::{Stream, join, SinkExt, StreamExt};


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
                    Ok(mut stream) => {
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
                        join!(async move {
                                loop {
                                    match rx.next().await {
                                        Some(Ok(msg)) => { handler.wine.drink(msg); },
                                        _ => {
                                            warn!("Potential disconnection from remote side");
                                            break;
                                        },
                                    }
                                }
                            },
                            async {
                                loop {
                                    match messenger.next().await {
                                        Some(msg) => {
                                            match tx.send(msg).await {
                                                Ok(_) => {
                                                    // info!("Message sent");
                                                }
                                                Err(e) => {
                                                    error!("Connection broken! error: {}", e);
                                                    break;
                                                },
                                            }
                                        },
                                        _ => { break; },
                                    }
                                }
                                us.wine.take_nap();
                            }
                        );
                    },
                    Err(e) => {
                        error!("Failed to connect to {}, error: {:?}", addr, e);
                    },
                }
            }

            if !success {
                warn!("Faild to parse target address: {}", us.target);
            } else {
                warn!("Disconnected from target address, will reconnect again after one second");
            }
            sleep!(1000);
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
