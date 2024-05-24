use std::{collections::HashMap, ops::{Deref, DerefMut}};
use common::SerializableMessage;
use server::{ClientId, Event, Server};

pub enum InstanceEvent<T> {
    ClientJoinedInstance {client_id:ClientId}, 
    ClientLeftInstance {client_id:ClientId},
    Message { client_id: ClientId, msg: T }
}

pub enum MasterEvent<T> {
    ClientConnected {client_id:ClientId}, 
    ClientDisconnected {client_id:ClientId},
    ClientJoinedInstance {client_id:ClientId, instance_id:InstanceId},
    ClientLeftInstance {client_id:ClientId, instance_id:InstanceId},
    Message { client_id: ClientId, msg: T }
}

/// Hosts a single `server` acting as master for instances.
/// 
/// A client initially connects to the master and then through the master is directed to an instance.
pub struct MasterServer<T, I> where T:SerializableMessage, I:Instance<T> {
    server:Server<T>,
    instances:HashMap<InstanceId, I>,
    client_instances:HashMap<ClientId, InstanceId>
}

impl<T:SerializableMessage, I:Instance<T>> MasterServer<T, I> {
    pub fn new(server:Server<T>) -> Self {
        Self {
            server,
            instances:Default::default(),
            client_instances:Default::default()
        }
    }

    /// Returns the `InstanceId`, if any, that the client with the given `ClientId` belongs.
    pub fn client_instance(&self, client_id:ClientId) -> Option<InstanceId> {
        let instance_id = self.client_instances.get(&client_id)?;
        Some(instance_id.to_owned())
    }

    /// Removes the client from the `Instance`, if any
    /// 
    /// Returns the `InstanceId` of the instance the client had joined
    pub fn leave_instance(&mut self, client_id:ClientId) -> Option<InstanceId> {
        self.client_instances.remove(&client_id)
    }

    pub fn join_instance(&mut self, client_id:ClientId, instance_id:InstanceId) {

    }

    /// Poll the master server, which ensures events will be propergated to and from the respective instances.
    /// 
    /// Returns events that were not processed in an `instance` i.e. which belongs to the master server itself.
    /// These events needs to be processed by the underlying application.
    pub fn poll(&mut self) -> Vec<MasterEvent<T>> {
        let mut master_events:Vec<MasterEvent<T>> = Vec::with_capacity(8);
        let mut instances = std::mem::take(&mut self.instances);
        let server_events = self.server.poll();

        for (iid, instance) in instances.iter_mut() {
            let mut instance_events = instance.poll();
            for e in instance_events.drain(..) {
                match e {
                    InstanceEvent::Message { client_id, msg } => {
                        self.server.send(client_id, msg);
                    },
                    InstanceEvent::ClientLeftInstance { client_id } => {
                        if let Some(iid) = self.leave_instance(client_id) {
                            master_events.push(MasterEvent::ClientLeftInstance { client_id: client_id, instance_id: iid });
                        }
                    },
                    _ => {}
                }
            }
        }

        let mut process_event = |e, client_id| {
            match self.client_instance(client_id) {
                Some(instance_id) => {
                    let Some(instance) = instances.get_mut(&instance_id) else { return };
                    instance.tx(e);
                },
                None => {
                    match e {
                        Event::ClientConnected { client_id } => {
                            master_events.push(MasterEvent::ClientConnected { client_id });
                        },
                        Event::ClientDisconnected { client_id } => {
                            master_events.push(MasterEvent::ClientDisconnected { client_id });
                        },
                        Event::Message { client_id, msg } => {
                            master_events.push(MasterEvent::Message { client_id, msg });
                        },
                    }
                },
            }
        };

        for e in server_events {
            match e {
                server::Event::ClientConnected { client_id } => {
                    process_event(e, client_id);
                },
                server::Event::ClientDisconnected { client_id } => {
                    process_event(e, client_id);
                },
                server::Event::Message { client_id, .. } => {
                    process_event(e, client_id);
                },
            }
        }
        
        self.instances = instances;
        master_events
    }
}

#[derive(Clone, Copy, PartialEq, PartialOrd, Hash, Eq)]
pub struct InstanceId(u64);
pub trait Instance<T:SerializableMessage> {
    /// send `Event` into the instance
    fn tx(&mut self, t:Event<T>);

    /// poll the `Instance`, producing x number of events that needs to be processed
    fn poll(&mut self) -> Vec<InstanceEvent<T>>;
}


