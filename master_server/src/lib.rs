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
    ClientLeftInstance {client_id:ClientId, instance_id:InstanceId},
    ClientJoinedInstance {client_id:ClientId, instance_id:InstanceId},
    Message { client_id: ClientId, msg: T }
}

/// Hosts a single `server` acting as master for instances.
/// 
/// A client initially connects to the master and then through the master is directed to an instance.
pub struct MasterServer<T, I> where T:SerializableMessage, I:Instance<T> {
    server:Server<T>,
    instances:HashMap<InstanceId, I>,
    client_instances:HashMap<ClientId, InstanceId>,
    next_instance_id:u64,
    events:Vec<MasterEvent<T>>
}

impl<T:SerializableMessage, I:Instance<T>> MasterServer<T, I> {
    pub fn new(server:Server<T>) -> Self {
        Self {
            server,
            instances:Default::default(),
            client_instances:Default::default(),
            next_instance_id:1,
            events:Default::default()
        }
    }

    /// Pushes a new instance, which will be periodically polled by the `MasterServer`
    pub fn push_instance(&mut self, instance:I) -> InstanceId {
        let id = InstanceId(self.next_instance_id);
        self.next_instance_id += 1;
        self.instances.insert(id, instance);
        id
    }

    /// Removes the given instance.
    /// 
    /// Will move all clients from the instance
    pub fn remove_instance(&mut self, instance_id:InstanceId) -> Option<I> {
        let client_instances = self.client_instances.clone();
        for (client_id, instance_id2) in client_instances.iter() {
            if &instance_id == instance_id2 {
                self.leave_instance(client_id.to_owned());
            }
        }
        self.instances.remove(&instance_id)
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
        match self.client_instances.remove(&client_id) {
            Some(iid) => {
                self.events.push(MasterEvent::ClientLeftInstance { client_id: client_id, instance_id: iid });
                return Some(iid);
            },
            None => return None,
        };
    }

    /// Add the client to the `Instance` denoted by `instance_id`
    /// 
    /// Returns the previous `Instance` that the client was part of. 
    pub fn join_instance(&mut self, client_id:ClientId, instance_id:InstanceId) -> Option<InstanceId> {
        let old_instance = self.leave_instance(client_id);
        self.client_instances.insert(client_id, instance_id);
        self.events.push(MasterEvent::ClientJoinedInstance { client_id: client_id, instance_id: instance_id });
        old_instance
    }

    /// Poll the master server, which ensures events will be propergated to and from the respective instances.
    /// 
    /// Returns events that were not processed in an `instance` i.e. which belongs to the master server itself.
    /// These events needs to be processed by the underlying application.
    pub fn poll(&mut self) -> Vec<MasterEvent<T>> {
        let mut instances = std::mem::take(&mut self.instances);
        let server_events = self.server.poll();
        let mut master_events = std::mem::take(&mut self.events);

        for (_, instance) in instances.iter_mut() {
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


