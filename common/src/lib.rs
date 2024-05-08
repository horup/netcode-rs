use std::time::Instant;

use serde::{de::DeserializeOwned, Serialize};

/// Messaging format used
#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Format {
    Binary,
    Json
}

impl Default for Format {
    fn default() -> Self {
        Self::Binary
    }
}

/// Trait messages must implement to ensure they can be serialized as `Binary` or as `Json`
pub trait SerializableMessage : Clone + Send + Sync + 'static {
    fn to_bytes(&self) -> Result<Vec<u8>, ()>;
    fn from_bytes(bytes:&[u8]) -> Result<Self, ()>;
    fn to_json(&self) -> Result<String, ()>;
    fn from_json(json:&str) -> Result<Self, ()>;
}

/// SerializableMessage is implemented for types that are serde serialiazable
impl<T> SerializableMessage for T where T: Serialize + DeserializeOwned + Sync + Send + Clone + 'static {
    fn to_bytes(&self) -> Result<Vec<u8>, ()> {
        bincode::serialize(self).map_err(|_|())
    }

    fn from_bytes(bytes:&[u8]) -> Result<Self, ()> {
        bincode::deserialize(bytes).map_err(|_|())
    }

    fn to_json(&self) -> Result<String, ()> {
        serde_json::to_string(self).map_err(|_|())
    }

    fn from_json(json:&str) -> Result<Self, ()> {
        serde_json::from_str(json).map_err(|_|())
    }
} 

/// Struct holdning common network metric information
pub struct Metrics {
    instant: Instant,
    recv_total: usize,
    recv_sec: usize,
    recv_sec_buffer: usize,
    send_total: usize,
    send_sec: usize,
    send_sec_buffer: usize,
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            instant: Instant::now(),
            recv_total: 0,
            recv_sec: 0,
            send_total: 0,
            send_sec: 0,
            recv_sec_buffer:0,
            send_sec_buffer:0
        }
    }
}

impl Metrics {
    fn update(&mut self) {
        if self.instant.elapsed().as_secs() >= 1 {
            self.send_sec = self.send_sec_buffer;
            self.recv_sec = self.recv_sec_buffer;
            self.send_sec_buffer = 0;
            self.recv_sec_buffer = 0;
            self.instant = Instant::now();
        }
    }
    pub fn add_send(&mut self, value:usize) {
        self.send_total += value;
        self.send_sec_buffer += value;
        self.update();
    }
    pub fn add_recv(&mut self, value:usize) {
        self.recv_total += value;
        self.recv_sec_buffer += value;
        self.update();
    }
    pub fn recv_total(&mut self) -> usize {
        self.update();
        self.recv_total
    }
    pub fn recv_per_sec(&mut self) -> usize {
        self.update();
        self.recv_sec
    }
    pub fn send_total(&mut self) -> usize {
        self.update();
        self.send_total
    }
    pub fn send_per_sec(&mut self) -> usize {
        self.update();
        self.send_sec
    }
}
