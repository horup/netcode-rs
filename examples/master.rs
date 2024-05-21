//! example with a master server, some instances and some clients
use std::time::Duration;

use master_server::MasterServer;

#[derive(Serialize, Deserialize, Clone)]
enum Msg {
}

#[derive(Default)]
struct Instance { 
    pub tx:Vec<Event<Msg>>,
    pub rx:Vec<Event<Msg>>
}
impl master_server::Instance<Msg> for Instance {
    fn tx(&mut self, t:Event<Msg>) {
        self.tx.push(t);
    }

    fn poll(&mut self) -> Vec<Event<Msg>> {
        std::mem::take(&mut self.rx)
    }
}

use serde::{Deserialize, Serialize};
use server::{Event, Server};
#[tokio::main]
async fn main() {
    let server = async {
        let mut server = Server::default();
        server.start(8080).await;
        let mut master_server:MasterServer<Msg, Instance> = MasterServer::new(server);
        let mut instances:Vec<Instance> = Vec::new();
        loop {
            let events = master_server.poll();
            for _e in events {
    
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    };

    let server_handle = tokio::spawn(server);

    let _ = server_handle.await;
}