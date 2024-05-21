use std::{collections::HashMap, ops::{Deref, DerefMut}};
use common::SerializableMessage;
use server::Server;

/// Hosts a single `server` acting as master for instances.
/// 
/// A client initially connects to the master and then through the master is directed to an instance.
pub struct MasterServer<T> where T:SerializableMessage {
    pub server:Server<T>,
    pub instances:HashMap<InstanceId, Instance<T>>
}

impl<T:SerializableMessage> MasterServer<T> {
    pub fn new(server:Server<T>) -> Self {
        Self {
            server,
            instances:Default::default()
        }
    }
    pub fn pool(&mut self) {
        
    }
}

#[derive(Clone, Copy, PartialEq, PartialOrd, Hash)]
pub struct InstanceId(u64);
pub struct Instance<T:SerializableMessage> {
    pub id:InstanceId,
    pub rx:Vec<T>,
    pub tx:Vec<T>
}