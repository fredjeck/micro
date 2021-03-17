use futures::{SinkExt, StreamExt};
use log::{debug, error, info};
use std::{collections::HashMap, path::PathBuf, sync::Arc, time::Duration};
use tokio::{
    sync::{
        mpsc::{self, UnboundedReceiver, UnboundedSender},
        RwLock,
    },
    time::sleep,
};
use uuid::Uuid;
use warp::{Filter, Rejection, Reply, ws::{Message, WebSocket, Ws}};

type Result<T> = std::result::Result<T, Rejection>;
/// Helper type used to store WebSocket connected client
pub type Clients = Arc<RwLock<HashMap<String, Client>>>;

/// A client connected via WebSocket
#[derive(Debug)]
pub struct Client {
    pub id: String,
    pub sender: Option<mpsc::UnboundedSender<Message>>,
}

#[derive(Debug, Clone)]
pub struct DevServer {
    clients: Clients,
}

/// A local development web server which includes WebSocket support
impl DevServer {
    pub fn new() -> DevServer {
        DevServer {
            clients: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Starts the local development web server, serving the content of __www_root__.
    /// If __open_in_browser__ is set to true, the system's default browser will be openened at the specified __root_url__
    pub async fn serve(
        &self,
        www_root: PathBuf,
        port: u16,
        open_in_browser: bool,
        root_url: Option<String>,
    ) {
        if !www_root.exists() {
            panic!(format!(
                "Cannot serve content from '{:#?}': the path does not exist",
                www_root
            ));
        }

        info!("Starting development server");
        let connexions = self.clients.clone();

        let uplink = warp::path("uplink")
            .and(warp::ws())
            .and(warp::any().map(move || connexions.clone()))
            .and_then(register_ws_handler);

        let root = warp::get().and(warp::fs::dir(www_root));
        let filter = root.or(uplink);
        let server = warp::serve(filter);

        let path = match root_url {
            Some(u) => u,
            None => "/".to_string(),
        };

        if open_in_browser {
            tokio::task::spawn(async move {
                // Lets delay the browser's opening
                sleep(Duration::from_millis(5000)).await;
                let url = format!("http://localhost:{}{}", &port, path);
                webbrowser::open(&url).unwrap();
            });
        }

        server.run(([127, 0, 0, 1], port)).await
    }

    /// Returns the list clients connected via WebSocket
    pub fn clients(&self) -> Clients {
        self.clients.clone()
    }
}

/// Registers the WebSocket connection handler
async fn register_ws_handler(ws: Ws, clients: Clients) -> Result<impl Reply> {
    Ok(ws.on_upgrade(move |socket| con_client_connected(socket, clients)))
}

/// Called whenever a new client connects via websockets
async fn con_client_connected(ws: WebSocket, clients: Clients) {
    debug!("New WS client connection request received");

    let (mut client_ws_sender, mut client_ws_rcv) = ws.split();
    let (client_sender, mut client_rcv): (UnboundedSender<Message>, UnboundedReceiver<Message>) =
        tokio::sync::mpsc::unbounded_channel();
    let id = Uuid::new_v4().to_string();

    // This should be coverd by a forward method, wich existed at some point - need to investigate
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

    info!("Live preview instance '{}' now registered", &id);

    while let Some(result) = client_ws_rcv.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                error!("Error receiving message from client '{}': {}", &id, e);
                break;
            }
        };
        client_msg(&id, msg, &clients).await;
    }

    clients.write().await.remove(&id);
    info!("Live preview instance '{}' is now disconnected", &id);
}

/// Triggered whenever a message is received via WebSocket
async fn client_msg(id: &str, msg: Message, clients: &Clients) {
    let mut locked = clients.write().await;
    if let Some(v) = locked.get_mut(id) {
        debug!("Message received from '{}': {:#?}", v.id, msg);
        if let Some(sender) = &v.sender {
            let _ = sender.send(Message::text("Pong !"));
        }
    }
}

/// Sends a message to the provided list of WebSocket connect clients
pub async fn send_message(clients: &Clients, message: String) {
    clients.read().await.iter().for_each(|(_, client)| {
        debug!("Notifiying client '{}' for '{}'", client.id, message);
        if let Some(sender) = &client.sender {
            //let _ = sender.send(Message::text(path.clone()));
            let _ = sender.send(Message::text(message.clone()));
        }
    });
}
