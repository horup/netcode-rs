use std::time::Duration;

#[tokio::main]
pub async fn main() {
    // spawn a server
    tokio::spawn(async {
        use netcode::server::*;
        let mut server = Server::default() as Server<String>;
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

    // spawn client
    let _ = tokio::spawn(async {
        use netcode::client::*;
        let mut client = Client::default() as Client<String>;
        client.connect("ws://localhost:8080").await;
        loop {
            for e in client.poll() {
                if let Event::Message(msg) = &e {
                    println!("{}", msg);
                }
            }

            if client.state() == State::Connected {
                client.send("hello world".to_owned());
            }

            tokio::time::sleep(Duration::from_millis(1000)).await;
        }
    })
    .await;
}
