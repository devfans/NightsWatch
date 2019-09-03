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
        pathes: Vec<String>,
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
        match msg {
            Dracarys::Target { id, ref pathes, ref name, ref extra} => {
                let path_count = pathes.len();
                let mut path_total_len: usize = 0;
                for path in pathes.iter() {
                    path_total_len += path.len();
                }
                let data_len = path_total_len + name.len() + extra.len();
                let total_len = 8 + data_len + 4 + 1 + path_count * 2;
                res.reserve(total_len);
                res.put_u16_le(0xe001);
                res.put_u32_le(total_len as u32);
                res.put_u16_le(id);
                res.put_u8(path_count as u8);
                for path in pathes.iter() {
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
        if bytes.len() < 8 { return Ok(None) }
        let len = utils::get_u32_le(&bytes[2..6]) as usize;
        if bytes.len() < len { return Ok(None) }
        let id = utils::get_u16_le(&bytes[6..8]);
        let flag = utils::get_u16_le(&bytes[0..2]);
        let mut pos: usize = 8;
        macro_rules! read_string {
            () => {
                {
                    if pos + 2 > len {
                        return Err(io::Error::new(io::ErrorKind::InvalidData, utils::CodecError));
                    }
                    let size = utils::get_u16_le(&bytes[pos..pos+2]) as usize;
                    pos += 2;
                    if pos + size > len {
                        return Err(io::Error::new(io::ErrorKind::InvalidData, utils::CodecError));
                    }
                    pos += size;
                    match String::from_utf8(bytes[pos..pos+size].to_vec()) {
                        Ok(string) => string,
                        Err(_) => {
                            return Err(io::Error::new(io::ErrorKind::InvalidData, utils::CodecError));
                        },
                    }
                }
            }
        }
        match flag {
            0xe001 => {
                let path_count = bytes[pos] as usize;
                pos += 1;
                let mut pathes = Vec::new();
                for _ in 0..path_count {
                    pathes.push(read_string!());
                }
                let name = read_string!();
                let extra = read_string!();
                let msg = Dracarys::Target {
                    id,
                    pathes,
                    name,
                    extra,
                };
                bytes.advance(pos);
                Ok(Some(msg))
            },
            0xe002 => {
                let health_status = bytes[pos] as u8; 
                let msg = Dracarys::Report {
                    id,
                    health_status,
                };
                bytes.advance(7);
                Ok(Some(msg))
            },
            0xe003 => {
                let data = read_string!();
                bytes.advance(pos);
                Ok(Some(Dracarys::Message { id, data }))
            },
            _ => {
                return Err(io::Error::new(io::ErrorKind::InvalidData, utils::CodecError));
            }
        }
    }
}


