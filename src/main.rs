mod convert;
mod devserver;
mod watcher;

use std::{
    ffi::OsStr,
    path::{Path, PathBuf, MAIN_SEPARATOR},
};

use convert::{markdown_to_html};
use devserver::DevServer;
use log::info;
use tokio::{
    join,
    sync::mpsc::{Receiver, Sender},
};

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let root_path = PathBuf::from("G:\\Workspaces\\Rust\\micro\\wwwroot\\");
    let templates_path = PathBuf::from("G:\\Workspaces\\Rust\\micro\\templates\\");

    start(root_path, templates_path).await;
}

async fn start(root_path: PathBuf, templates_path: PathBuf) {
    let (sender, mut receiver): (Sender<String>, Receiver<String>) =
        tokio::sync::mpsc::channel(100);

    let root_watcher = watcher::make_fs_watcher(root_path.clone(), sender.clone(), true, 1000);
    let templates_watcher = watcher::make_fs_watcher(templates_path.clone(), sender, true, 1000);

    let server = DevServer::new();
    let server_task = server.serve(root_path.clone(), 4200, true, None);
    let clients = server.clients();

    let consumer = tokio::task::spawn(async move {
        loop {
            let message = receiver.recv().await;
            if let Some(text) = message {
                info!("File {} changed", &text);
                let file_path = Path::new(&text);

                if file_path.starts_with(templates_path.to_path_buf()) {
                    // Will have to deal with template changes and maybe issue a reload command
                    continue;
                }

                let extension = match file_path.extension() {
                    Some(e) => e,
                    None => OsStr::new(""),
                };

                if extension == "md" {
                    if let Ok(html) = markdown_to_html(
                        file_path.to_path_buf(),
                        None,
                        templates_path.to_path_buf(),
                    ) {
                        if let Ok(p) = html.strip_prefix(&root_path) {
                            let str = p.to_str().unwrap();
                            devserver::send_message(&clients, str.replace(MAIN_SEPARATOR, "/"))
                                .await;
                        }
                    }
                }
            }
        }
    });

    let (_, _, _, _) = join!(templates_watcher, root_watcher, consumer, server_task);
}
