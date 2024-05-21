//! example with a master server, some instances and some clients

use master_server::MasterServer;
use server::Server;
#[tokio::main]
async fn main() {
    let server = async {
        let mut server = Server::default();
        server.start(8080).await;
        let mut master_server:MasterServer<()> = MasterServer::new(server);
        
    };

    let server_handle = tokio::spawn(server);

    let _ = server_handle.await;
}