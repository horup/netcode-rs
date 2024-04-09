#![feature(noop_waker)]

use std::{collections::HashMap, task::Waker};

use futures::{select, sink::SinkExt};
use futures::stream::StreamExt;
use http_body_util::Full;
use hyper::{
    body::{Bytes, Incoming},
    Request, Response,
};
use hyper_tungstenite::tungstenite::Message;
use hyper_tungstenite::HyperWebsocket;
use tokio::{net::TcpListener, sync::mpsc::Sender};
use tokio_util::sync::CancellationToken;

/// Unique id of a client
#[derive(Hash, PartialEq, Eq, Clone, Copy)]
pub struct ClientId(u64);

pub struct Client {
    pub sink:futures::prelude::stream::SplitSink<hyper_tungstenite::WebSocketStream<hyper_util::rt::TokioIo<hyper::upgrade::Upgraded>>, Message>
}

pub enum InternalEvent<T> {
    ClientConnected { client_id: ClientId, sink:futures::prelude::stream::SplitSink<hyper_tungstenite::WebSocketStream<hyper_util::rt::TokioIo<hyper::upgrade::Upgraded>>, Message> },
    ClientDisconnected { client_id: ClientId },
    Message { client_id: ClientId, msg: T },
}

pub enum Event<T> {
    ClientConnected { client_id:ClientId },
    ClientDisconnected {client_id:ClientId},
    Message { client_id: ClientId, msg: T }
}

/// `Server` part of `netcode`
pub struct Server<T: common::Message> {
    /// The address which the server is currently listening to
    listener_addr: Option<std::net::SocketAddr>,
    /// Token to cancel listening
    cancellation_token: Option<tokio_util::sync::CancellationToken>,
    /// receiver of events from connected clients
    event_receiver: Option<tokio::sync::mpsc::Receiver<InternalEvent<T>>>,
    clients:HashMap<ClientId, Client>
}
impl<T: common::Message> Default for Server<T> {
    fn default() -> Self {
        Self {
            listener_addr: None,
            cancellation_token: None,
            event_receiver: None,
            clients:HashMap::default()
        }
    }
}
type Error = Box<dyn std::error::Error + Send + Sync + 'static>;
impl<T: common::Message> Server<T> {
    /// Handle a websocket connection.
    async fn spawn_client(websocket: HyperWebsocket, event_sender:Sender<InternalEvent<T>>, client_id:ClientId, cancellation_token: CancellationToken) -> Result<(), Error> {
        let websocket = websocket.await?;
        let (sink, mut stream) = websocket.split();
        event_sender.send(InternalEvent::ClientConnected { client_id: client_id, sink: sink }).await?;
        loop {
            tokio::select! {
                _ = cancellation_token.cancelled() => {
                    break;
                }
                message = stream.next() => {
                    match message {
                        Some(message) => {
                            match message {
                                Ok(message) => {
                                    match message {
                                        Message::Binary(bincoded) => {
                                            let msg = bincode::deserialize(&bincoded) as Result<T, _>;
                                            match msg {
                                                Ok(msg) => {
                                                    let _ = event_sender.send(InternalEvent::Message { client_id: client_id, msg: msg }).await;
                                                }
                                                Err(_) => {
                                                    break;
                                                }
                                            }
                                        },
                                        _=>{}
                                    }
                                },
                                Err(_) => {
                                    break;
                                }
                            }
                        },
                        _=> {
                            break;
                        }
                    }
                }
            }
        }
      
        Ok(())
    }

    /// handle each http request
    /// 
    /// requests that are upgraded to websocket connections, are passed to a new task that takes care of polling the connection
    async fn handle_request(
        mut request: Request<Incoming>,
        event_sender:Sender<InternalEvent<T>>,
        client_id:ClientId,
        cancellation_token: CancellationToken
    ) -> Result<Response<Full<Bytes>>, Error> {
        if hyper_tungstenite::is_upgrade_request(&request) {
            let (response, websocket) = hyper_tungstenite::upgrade(&mut request, None)?;
            tokio::spawn(async move {
                let _ = Self::spawn_client(websocket, event_sender, client_id, cancellation_token).await;
            });
            Ok(response) as Result<Response<Full<Bytes>>, _>
        } else {
            Ok(Response::new(Full::<Bytes>::from("Hello HTTP!"))) as Result<Response<Full<Bytes>>, _>
        }
    }

