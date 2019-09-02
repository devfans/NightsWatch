use std::time::{SystemTime, UNIX_EPOCH};
use serde_json::Value;
use serde_json::map::Map;
use serde_json::value::Index;

pub type JsonMap = Map<String, Value>;

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
