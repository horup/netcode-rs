//! example with a master server, some instances and some clients
use std::time::Duration;

use master_server::{InstanceEvent, MasterEvent, MasterServer};

#[derive(Serialize, Deserialize, Clone)]
enum Msg {
}

#[derive(Default)]
struct Instance { 
    pub tx:Vec<InstanceEvent<Msg>>,
    pub rx:Vec<InstanceEvent<Msg>>
}
impl master_server::Instance<Msg> for Instance {
    fn tx(&mut self, t:InstanceEvent<Msg>) {
        self.tx.push(t);
    }

    fn poll(&mut self) -> Vec<InstanceEvent<Msg>> {
        std::mem::take(&mut self.rx)
    }
}

use serde::{Deserialize, Serialize};
use server::Server;
#[tokio::main]
async fn main() {
    let server = async {
        let mut server = Server::default();
        server.start(8080).await;
        let mut master_server:MasterServer<Msg, Instance> = MasterServer::new(server);
        let mut instances:Vec<Instance> = Vec::new();
        loop {
            let events = master_server.poll();
            for e in events {
                match e {
                    MasterEvent::ClientConnected { client_id } => {
                        println!("{:?} connected", client_id);
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
                    MasterEvent::Message { .. } => {},
                }
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    };

    let server_handle = tokio::spawn(server);

    let _ = server_handle.await;
}