    /// spawns tcp listener which accepts tcp connects
    /// 
    /// accepted connections are passed to a task where they are upgraded into http requests
    fn spawn_listener(cancellation_token: CancellationToken, listener: TcpListener, event_sender:Sender<InternalEvent<T>>) {
        tokio::spawn(async move {
            let mut next_client_id = 1;
            loop {
                let cancellation_token = cancellation_token.clone();
                tokio::select! {
                    _ = cancellation_token.cancelled() => {
                        break;
                    }
                    stream = listener.accept() => {
                        if let Ok((stream, _)) = stream {
                            let event_sender = event_sender.clone();
                            let client_id = ClientId(next_client_id);
                            next_client_id += 1;
                            tokio::spawn(async move {
                                let _ = stream.set_nodelay(true);
                                let mut http_builder = hyper::server::conn::http1::Builder::new();
                                http_builder.keep_alive(true);
                                let connection = http_builder.serve_connection(hyper_util::rt::TokioIo::new(stream), hyper::service::service_fn(|request| {
                                    let event_sender = event_sender.clone();
                                    let cancellation_token = cancellation_token.clone();
                                    Self::handle_request(request, event_sender, client_id, cancellation_token)
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
        self.stop().await;
        let addr: std::net::SocketAddr = format!("0.0.0.0:{}", port)
            .parse()
            .expect("could not parse address");
        let (event_sender, event_receiver) = tokio::sync::mpsc::channel(1024);
        self.event_receiver = Some(event_receiver);
        match tokio::net::TcpListener::bind(&addr).await {
            Ok(listener) => {
                self.listener_addr = Some(addr);
                let mut http = hyper::server::conn::http1::Builder::new();
                http.keep_alive(true);
                let cancellation_token = CancellationToken::new();
                self.cancellation_token = Some(cancellation_token.clone());
                Self::spawn_listener(cancellation_token, listener, event_sender);
                return true;
            }
            Err(_) => {
                return false;
            }
        }
    }

    /// Send a message to a client.
    /// 
    /// Returns `true` if the message was sent (but not neccesarily received)
    pub fn send(&mut self, client_id:impl Into<ClientId>, msg:T) -> bool {
        if let Some(client) = self.clients.get_mut(&client_id.into()) {
            let Ok(bincoded) = bincode::serialize(&msg) else { return false };
            match client.sink.start_send_unpin(Message::Binary(bincoded)) {
                Ok(_) => return true,
                Err(_) => return false,
            }
        }

        false
    }

    /// Collect and process events
    /// 
    /// Returns the processed events which can be further processed by the calling application 
    pub fn poll(&mut self) -> Vec<Event<T>> {
        let mut events = Vec::default();
        let mut cx = std::task::Context::from_waker(&Waker::noop());
        for (_, client) in self.clients.iter_mut() {
            let _ = client.sink.poll_flush_unpin(&mut cx);
        }
        if let Some(event_receiver) = &mut self.event_receiver {
            while let core::task::Poll::Ready(Some(e)) = event_receiver.poll_recv(&mut cx) {
                match e {
                    InternalEvent::ClientConnected { client_id, sink } => {
                        self.clients.insert(client_id, Client { sink });
                        events.push(Event::ClientConnected { client_id });
                    },
                    InternalEvent::ClientDisconnected { client_id } => {
                        self.clients.remove(&client_id);
                        events.push(Event::ClientDisconnected { client_id });
                    },
                    InternalEvent::Message { client_id, msg } => {
                        events.push(Event::Message { client_id, msg })
                    },
                }
            }
        }
        events
    }

    /// Stops the server, aka stops listening for new connections and drops existing connections
    pub async fn stop(&mut self) {
        let token = self.cancellation_token.take();
        if let Some(token) = token {
            token.cancel();
            self.cancellation_token = None;
            self.listener_addr = None;
            self.event_receiver = None;
            self.clients.clear();
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