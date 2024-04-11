use std::time::Instant;

use serde::{de::DeserializeOwned, Serialize};

/// Serializable message between `Client` and `Server`
pub trait Msg: Send + Sync + Clone + Serialize + DeserializeOwned + 'static {}
impl<T> Msg for T where T: Send + Sync + Clone + Serialize + DeserializeOwned + 'static {}

/// Struct holdning common network metric information
pub struct Metrics {
    instant: Instant,
    recv_total: u64,
    recv_sec: u64,
    send_total: u64,
    send_sec: u64,
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            instant: Instant::now(),
            recv_total: 0,
            recv_sec: 0,
            send_total: 0,
            send_sec: 0,
        }
    }
}

impl Metrics {
    fn update(&mut self) {
        if self.instant.elapsed().as_secs() >= 1 {
            self.send_sec = self.send_total;
            self.recv_sec = self.recv_total;
            self.instant = Instant::now();
        }
    }
    pub fn add_send(&mut self, value:u64) {
        self.send_total += value;
        self.update();
    }
    pub fn add_recv(&mut self, value:u64) {
        self.recv_total += value;
        self.update();
    }
    pub fn recv_total(&mut self) -> u64 {
        self.update();
        self.recv_total
    }
    pub fn recv_per_sec(&mut self) -> u64 {
        self.update();
        self.recv_sec
    }
    pub fn send_total(&mut self) -> u64 {
        self.update();
        self.send_total
    }
    pub fn send_per_sec(&mut self) -> u64 {
        self.update();
        self.send_sec
    }
}
