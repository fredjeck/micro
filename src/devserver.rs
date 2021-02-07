use std::{collections::HashMap, path::PathBuf, sync::Arc};

use futures::{Future, SinkExt, StreamExt};
use log::{debug, error, info};
use tokio::sync::{
    mpsc::{self, UnboundedReceiver, UnboundedSender},
    RwLock,
};
use uuid::Uuid;
use warp::{
    ws::{Message, WebSocket, Ws},
    Filter, Rejection, Reply,
};

type Result<T> = std::result::Result<T, Rejection>;
/// Helper type used to store WebSocket connected client
pub type Clients = Arc<RwLock<HashMap<String, Client>>>;

/// A WebSocket connected client
#[derive(Debug)]
pub struct Client {
    pub id: String,
    pub sender: Option<mpsc::UnboundedSender<Message>>,
}

#[derive(Debug, Clone)]
pub struct DevServer {
    clients: Clients,
}

/// A local web server which includes a WebSocket server
impl DevServer {

    pub fn new() -> DevServer {
        DevServer {
            clients: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn serve(&self, www_root: PathBuf, port: u16, open_browser: bool){
        if !www_root.exists() {
            panic!(format!(
                "Cannot serve '{:#?}': the path does not exist",
                www_root
            ));
        }

        info!("Starting development server");

        let connected = self.clients.clone();

        let uplink = warp::path("uplink")
            .and(warp::ws())
            .and(warp::any().map(move || connected.clone()))
            .and_then(handle_ws);

        let root = warp::get().and(warp::fs::dir(www_root));

        let server = warp::serve(root.or(uplink));
        server.run(([127, 0, 0, 1], port)).await
    }


    pub fn clients(&self) -> Clients {
        self.clients.clone()
    }
}

pub async fn handle_ws(ws: Ws, clients: Clients) -> Result<impl Reply> {
    Ok(ws.on_upgrade(move |socket| con_client_connected(socket, clients)))
}

pub async fn con_client_connected(ws: WebSocket, clients: Clients) {
    info!("Live preview instance connected");

    let (mut client_ws_sender, mut client_ws_rcv) = ws.split();
    let (client_sender, mut client_rcv): (
        UnboundedSender<Message>,
        UnboundedReceiver<Message>,
    ) = tokio::sync::mpsc::unbounded_channel();
    let id = Uuid::new_v4().to_string();

    tokio::task::spawn(async move {
        while let Some(result) = client_rcv.recv().await {
            let _ = client_ws_sender.send(result).await;
        }
    });

    let client = Client {
        id: id.clone(),
        sender: Some(client_sender),
    };

    clients.write().await.insert(client.id.clone(), client);

    info!("Live preview instance '{}' is now registered", &id);

    while let Some(result) = client_ws_rcv.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                error!("Error receiving ws message for id: {}): {}", &id, e);
                break;
            }
        };
        client_msg(&id, msg, &clients).await;
    }

    clients.write().await.remove(&id);
    info!("Live preview instance '{}' is now disconnected", &id);
}

async fn client_msg(id: &str, msg: Message, clients: &Clients) {
    let mut locked = clients.write().await;
    if let Some(v) = locked.get_mut(id) {
        if let Some(sender) = &v.sender {
            let _ = sender.send(Message::text("Micro live preview"));
        }
    }
}

pub async fn notify_clients(clients: &Clients,  path: String) {
    clients.read().await.iter().for_each(|(_, client)| {
        debug!("Notifiying client '{}' for '{}'", client.id, path);
        if let Some(sender) = &client.sender {
            //let _ = sender.send(Message::text(path.clone()));
            let _ = sender.send(Message::text(path.clone()));
        }
    });
}
