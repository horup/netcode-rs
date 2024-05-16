use common::SerializableMessage;
use server::Server;

/// Hosts a single `server` acting as master for instances.
/// 
/// A client initially connects to the master and then through the master is directed to an instance.
pub struct MasterServer<T> where T:SerializableMessage {
    pub server:Server<T>
}

pub struct Instance<T:SerializableMessage> {
    pub rx:Vec<T>,
    pub tx:Vec<T>
}