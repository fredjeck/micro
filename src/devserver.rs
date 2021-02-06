use std::{collections::HashMap, convert::Infallible, path::PathBuf, sync::Arc};

use futures::{FutureExt, SinkExt, StreamExt};
use log::{debug, error, info};
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;
use warp::{Filter, Rejection, Reply, ws::{Message, WebSocket, Ws}};

// https://github.com/zupzup/warp-websockets-example/blob/main/src/main.rs

type Result<T> = std::result::Result<T, Rejection>;

#[derive(Debug, Clone)]
pub struct Client {
    pub id: String,
    pub sender: Option<mpsc::UnboundedSender<std::result::Result<Message, warp::Error>>>,
}

pub type Clients = Arc<RwLock<HashMap<String, Client>>>;

#[derive(Default)]
pub struct DevServer {
    port: u16,
    open_browser: bool,
    www_root: PathBuf,
    clients: Clients,
}

impl DevServer {
    pub fn new(www_root: PathBuf, port: u16, open_browser: bool) -> DevServer {
        if !www_root.exists() {
            panic!(format!(
                "Cannot serve '{:#?}': the path does not exist",
                www_root
            ));
        }

        DevServer {
            port: port,
            open_browser: open_browser,
            www_root: www_root,
            clients: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn start(&'static self) {
        info!("Starting development server");

        let uplink = warp::path("uplink")
            .and(warp::ws())
            .and(warp::any().map(move || self.clients.clone()))
            .and_then(handle_ws);

        let root = warp::get().and(warp::fs::dir(self.www_root.to_path_buf()));

        let server = warp::serve(uplink.or(root));
        server.run(([127, 0, 0, 1], self.port)).await;
    }

    pub async fn notify_clients(&self, path: String){
        self.clients.read().await.iter().for_each(|(_, client)|{
            if let Some(sender) = &client.sender {
                let _ = sender.send(Ok(Message::text(path.clone())));
            }
        });
    }

    pub fn clients(&self) -> Clients {
        self.clients.clone()
    }
}

pub async fn handle_ws(ws: Ws, clients: Clients) -> Result<impl Reply> {
    Ok(ws.on_upgrade(move |socket| con_client_connected(socket,  clients)))
}

pub async fn con_client_connected(ws: WebSocket, clients: Clients) {
    info!("Live preview instance connected");

    let (mut client_ws_sender, mut client_ws_rcv) = ws.split();
    let (client_sender, _) = mpsc::unbounded_channel();
    let id = Uuid::new_v4().to_string();

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
        debug!("Message received from {}: {:?}", &id, msg);
        client_ws_sender.send(Message::text("Micro::live preview server")).await.unwrap_or_default();
    }

    clients.write().await.remove(&id);
    info!("Live preview instance '{}' is now disconnected", &id);
}


