use futures::sink::SinkExt;
use futures::stream::StreamExt;
use http_body_util::Full;
use hyper::{
    body::{Bytes, Incoming},
    Request, Response,
};
use hyper_tungstenite::tungstenite::Message;
use hyper_tungstenite::HyperWebsocket;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

#[derive(Debug)]
pub enum Event<T> {
    ClientConnected { client_id: u32 },
    ClientDisconnected { client_id: u32 },
    Message { client_id: u32, msg: T },
}

/// `Server` part of `netcode`
pub struct Server<T: common::Message> {
    /// The address which the server is currently listening to
    listener_addr: Option<std::net::SocketAddr>,
    /// Token to cancel listening
    cancellation_token: Option<tokio_util::sync::CancellationToken>,
    event_receiver: Option<tokio::sync::mpsc::Receiver<Event<T>>>,
}
impl<T: common::Message> Default for Server<T> {
    fn default() -> Self {
        Self {
            listener_addr: None,
            cancellation_token: None,
            event_receiver: None,
        }
    }
}
type Error = Box<dyn std::error::Error + Send + Sync + 'static>;
impl<T: common::Message> Server<T> {
    /// Handle a websocket connection.
    async fn serve_websocket(websocket: HyperWebsocket) -> Result<(), Error> {
        let mut websocket = websocket.await?;
        while let Some(message) = websocket.next().await {
            match message? {
                Message::Text(msg) => {
                    println!("Received text message: {msg}");
                    websocket
                        .send(Message::text("Thank you, come again."))
                        .await?;
                }
                Message::Binary(msg) => {
                    println!("Received binary message: {msg:02X?}");
                    websocket
                        .send(Message::binary(b"Thank you, come again.".to_vec()))
                        .await?;
                }
                Message::Ping(msg) => {
                    // No need to send a reply: tungstenite takes care of this for you.
                    println!("Received ping message: {msg:02X?}");
                }
                Message::Pong(msg) => {
                    println!("Received pong message: {msg:02X?}");
                }
                Message::Close(msg) => {
                    // No need to send a reply: tungstenite takes care of this for you.
                    if let Some(msg) = &msg {
                        println!(
                            "Received close message with code {} and message: {}",
                            msg.code, msg.reason
                        );
                    } else {
                        println!("Received close message");
                    }
                }
                Message::Frame(_msg) => {
                    unreachable!();
                }
            }
        }

        Ok(())
    }
    async fn handle_request(
        mut request: Request<Incoming>,
    ) -> Result<Response<Full<Bytes>>, Error> {
        if hyper_tungstenite::is_upgrade_request(&request) {
            let (response, websocket) = hyper_tungstenite::upgrade(&mut request, None)?;
            tokio::spawn(async move {
                let _ = Self::serve_websocket(websocket).await;
            });
            Ok(response)
        } else {
            Ok(Response::new(Full::<Bytes>::from("Hello HTTP!")))
        }
    }

    fn spawn_listener(cancellation_token: CancellationToken, listener: TcpListener) {
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = cancellation_token.cancelled() => {
                        break;
                    }
                    stream = listener.accept() => {
                        if let Ok((stream, _)) = stream {
                            tokio::spawn(async move {
                                let _ = stream.set_nodelay(true);
                                let mut http_builder = hyper::server::conn::http1::Builder::new();
                                http_builder.keep_alive(true);
                                let connection = http_builder.serve_connection(hyper_util::rt::TokioIo::new(stream), hyper::service::service_fn(|request| {
                                    Self::handle_request(request)
                                }))
                                .with_upgrades();
                                let _ = connection.await;
                            });
                        }
                    }
                };
            }
        });
    }

    /// Listens on the specific port
    ///
    /// Returns `false` if port cannot be acquired
    pub async fn start(&mut self, port: u16) -> bool {
        let addr: std::net::SocketAddr = format!("0.0.0.0:{}", port)
            .parse()
            .expect("could not parse address");
        match tokio::net::TcpListener::bind(&addr).await {
            Ok(listener) => {
                self.listener_addr = Some(addr);
                let mut http = hyper::server::conn::http1::Builder::new();
                http.keep_alive(true);
                let cancellation_token = CancellationToken::new();
                self.cancellation_token = Some(cancellation_token.clone());
                Self::spawn_listener(cancellation_token, listener);
                return true;
            }
            Err(_) => {
                return false;
            }
        }
    }

    /// Collect and process events
    /// 
    /// Returns the processed events which can be further processed by the calling application 
    pub fn events(&mut self) -> Vec<Event<T>> {
        let events = Vec::default();
        events
    }

    /// Stops the server, aka stops listening for new connections and drops existing connections
    pub async fn stop(&mut self) {
        let token = self.cancellation_token.take();
        if let Some(token) = token {
            token.cancel();
            self.cancellation_token = None;
            self.listener_addr = None;
        }

        // yields back to tokio to ensure listener can be shutdown before returning from the function
        tokio::task::yield_now().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        tokio_test::block_on(async {
            let mut server = Server::default() as Server<String>;
            assert_eq!(server.start(8080).await, true);
            assert_eq!(server.start(8080).await, false);
            server.stop().await;
            assert_eq!(server.start(8080).await, true);
        });
    }
}
