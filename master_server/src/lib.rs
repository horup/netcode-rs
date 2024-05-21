use std::{collections::HashMap, ops::{Deref, DerefMut}};
use common::SerializableMessage;
use server::{ClientId, Event, Server};

/// Hosts a single `server` acting as master for instances.
/// 
/// A client initially connects to the master and then through the master is directed to an instance.
pub struct MasterServer<T, I> where T:SerializableMessage, I:Instance<T> {
    server:Server<T>,
    instances:HashMap<InstanceId, I>,
    client_instances:HashMap<ClientId, Option<InstanceId>>
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
        let instance_id = self.client_instances.get(&client_id)?.as_ref()?;
        Some(instance_id.to_owned())
    }

    /// Pools the master server, which ensures events will be propergated to the respective 
    /// `Instances` where they can be handled by the respective `Instance`
    /// 
    /// Returns events that do not belong to an `instance`.
    pub fn poll(&mut self) -> Vec<Event<T>> {
        let mut master_events = Vec::with_capacity(8);
        let mut instances = std::mem::take(&mut self.instances);
        let events = self.server.poll();
        let mut process_event = |e, client_id| {
            match self.client_instance(client_id) {
                Some(instance_id) => {
                    let Some(instance) = instances.get_mut(&instance_id) else { return };
                    instance.tx(e);
                },
                None => master_events.push(e),
            }
        };
        for e in events {
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
    /// send event into the instance
    fn tx(&mut self, t:Event<T>);

    /// poll the instance, producing x number of events that can be processed
    fn poll(&mut self) -> Vec<Event<T>>;
}


