#![feature(noop_waker)]

use std::{task::Waker, time::Duration};
use common::Message;
use tokio::sync::mpsc::{Receiver, Sender};

#[derive(Debug)]
pub enum Event<T> {
    Connecting,
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
    pub async fn connect(&mut self, url:impl Into<String>) {
        self.disconnect().await;
        let url:String = url.into();
        self.url = url.clone();
        let (sender, mut outer_receiver) = tokio::sync::mpsc::channel(1024) as (Sender<T>, Receiver<T>);
        self.ws_sender = Some(sender);
        let (event_sender, event_receiver) = tokio::sync::mpsc::channel(1024) as (Sender<Event<T>>, Receiver<Event<T>>);
        self.event_receiver = Some(event_receiver);
        self.join_handle = Some(tokio::spawn(async move {
            let mut emit_disconnected = false;
            loop {
                let mut recreate_socket = false;
                let socket = ewebsock::connect(&url, ewebsock::Options {
                    ..Default::default()
                });
                if let Ok((mut ws_sender, ws_receiver)) = socket {
                    let _ = event_sender.send(Event::Connecting).await;
                    loop {
                        // send messages
                        while let Ok(msg) = outer_receiver.try_recv() {
                            if let Ok(bincoded) = bincode::serialize(&msg) {
                                ws_sender.send(ewebsock::WsMessage::Binary(bincoded));
                            }
                        }
                        
                        // recv messages
                        while let Some(msg) = ws_receiver.try_recv() {
                            match msg {
                                ewebsock::WsEvent::Opened => {
                                    let _ = event_sender.send(Event::Connected).await;
                                    emit_disconnected = true;
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
                                ewebsock::WsEvent::Error(_err) => {
                                    recreate_socket = true;
                                },
                                ewebsock::WsEvent::Closed => {
                                    recreate_socket = true;
                                },
                            }
                        }
                        if recreate_socket {
                            if emit_disconnected {
                                let _ = event_sender.send(Event::Disconnected).await;
                                emit_disconnected = false;
                            }
                            break;
                        } else {
                            tokio::time::sleep(Duration::from_millis(1)).await;
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

    /// Collect and process events
    /// 
    /// Returns the processed events which can be further processed by the calling application 
    pub fn poll(&mut self) -> Vec<Event<T>> {
        let mut events = Vec::with_capacity(32);
        let Some(event_receiver) = &mut self.event_receiver else { return events };
        let mut cx = std::task::Context::from_waker(&Waker::noop());
        while let core::task::Poll::Ready(Some(v)) = event_receiver.poll_recv(&mut cx) {
            match &v {
                Event::Connecting => {
                    self.state = State::Disconnected;
                },
                Event::Connected => {
                    self.state = State::Connected;
                },
                Event::Disconnected => {
                    self.state = State::Disconnected;
                },
                Event::Message(_) => {
                },
            }
            events.push(v);
        }

        events
    }

    /// State of the connection
    /// 
    /// Ensure `events()` has been called before obtaining the state of the connection
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
                for e in client.poll() {
                }

                if client.state == State::Connected {
                    client.send("hello from client".to_owned());
                }

                yield_now().await;
            }
        });
    }
}
