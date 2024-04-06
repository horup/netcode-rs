use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;
use ewebsock::{WsReceiver, WsSender};
struct WebSocket {
    pub sender:WsSender,
    pub receiver:WsReceiver
}

#[derive(Default)]
pub struct SharedClient {
    connected:bool,
    url:Option<String>,
    socket:Option<Mutex<WebSocket>>,
}

/// `Client` for `netcode`
/// The client will autoconnect to the server and try to keep connected
pub struct Client {
    shared:Arc<Mutex<SharedClient>>,
    join_handle:Option<tokio::task::JoinHandle<()>>
}

impl Default for Client {
    fn default() -> Self {
        Self { shared:Arc::new(Mutex::new(SharedClient::default())), join_handle:None }
    }
}

impl Client {
    /// Connect to the websocket server
    /// 
    /// Will try to connect forever and will re-connect in case of disconnections. 
    pub async fn connect(&mut self, addr:impl Into<String>) {
        let addr:String = addr.into();
        self.disconnect().await;
        {
            let mut shared = self.shared.lock().await;
            shared.url = Some(addr.clone());
        }

        let shared = self.shared.clone();
        self.join_handle = Some(tokio::spawn(async move {
            loop {
                let mut recreate_socket = false;
                let socket = ewebsock::connect(&addr);
                if let Ok((sender, receiver)) = socket {
                    loop {
                        while let Some(msg) = receiver.try_recv() {
                            match msg {
                                ewebsock::WsEvent::Opened => {
                                },
                                ewebsock::WsEvent::Message(msg) => match msg {
                                    ewebsock::WsMessage::Binary(msg) => {

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
                            break;
                        }
                    }
                }
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }));
    }
    pub async fn disconnect(&mut self) {
        let mut shared = self.shared.lock().await;
        shared.socket = None;
        shared.url = None;
        shared.connected = false;
        if let Some(join_handle) = self.join_handle.take() {
            join_handle.abort();
            let _ = join_handle.await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        tokio_test::block_on(async {
            let mut client = Client::default();
            client.connect("wss://echo.websocket.org/").await;
            client.connect("wss://echo.websocket.org/").await;

        });
    }
}
