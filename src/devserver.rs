use std::{collections::HashMap, path::PathBuf, sync::Arc};

use futures::{SinkExt, StreamExt};
use log::{debug, error, info};
use tokio::sync::{RwLock, mpsc::{self, UnboundedReceiver, UnboundedSender}};
use uuid::Uuid;
use warp::{
    ws::{Message, WebSocket, Ws},
    Filter, Rejection, Reply,
};

// https://github.com/zupzup/warp-websockets-example/blob/main/src/main.rs

type Result<T> = std::result::Result<T, Rejection>;

#[derive(Debug)]
pub struct Client {
    pub id: String,
    //pub sender: Option<SplitSink<WebSocket, Message>>,
    pub sender: Option<mpsc::UnboundedSender<std::result::Result<Message, warp::Error>>>,
}

pub type Clients = Arc<RwLock<HashMap<String, Client>>>;

#[derive(Debug, Clone)]
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

    pub async fn start(&self) {
        info!("Starting development server");

        let connected = self.clients.clone();

        let uplink = warp::path("uplink")
            .and(warp::ws())
            .and(warp::any().map(move || connected.clone()))
            .and_then(handle_ws);

        let root = warp::get().and(warp::fs::dir(self.www_root.clone()));

        let server = warp::serve(root.or(uplink));
        server.run(([127, 0, 0, 1], self.port.clone())).await
    }

    pub async fn notify_clients(&self, path: String) {
        self.clients.read().await.iter().for_each(|(_, client)| {
            debug!("Notifiying client '{}' for '{}'", client.id, path);
            if let Some(sender) = &client.sender {
                //let _ = sender.send(Message::text(path.clone()));
                let _ = sender.send(Ok(Message::text(path.clone())));
            }
        });
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
    let (client_sender, mut client_rcv) : (UnboundedSender<std::result::Result<Message, warp::Error>>, UnboundedReceiver<std::result::Result<Message, warp::Error>>)= tokio::sync::mpsc::unbounded_channel();
    let id = Uuid::new_v4().to_string();

   

     tokio::task::spawn(async move {
        while let Some(result) = client_rcv.recv().await {
            let _ = client_ws_sender.send(result.unwrap()).await;
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
            //let _ = sender.send(Message::text(path.clone()));
            let _ = sender.send(Ok(Message::text("Micro live preview")));
        }
    }
}
