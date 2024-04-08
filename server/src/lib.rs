use tokio_util::sync::CancellationToken;

pub enum Event {
    ClientConnected,
    ClientDisconnected
}

/// `Server` part of `netcode`
pub struct Server {
    /// The address which the server is currently listening to
    pub listener_addr: Option<std::net::SocketAddr>,
    /// Token to cancel listening
    pub cancellation_token: Option<tokio_util::sync::CancellationToken>
}
impl Default for Server {
    fn default() -> Self {
        Self {
            listener_addr: None,
            cancellation_token:None
        }
    }
}
impl Server {
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
                tokio::spawn(async move {
                    loop {
                        tokio::select! {
                            _ = cancellation_token.cancelled() => {
                                break;
                            }
                            stream = listener.accept() => {
                                if let Ok(stream) = stream {
                                    todo!();
                                }
                            }
                        };

                    }
                });
                return true;
            }
            Err(_) => {
                return false;
            }
        }
    }

    /// Collect events
    pub fn events(&mut self) -> Vec<Event>{
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
            let mut server = Server::default();
            assert_eq!(server.start(8080).await, true);
            assert_eq!(server.start(8080).await, false);
            server.stop().await;
            assert_eq!(server.start(8080).await, true);
        });
    }
}
