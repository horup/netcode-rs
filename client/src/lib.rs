#![feature(noop_waker)]

use std::{future::Ready, sync::Arc, task::Waker, time::Duration};
use common::Message;
use tokio::{sync::{mpsc::{Receiver, Sender}, Mutex}};
use ewebsock::{WsReceiver, WsSender};

pub enum Event<T> {
    Connected,
    Disconnected,
    Message(T)
}


#[derive(Clone, Copy, PartialEq, Eq)]
pub enum State {
    Connected,
    Disconnected
}

/// `Client` for `netcode`
/// The client will autoconnect to the server and try to keep connected
pub struct Client<T> {
    /// Address to connect to
    url:String,

    /// Background task polling the socket
    join_handle:Option<tokio::task::JoinHandle<()>>,

    /// Sender used to send messages to the server
    ws_sender:Option<tokio::sync::mpsc::Sender<T>>,

    /// Receiver used to receive events from the background task
    event_receiver:Option<tokio::sync::mpsc::Receiver<Event<T>>>,

    /// The state of the connection
    state:State
}

impl<T:Message> Default for Client<T> {
    fn default() -> Self {
        Self { url:Default::default(), join_handle:None, ws_sender:None, event_receiver:None, state:State::Disconnected }
    }
}

impl<T:Message> Client<T> {
    /// Connect to the websocket server
    /// 
    /// Will try to connect forever and will re-connect in case of disconnections. 
    pub async fn connect(&mut self, addr:impl Into<String>) {
        let addr:String = addr.into();
        self.disconnect().await;
        {
            self.url = addr.clone();
        }

        let (sender, outer_receiver) = tokio::sync::mpsc::channel(1024) as (Sender<T>, Receiver<T>);
        self.ws_sender = Some(sender);
        let (event_sender, event_receiver) = tokio::sync::mpsc::channel(1024) as (Sender<Event<T>>, Receiver<Event<T>>);
        self.event_receiver = Some(event_receiver);
        self.join_handle = Some(tokio::spawn(async move {
            loop {
                let mut recreate_socket = false;
                let socket = ewebsock::connect(&addr, ewebsock::Options {
                    ..Default::default()
                });
                if let Ok((ws_sender, ws_receiver)) = socket {
                    loop {
                        while let Some(msg) = ws_receiver.try_recv() {

                            match msg {
                                ewebsock::WsEvent::Opened => {
                                    let _ = event_sender.send(Event::Connected).await;
                                },
                                ewebsock::WsEvent::Message(msg) => match msg {
                                    ewebsock::WsMessage::Binary(msg) => {
                                        let msg = bincode::deserialize(&msg);
                                        match msg {
                                            Ok(msg) => {
                                                let _ = event_sender.send(Event::Message(msg)).await;
                                            },
                                            Err(_) => {
                                                recreate_socket = true;
                                            },
                                        };
                                    },
                                    _ => {}
                                },
                                ewebsock::WsEvent::Error(err) => {
                                    recreate_socket = true;
                                },
                                ewebsock::WsEvent::Closed => {
                                    recreate_socket = true;
                                },
                            }
                        }
                        if recreate_socket {
                            let _ = event_sender.send(Event::Disconnected).await;
                            break;
                        }
                    }
                }
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }));
    }

    pub async fn disconnect(&mut self) {
        if let Some(join_handle) = self.join_handle.take() {
            join_handle.abort();
            let _ = join_handle.await;
            self.join_handle = None;
            self.ws_sender = None;
            self.event_receiver = None;
        }
    }

    /// Sends a message to the server
    /// 
    /// Returns `true` if the message was sent (but not necessarly received by the server)
    pub fn send(&mut self, msg:T) -> bool {
        if self.state != State::Connected {
            return false;
        }

        match &mut self.ws_sender {
            Some(sender) => {
                sender.try_send(msg).is_ok()
            },
            None => {
                false
            },
        }
    }

    /// Collect events 
    pub fn events(&mut self) -> Vec<Event<T>> {
        let mut events = Vec::with_capacity(32);
        let Some(event_receiver) = &mut self.event_receiver else { return events };
        let mut cx = std::task::Context::from_waker(&Waker::noop());
        while let core::task::Poll::Ready(Some(v)) = event_receiver.poll_recv(&mut cx) {
            events.push(v);
        }

        events
    }

    /// State of the connection
    pub fn state(&self) -> State {
        self.state
    }
}

#[cfg(test)]
mod tests {
    use tokio::task::yield_now;

    use super::*;

    #[test]
    fn test() {
        tokio_test::block_on(async {
            let mut client = Client::default() as Client<String>;
            client.connect("wss://echo.websocket.org/").await;
            client.connect("wss://echo.websocket.org/").await;
            assert_eq!(client.send("hello world".to_owned()), false);
            loop {
                for e in client.events() {
                    dbg!("he");
                }

                yield_now().await;
            }
        });
    }
}
