use std::{collections::HashMap, time::{Duration, Instant}};

use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
enum Msg {
    StateUpdate(HashMap<u64, Player>),
    MovePlayer(f32, f32)
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Player {
    pub id:u64,
    pub x:f32,
    pub y:f32,
}

pub fn spawn_client() {
    tokio::spawn(async {
        use netcode::client::*;
        let mut client = Client::default() as Client<Msg>;
        client.connect("ws://localhost:8080").await;
        let tick_rate = 20;
        let target = Duration::from_millis(1000 / tick_rate);
        loop {
            let now = Instant::now();
            for e in client.poll() {}
            let took = Instant::now() - now;
            if target > took {
                let sleep = target - took;
                client.send(Msg::MovePlayer(rand::random::<f32>() - 0.5, rand::random::<f32>() - 0.5));
                tokio::time::sleep(sleep).await;
            }
        }
    });
}

pub fn spawn_server() -> tokio::task::JoinHandle<()> {
    let server_handle = tokio::spawn(async {
        use netcode::server::*;
        let tick_rate = 20;
        let target = Duration::from_millis(1000 / tick_rate);
        let mut server = Server::default() as Server<Msg>;
        server.start(8080).await;
        let mut connected = 0;
        let mut players = HashMap::default() as HashMap<u64, Player>;
        loop {
            let now = Instant::now();
            for e in server.poll() {
                match e {
                    Event::ClientConnected { client_id } => {
                        players.insert(u64::from(client_id), Player {
                            id: client_id.into(),
                            x: 0.0,
                            y: 0.0,
                        });
                        connected += 1;
                    }
                    Event::ClientDisconnected { client_id } => {
                        connected -= 1;
                        players.remove(&client_id.into());
                    }
                    Event::Message { client_id, msg } => {
                        match msg {
                            Msg::MovePlayer(dx, dy) => {
                                if let Some(player) = players.get_mut(&client_id.into()) {
                                    player.x += dx;
                                    player.y += dy;
                                }
                            },
                            _ => {}
                        }
                    }
                }
            }

            let connected_clients:Vec<ClientId> = players.keys().map(|x|ClientId::from(*x)).collect();
            let mut dummy_hashmap = HashMap::default();
            for i in 0..10 {
                dummy_hashmap.insert(i, Player { id: i, x: i as f32, y: i as f32 });
            }
            for client_id in connected_clients {
                let res = server.send(client_id, Msg::StateUpdate(dummy_hashmap.clone()));
                if res == false {
                    panic!();
                }
            }
            

            let took = Instant::now() - now;

            println!("Connected: {}, Took: {}ms", connected, took.as_millis());

            if target > took {
                let sleep = target - took;
                tokio::time::sleep(sleep).await;
            }
        }
    });
    return server_handle;
}

/// scaling example where many clients connect to a single server
#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
pub async fn main() {
    // spawn a server
    let server_handle = spawn_server();

    let num_clients = 100;
    for _ in 0..num_clients {
        spawn_client();
    }

    let _ = server_handle.await;
}
