use std::time::{SystemTime, UNIX_EPOCH};
use serde_json::Value;
use serde_json::map::Map;
use serde_json::value::Index;
use std::error::Error;
use std::fmt;

pub type JsonMap = Map<String, Value>;
#[allow(dead_code)]
pub type AsyncRes = Result<(), Box<dyn Error>>;

pub trait JsonParser {
    fn get_bool<I: Index>(&self, index: I, default: bool) -> bool;
    fn get_string<I: Index>(&self, index: I, default: String) -> String;
    fn get_str<I: Index>(&self, index: I, default: &str) -> String;
    fn get_u64<I: Index>(&self, index: I, default: u64) -> u64;
    fn get_f64<I: Index>(&self, index: I, default: f64) -> f64;
}
#[allow(dead_code)]
#[inline]
pub fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}

#[allow(dead_code)]
#[inline]
pub fn now_ms() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis()
}

#[allow(dead_code)]
impl JsonParser for Value {
    fn get_bool<I: Index>(&self, index: I, default: bool) -> bool {
        match self[index].as_bool() {
            Some(v) => v,
            None => default,
        }
    }
    fn get_string<I: Index>(&self, index: I, default: String) -> String {
        match self[index].as_str() {
            Some(v) => v.to_string(),
            None => default,
        }
    }
    fn get_str<I: Index>(&self, index: I, default: &str) -> String {
        match self[index].as_str() {
            Some(v) => v.to_string(),
            None => default.to_string(),
        }
    }

    fn get_u64<I: Index>(&self, index: I, default: u64) -> u64 {
        match self[index].as_u64() {
            Some(v) => v,
            None => default,
        }
    }

    fn get_f64<I: Index>(&self, index: I, default: f64) -> f64 {
        match self[index].as_f64() {
            Some(v) => v,
            None => default,
        }
    }
}

#[allow(dead_code)]
#[inline]
pub fn get_u16_le(v: &[u8]) -> u16 {
    ((v[1] as u16) << 8) | v[0] as u16
}

#[allow(dead_code)]
#[inline]
pub fn get_u32_le(v: &[u8]) -> u32 {
    ((v[3] as u32) << 24) |
    ((v[2] as u32) << 16) |
    ((v[1] as u32) << 8) |
    v[0] as u32
}

#[derive(Debug)]
pub struct CodecError;
impl fmt::Display for CodecError {
	fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
		fmt.write_str("Bad data")
	}
}
impl Error for CodecError {
	fn description(&self) -> &str {
		"Bad data"
	}
}
