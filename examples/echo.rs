use tokio::task::yield_now;

#[tokio::main]
pub async fn main() {
    // spawn a server
    tokio::spawn(async {
        use netcode::server::*;
        let mut server = Server::default() as Server<String>;
        server.start(8080).await;
        loop {
            for e in server.events() {
                if let Event::Message { client_id, msg } = e {
                    todo!();
                }
            }
            yield_now().await;
        }
    });

    // spawn client
    let _ = tokio::spawn(async {
        use netcode::client::*;
        let mut client = Client::default() as Client<String>;
        client.connect("ws://localhost:8080").await;
        loop {
            for e in client.events() {
                if let Event::Message(msg) = &e {
                    println!("{}", msg);
                }
            }

            if client.state() == State::Connected {
                client.send("hello world".to_owned());
            }
            yield_now().await;
        }
    })
    .await;
}
