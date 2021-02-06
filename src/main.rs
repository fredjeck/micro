mod watcher;
mod devserver;

use std::path::PathBuf;

use devserver::DevServer;
use log::info;
use tokio::{join, sync::mpsc::{Receiver, Sender}};

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let (sender, mut receiver): (Sender<String>, Receiver<String>) =
        tokio::sync::mpsc::channel(100);

    let root_path = PathBuf::from("G:\\Workspaces\\Rust\\micro\\wwwroot\\");
    let root_watcher = watcher::make_fs_watcher(root_path.clone(), sender.clone(), true, 1000);

    let templates_path = PathBuf::from("G:\\Workspaces\\Rust\\micro\\templates\\");
    let templates_watcher = watcher::make_fs_watcher(templates_path, sender, true, 1000);
 

    let server = DevServer::new(root_path, 4200, true);
    let cloned = server.clone(); // Bit odd, to be replaced with a geneator function or access to clients

    let consumer = tokio::task::spawn(async move {
        loop {
            let message = receiver.recv().await;
            if let Some(text) = message {
                info!("File {} changed", text);
                cloned.notify_clients(text).await;
            }
        }
    });


    let server_task = server.start();
    let (_, _, _, _) = join!(templates_watcher, root_watcher, consumer, server_task);
}
