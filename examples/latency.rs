use std::time::Instant;
use serde::{Deserialize, Serialize};
use tokio::task::yield_now;

#[derive(Clone, Serialize, Deserialize)]
enum Msg {
    Ping(u128),
    Pong(u128)
}

/// latency example
/// 
/// where ping time is measured between server and client
#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
pub async fn main() {
    let server_handle = spawn_server();
    let num_clients = 10;
    for i in 0..num_clients {
        spawn_client(i);
    }

    let _ = server_handle.await;
}


pub fn spawn_client(i:i32) {
    tokio::spawn(async move {
        use netcode::client::*;
        let mut client = Client::default() as Client<Msg>;
        client.connect("ws://localhost:8080").await;
        let clock = Instant::now();
        let mut next = Instant::now();
        loop {
            for e in client.poll() {
                match e {
                    Event::Message(msg) => {
                        match msg {
                            Msg::Pong(v) => {
                                let now = clock.elapsed().as_millis();
                                let took = now - v;
                                if i == 0 {
                                    println!("{}:latency:{}ms", i, took);
                                }
                            }
                            _ => {}
                        }
                    },
                    _ => {}
                }
            }
            
            if next.elapsed().as_millis() >= 50 {
                client.send(Msg::Ping(clock.elapsed().as_millis()));
                next = Instant::now();
            }
            yield_now().await;
        }
    });
}

pub fn spawn_server() -> tokio::task::JoinHandle<()> {
    
    tokio::spawn(async {
        use netcode::server::*;
        let mut server = Server::default() as Server<Msg>;
        server.start(8080).await;
        loop {
            for e in server.poll() {
                match e {
                    Event::ClientConnected { client_id: _ } => {
                    }
                    Event::ClientDisconnected { client_id: _ } => {
                    }
                    Event::Message { client_id, msg } => {
                        match msg {
                            Msg::Ping(v) => {
                                server.send(client_id, Msg::Pong(v));
                            },
                            _ => {}
                        }
                    }
                }
            }

            yield_now().await;
        }
    })
}
