use std::time::Duration;

use netcode::Client;
use tokio::task::yield_now;

#[tokio::main]
pub async fn main() {
    let mut client = Client::default() as Client<String>;
    client.connect("wss://echo.websocket.org").await;
    loop {
        for e in client.events() {
            dbg!("h");
        }
        yield_now().await;
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}