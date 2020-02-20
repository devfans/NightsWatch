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

use bytes;
use bytes::{Buf, BufMut};
use tokio_util::codec;
use std::io;

use crate::utils;

// Dracarys Structure
// FLAG(2) LEN(4) DATA(n)
// 0xe001  Target
// 0xe002  Report
// 0xe003  Message


#[derive(Debug)]
pub enum Dracarys {
    Target {
        id: u16,
        paths: Vec<String>,
        name: String,
        extra: String,
    },
    Report {
        id: u16,
        health_status: u8,
    },
    Message {
        id: u16,
        data: String,
    },
    Metric {
        id: u16,
        relative: bool,
        metrics: Vec<(String, String, u64)>,
    }
}

#[derive(Clone)]
pub struct DracarysFramer {
}

impl DracarysFramer {
    pub fn new() -> DracarysFramer {
        DracarysFramer {}
    }
}

impl codec::Encoder for DracarysFramer {
    type Item = Dracarys;
    type Error = io::Error;

    fn encode(&mut self, msg: Dracarys, res: &mut bytes::BytesMut) -> Result<(), io::Error> {
        info!("Sending message: {:?}", msg);
        match msg {
            Dracarys::Target { id, ref paths, ref name, ref extra} => {
                let path_count = paths.len();
                let mut path_total_len: usize = 0;
                for path in paths.iter() {
                    path_total_len += path.len();
                }
                let data_len = path_total_len + name.len() + extra.len();
                let total_len = 8 + data_len + 4 + 1 + path_count * 2;
                res.reserve(total_len);
                res.put_u16_le(0xe001);
                res.put_u32_le(total_len as u32);
                res.put_u16_le(id);
                res.put_u8(path_count as u8);
                for path in paths.iter() {
                    res.put_u16_le(path.len() as u16);
                    res.put_slice(path.as_bytes());
                }
                res.put_u16_le(name.len() as u16);
                res.put_slice(name.as_bytes());
                res.put_u16_le(extra.len() as u16);
                res.put_slice(extra.as_bytes());
            },
            Dracarys::Report { id, health_status } => {
                let total_len = 8 + 1;
                res.reserve(total_len);
                res.put_u16_le(0xe002);
                res.put_u32_le(total_len as u32);
                res.put_u16_le(id);
                res.put_u8(health_status);
            },
            Dracarys::Message { id, ref data } => {
                let total_len = 8 + data.len();
                res.reserve(total_len);
                res.put_u16_le(0xe003);
                res.put_u32_le(total_len as u32);
                res.put_u16_le(id);
                res.put_slice(data.as_bytes());
            },
            Dracarys::Metric { id, relative, ref metrics } => {
                let count = metrics.len();
                let mut metric_total_len: usize = 0;
                for m in metrics.iter() {
                    metric_total_len += 8 + m.0.len() + m.1.len();
                }
                let total_len = 8 + metric_total_len + 2 + count * 4;
                res.reserve(total_len);
                res.put_u16_le(0xe004);
                res.put_u32_le(total_len as u32);
                res.put_u16_le(id);
                res.put_u8(relative as u8);
                res.put_u8(count as u8);
                for m in metrics.iter() {
                    res.put_u16_le(m.0.len() as u16);
                    res.put_slice(m.0.as_bytes());
                    res.put_u16_le(m.1.len() as u16);
                    res.put_slice(m.1.as_bytes());
                    res.put_u64_le(m.2);
                }
            },
        }
        Ok(())
    }
}

impl codec::Decoder for DracarysFramer {
    type Item = Dracarys;
    type Error = io::Error;

    fn decode(&mut self, bytes: &mut bytes::BytesMut) -> Result<Option<Dracarys>, io::Error> {
        // info!("Receiving {:?}", bytes.len());
        if bytes.len() < 8 { 
            // error!("Failed to decode message: {:?}", bytes);
            return Ok(None); 
        }
        let len = utils::get_u32_le(&bytes[2..6]) as usize;
        if bytes.len() < len {
            error!("Failed to decode message: {:?}", bytes);
            return Ok(None);
        }
        let id = utils::get_u16_le(&bytes[6..8]);
        let flag = utils::get_u16_le(&bytes[0..2]);
        let mut pos: usize = 8;

        macro_rules! read_string {
            () => {
                {
                    if pos + 2 > len {
                        error!("Failed to decode message: {:?}", bytes);
                        return Err(io::Error::new(io::ErrorKind::InvalidData, utils::CodecError));
                    }
                    let size = utils::get_u16_le(&bytes[pos..pos+2]) as usize;
                    pos += 2;
                    if pos + size > len {
                        error!("Failed to decode message: {:?}", bytes);
                        return Err(io::Error::new(io::ErrorKind::InvalidData, utils::CodecError));
                    }
                    pos += size;
                    match String::from_utf8(bytes[pos-size..pos].to_vec()) {
                        Ok(string) => string,
                        Err(_) => {
                            error!("Failed to decode message: {:?}", bytes);
                            return Err(io::Error::new(io::ErrorKind::InvalidData, utils::CodecError));
                        },
                    }
                }
            }
        }
        let msg: Dracarys;
        match flag {
            0xe001 => {
                let path_count = bytes[pos] as usize;
                pos += 1;
                let mut paths = Vec::new();
                for _ in 0..path_count {
                    paths.push(read_string!());
                }
                let name = read_string!();
                let extra = read_string!();
                msg = Dracarys::Target {
                    id,
                    paths,
                    name,
                    extra,
                };
                bytes.advance(pos);
            },
            0xe002 => {
                let health_status = bytes[pos] as u8; 
                pos += 1;
                msg = Dracarys::Report {
                    id,
                    health_status,
                };
                bytes.advance(pos);
            },
            0xe003 => {
                let data = read_string!();
                bytes.advance(pos);
                msg = Dracarys::Message { id, data };
            },
            0xe004 => {
                let relative = bytes[pos] as usize > 0;
                pos += 1;
                let count = bytes[pos] as usize;
                pos += 1;
                let mut metrics = Vec::new();
                for _ in 0..count {
                    metrics.push((read_string!(), read_string!(), utils::get_u64_le(&bytes[pos..pos+8])));
                    pos += 8;
                }
                msg = Dracarys::Metric {
                    id,
                    relative,
                    metrics,
                };
                bytes.advance(pos);
            },

            _ => {
                error!("Failed to decode message for unknown flag: {:?}", flag);
                return Err(io::Error::new(io::ErrorKind::InvalidData, utils::CodecError));
            }
        }
        // info!("Rx: {:?}", msg);
        Ok(Some(msg))
    }
}


