use bytes;
use bytes::BufMut;
use tokio::codec;
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
        let mut msg: Dracarys;
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
            _ => {
                error!("Failed to decode message for unknown flag: {:?}", flag);
                return Err(io::Error::new(io::ErrorKind::InvalidData, utils::CodecError));
            }
        }
        // info!("Rx: {:?}", msg);
        Ok(Some(msg))
    }
}


