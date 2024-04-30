use std::marker::PhantomData;
use common::{Format, Msg};
use ewebsock::{WsReceiver, WsSender};

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
    /// Messaging format used
    format:Format,
    /// Address to connect to
    url:String,

    /// Sender used to send messages to the server
    ws_sender:Option<WsSender>,

    /// Receiver used to receive events from the background task
    ws_receiver:Option<WsReceiver>,

    /// The state of the connection
    state:State,

    phantom:PhantomData<T>
}

impl<T:Msg> Default for Client<T> {
    fn default() -> Self {
        Self { url:Default::default(), ws_sender:None, ws_receiver:None, state:State::Disconnected, phantom:Default::default(), format:Format::Bincode }
    }
}

impl<T:Msg> Client<T> {
    /// Connect to the websocket server
    /// 
    /// Will try to connect forever and will re-connect in case of disconnections. 
    /*pub async fn connect(&mut self, url:impl Into<String>) {
        self.disconnect().await;
        let url:String = url.into();
        self.url.clone_from(&url);
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
    }*/

    pub fn with_format(mut self, format:Format) -> Self {
        self.format = format;
        self
    }

    /// Connect to the websocket server
    /// 
    /// Will try to connect forever and will re-connect in case of disconnections. 
    pub fn connect(&mut self, url:impl Into<String>) {
        self.disconnect();
        let url:String = url.into();
        self.url.clone_from(&url);
    }

    /// Connect to the websocket server
    /// 
    /// Will try to connect forever and will re-connect in case of disconnections. 
    pub fn disconnect(&mut self) {
        self.ws_receiver = None;
        self.ws_sender = None;
        self.url = String::default();
        //self.state = State::Disconnected;
    }

   /* pub async fn disconnect(&mut self) {
        if let Some(join_handle) = self.join_handle.take() {
            join_handle.abort();
            let _ = join_handle.await;
            self.join_handle = None;
            self.ws_sender = None;
            self.event_receiver = None;
        }
    }*/

    /// Sends a message to the server
    /// 
    /// Returns `true` if the message was sent (but not necessarly received by the server)
    pub fn send(&mut self, msg:T) -> bool {
        if self.state != State::Connected {
            return false;
        }

        if let Some(ws_sender) = &mut self.ws_sender {
            match self.format {
                Format::Bincode => {
                    match bincode::serialize(&msg) {
                        Ok(msg) => {
                            ws_sender.send(ewebsock::WsMessage::Binary(msg));
                            return true;
                        },
                        Err(_) => {
                            return false;
                        },
                    }
                },
                Format::Json => {
                    match serde_json::to_string(&msg) {
                        Ok(json) => {
                            ws_sender.send(ewebsock::WsMessage::Text(json));
                        },
                        Err(_) => {
                            return false;
                        },
                    }
                },
            }
            
        }

        false
    }

    /// Collect and process events
    /// 
    /// Returns the processed events which can be further processed by the calling application 
    pub fn poll(&mut self) -> Vec<Event<T>> {
        let mut events = Vec::with_capacity(32);

        if self.url.is_empty() {
            if self.state == State::Connected {
                self.state = State::Disconnected;
                events.push(Event::Disconnected);
            }
            return events;
        }

        if self.ws_sender.is_none() {
            let socket = ewebsock::connect(&self.url, ewebsock::Options {
                ..Default::default()
            });
            if let Ok((ws_sender, ws_receiver)) = socket {
                self.ws_sender = Some(ws_sender);
                self.ws_receiver = Some(ws_receiver);
            }
            events.push(Event::Connecting);
        }

        let mut recreate_socket = false;

        if let Some(ws_receiver) = &mut self.ws_receiver {
            while let Some(msg) = ws_receiver.try_recv() {
                match msg {
                    ewebsock::WsEvent::Opened => {
                        events.push(Event::Connected);
                        self.state = State::Connected;
                    },
                    ewebsock::WsEvent::Message(msg) => match msg {
                        ewebsock::WsMessage::Binary(msg) => {
                            if let Format::Bincode = self.format {
                                let msg = bincode::deserialize(&msg);
                                match msg {
                                    Ok(msg) => {
                                        events.push(Event::Message(msg));
                                    },
                                    Err(_) => {
                                        recreate_socket = true;
                                    },
                                };
                            }
                        },
                        ewebsock::WsMessage::Text(msg) => {
                            if let Format::Json = self.format {
                                let msg = serde_json::from_str(&msg);
                                match msg {
                                    Ok(msg) => {
                                        events.push(Event::Message(msg));
                                    },
                                    Err(_) => {
                                        recreate_socket = true;
                                    },
                                };
                            }
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
        }

        if recreate_socket {
            self.ws_sender = None;
            self.ws_receiver = None;
            if self.state == State::Connected {
                events.push(Event::Disconnected);
            }

            self.state = State::Disconnected;
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