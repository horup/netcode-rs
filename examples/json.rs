use std::time::Duration;
/// server client using json
#[tokio::main]
pub async fn main() {
    // spawn a server
    let server_handle = tokio::spawn(async {
        use netcode::server::*;
        let mut server:Server<String> = Server::default().with_format(common::Format::Json);
        server.start(8080).await;
        loop {
            for e in server.poll() {
                if let Event::Message { client_id, msg } = e {
                    server.send(client_id, format!("echo from server: {}", msg));
                }
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });

    let mut clients = Vec::default();
    // spawn clients
    for i in 0..10 {
        let j = tokio::spawn(async move {
            use netcode::client::*;
            let mut client:Client<String> = Client::default().with_format(common::Format::Json);
            client.connect("ws://localhost:8080");
            loop {
                for e in client.poll() {
                    if let Event::Message(msg) = &e {
                        println!("{}", msg);
                        client.disconnect();
                    }
                }

                if client.state() == State::Connected {
                    client.send(format!("hello from {}", i));
                }

                tokio::time::sleep(Duration::from_millis(1000)).await;
            }
        });
        clients.push(j);
    }

    for client in clients {
        let _ = client.await;
    }
    
    dbg!("all done");
    server_handle.abort();
}
