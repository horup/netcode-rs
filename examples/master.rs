//! example with a master server, some instances and some clients
use std::{cell::RefCell, rc::Rc, time::Duration};

use master_server::{InstanceEvent, InstanceId, MasterEvent, MasterServer};

#[derive(Serialize, Deserialize, Clone)]
enum Msg {
    Hello
}
#[derive(Clone)]
struct Instance { 
    pub id:Rc<InstanceId>,
    pub tx:Rc<RefCell<Vec<InstanceEvent<Msg>>>>,
    pub rx:Rc<RefCell<Vec<InstanceEvent<Msg>>>>
}
unsafe impl Send for Instance {}

impl master_server::Instance<Msg> for Instance {
    fn tx(&mut self, t:InstanceEvent<Msg>) {
        self.tx.borrow_mut().push(t);
    }

    fn poll(&mut self) -> Vec<InstanceEvent<Msg>> {
        std::mem::take(&mut self.rx.borrow_mut())
    }
}

impl Instance {
    pub fn update(&mut self) {
        let events = self.tx.borrow_mut().clone();
        self.tx.borrow_mut().clear();
        for e in events {
            match e {
                InstanceEvent::ClientJoinedInstance { client_id } => {
                    println!("{:?}>client {:?} joined", self.id, client_id);
                },
                InstanceEvent::ClientLeftInstance { client_id } => {
                    println!("{:?}>client {:?} left", self.id, client_id);
                },
                InstanceEvent::Message { client_id, msg } => {
                    println!("{:?}>client {:?} sent a message", self.id, client_id);
                },
            }
        }
    }
}

/// example where all clients are assigned their own instance after sending the first Hello message
use serde::{Deserialize, Serialize};
use server::Server;
#[tokio::main]
async fn main() {
    let server = async {
        let mut server = Server::default().with_format(common::Format::Json);
        server.start(8080).await;
        let mut master_server:MasterServer<Msg, Instance> = MasterServer::new(server);
        let mut instances:Vec<Instance> = Vec::new();
        loop {
            let events = master_server.poll();
            for e in events {
                match e {
                    MasterEvent::ClientConnected { client_id } => {
                        println!("{:?} connected", client_id);
                        master_server.send(client_id, Msg::Hello);
                    },
                    MasterEvent::ClientDisconnected { client_id } => {
                        println!("{:?} disconnceted", client_id);
                    },
                    MasterEvent::ClientLeftInstance { client_id, instance_id } => {
                        println!("{:?} left instance {:?}", client_id, instance_id);
                    },
                    MasterEvent::ClientJoinedInstance { client_id, instance_id } => {
                        println!("{:?} joined instance {:?}", client_id, instance_id);
                    },
                    MasterEvent::Message { client_id, msg } => {
                        match msg {
                            Msg::Hello => {
                                let instance_id: InstanceId = master_server.push_instance(|id|{
                                    let instance = Instance { id:id.into(), tx: Default::default(), rx: Default::default() };
                                    instances.push(instance.clone());
                                    instance
                                });
                                master_server.join_instance(client_id, instance_id);
                            },
                        }
                    },
                }
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    };

    let server_handle = tokio::spawn(server);
    let _ = server_handle.await;
}