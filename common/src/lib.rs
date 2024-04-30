use std::time::Instant;

use serde::{de::DeserializeOwned, Serialize};

/// Messaging format used
pub enum Format {
    Bincode,
    Json
}

impl Default for Format {
    fn default() -> Self {
        Self::Bincode
    }
}

/// Serializable message between `Client` and `Server`
pub trait Msg: Send + Sync + Clone + Serialize + DeserializeOwned + 'static {}
impl<T> Msg for T where T: Send + Sync + Clone + Serialize + DeserializeOwned + 'static {}

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
