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

use std::time::{SystemTime, UNIX_EPOCH, Duration, Instant};
use serde_json::Value;
use serde_json::map::Map;
use serde_json::value::Index;
use std::error::Error;
use std::fmt;
use tokio::timer::delay;
use chrono::{ prelude::DateTime, Utc};
use crate::node::{NodeType, HealthCheckType};

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
pub fn read_tsp(secs: u64) -> String {
    DateTime::<Utc>::from(UNIX_EPOCH + Duration::from_secs(secs)).format("%Y-%m-%d %H:%M:%S.%f").to_string()
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

#[allow(dead_code)]
#[inline]
pub fn get_u64_le(v: &[u8]) -> u64 {
    ((v[7] as u64) << 56) |
    ((v[6] as u64) << 48) |
    ((v[5] as u64) << 40) |
    ((v[4] as u64) << 32) |
    ((v[3] as u64) << 24) |
    ((v[2] as u64) << 16) |
    ((v[1] as u64) << 8) |
    v[0] as u64
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

#[allow(dead_code)]
pub async fn sleep(sleep_ms: u64) -> AsyncRes {
    delay(Instant::now() + Duration::from_millis(sleep_ms)).await;
    Ok(())
}

macro_rules! show_name {
    ($type: ty) => {
        impl fmt::Display for $type {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "{:?}", self)
            }
        }
    }
}

show_name!(NodeType);
show_name!(HealthCheckType);



