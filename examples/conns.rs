use std::time::{Duration, Instant};

use client::{Client, State};
/// example where clients connect, drops and server drops at some point
#[tokio::main]
pub async fn main() {
    tokio::spawn(async {
        use netcode::server::*;
        let mut server = Server::default() as Server<String>;
        server.start(8080).await;
        let kill_server_after_sec = 10;
        let now = Instant::now();
        loop {
            let _ = server.poll();
            tokio::time::sleep(Duration::from_millis(100)).await;
            if now.elapsed().as_secs() > kill_server_after_sec {
                println!("server> stopping...");
                break;
            }
        }
    });


    loop {
        let r = tokio::spawn(async move {
            let exit_when_elapsed_sec = 7;
            let now = Instant::now();
            let mut client = Client::default() as Client<String>;
            client.connect("ws://localhost:8080");
            loop {
                for e in client.poll() {
                    match e {
                        client::Event::Connecting => {
                            println!("client> connecting!");
                        },
                        client::Event::Connected => {
                            println!("client> connected!");
                        },
                        client::Event::Disconnected => {
                            println!("client> disconnected!");
                        },
                        client::Event::Message(_) => {},
                    }
                }
                if now.elapsed().as_secs() > exit_when_elapsed_sec {
                    if client.state() == State::Connected {
                        client.disconnect();
                    } else {
                        break;
                    }
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        });
        let _ = r.await;
        println!("------");
    }
    
}
