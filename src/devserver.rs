use std::{collections::HashMap, convert::Infallible, path::PathBuf, sync::Arc};

use futures::{FutureExt, StreamExt};
use log::info;
use tokio::sync::{mpsc, RwLock};
use warp::{
    ws::{Message, WebSocket},
    Filter, Rejection, Reply,
};

// https://github.com/zupzup/warp-websockets-example/blob/main/src/main.rs

type Result<T> = std::result::Result<T, Rejection>;

#[derive(Debug, Clone)]
pub struct Client {
    pub user_id: usize,
    pub topics: Vec<String>,
    pub sender: Option<mpsc::UnboundedSender<std::result::Result<Message, warp::Error>>>,
}

type Clients = Arc<RwLock<HashMap<String, Client>>>;

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
            //.and(warp::path::param())
            .and(warp::any().map(move || self.clients.clone()))
            .and_then(ws_handler);

        // let uplink = warp::path("uplink")
        // .and(warp::ws())
        // .and_then(wh);

        //let root = warp::get().and(warp::fs::dir(self.www_root.to_path_buf()));

        // let routes = uplink.or(root);

        let server = warp::serve(uplink);
        server.run(([127, 0, 0, 1], self.port)).await;
    }

    //  let heartbeat = warp::path("heartbeat")
    //         .and(warp::ws())
    //         .map(|ws: warp::ws::Ws| {
    //             // And then our closure will be called when it completes...
    //             ws.on_upgrade(|websocket| {
    //                 // Just echo all messages back...
    //                 let (tx, rx) = websocket.split();
    //                 rx.forward(tx).map(|result| {
    //                     if let Err(e) = result {
    //                         eprintln!("websocket error: {:?}", e);
    //                     }
    //                 })
    //             })
    //         });
}

pub async fn wh(ws: warp::ws::Ws) -> Result<impl Reply> {
    info!("in handler");
    Ok(ws.on_upgrade(|websocket| {
        // Just echo all messages back...
        let (tx, rx) = websocket.split();
        rx.forward(tx).map(|result| {
            if let Err(e) = result {
                eprintln!("websocket error: {:?}", e);
            }
        })
    }))
}

async fn client_connected(socket: WebSocket) {
    let (mut tx, _) = socket.split();
    info!("Client connected");
    // self.clients.push(tx);

    // tx.send(Message::text("Hello there")).await.unwrap();
}

fn with_clients(clients: Clients) -> impl Filter<Extract = (Clients,), Error = Infallible> + Clone {
    warp::any().map(move || clients.clone())
}

async fn register_client(id: String, user_id: usize, clients: Clients) {
    clients.write().await.insert(
        id,
        Client {
            user_id,
            topics: vec![String::from("cats")],
            sender: None,
        },
    );
}

pub async fn ws_handler(ws: warp::ws::Ws, clients: Clients) -> Result<impl Reply> {
    info!("in handler");
    register_client("a".to_string(), 23, clients.clone()).await;
    let client = clients.read().await.get("a").cloned();
    match client {
        Some(c) => Ok(ws.on_upgrade(move |socket| client_connection(socket, "a".to_string(), clients, c))),
        None => Err(warp::reject::not_found()),
    }
}

pub async fn client_connection(ws: WebSocket, id: String, clients: Clients, mut client: Client) {
    let (client_ws_sender, mut client_ws_rcv) = ws.split();
    let (client_sender, client_rcv) = mpsc::unbounded_channel();

    // tokio::task::spawn(client_rcv.forward(client_ws_sender).map(|result| {
    //     if let Err(e) = result {
    //         eprintln!("error sending websocket msg: {}", e);
    //     }
    // }));

    client.sender = Some(client_sender);
    clients.write().await.insert(id.clone(), client);

    println!("{} connected", id);

    while let Some(result) = client_ws_rcv.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("error receiving ws message for id: {}): {}", id.clone(), e);
                break;
            }
        };
        client_msg(&id, msg, &clients).await;
    }

    clients.write().await.remove(&id);
    println!("{} disconnected", id);
}

async fn client_msg(id: &str, msg: Message, clients: &Clients) {
    println!("received message from {}: {:?}", id, msg);
    let message = match msg.to_str() {
        Ok(v) => v,
        Err(_) => return,
    };

    if message == "ping" || message == "ping\n" {
        return;
    }
}
