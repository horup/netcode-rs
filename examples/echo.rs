use server::Server;
use tokio::task::yield_now;

#[tokio::main]
pub async fn main() {
    // spawn a server
    tokio::spawn(async {
        use netcode::server::*;
        let mut server = Server::default();
        server.start(8080).await;
        loop {
            yield_now().await;
        }
    });

    // spawn client
    let r = tokio::spawn(async {
        use netcode::client::*;
        let mut client = Client::default() as Client<String>;
        client.connect("ws://localhost:8080").await;
        loop {
            for e in client.events() {
                dbg!(e);
            }
            yield_now().await;
        }
    })
    .await;
}
