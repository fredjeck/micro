mod convert;
mod devserver;
mod watcher;

use std::{
    env,
    ffi::OsStr,
    path::{Path, PathBuf, MAIN_SEPARATOR},
};

use clap::{App, Arg};
use convert::markdown_to_html;
use devserver::DevServer;
use log::info;
use tokio::{
    join,
    sync::mpsc::{Receiver, Sender},
};

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let matches = App::new("micro")
    .author("FredJeck")
    .about("A super simple static website generator")
    .arg(Arg::new("SOURCE")
        .short('s')
        .long("src")
        .about("Path to the directory where the markdown source files are stored")
        .default_value(env::current_dir().unwrap().join("wwwroot").to_str().unwrap())
        .validator(|p|{
            if !Path::new(p).exists(){
                return Err(format!("Unable to find '{}'. Please make sure the 'src' argument points to a valid directory", p));
            }
            Ok(())
        }))
    .arg(Arg::new("DEV")
        .short('d')
        .long("dev")
        .takes_value(false)
        .about("Runs micro in development mode spawning a child process monitoring for pages and template changes and automatically publishing them. A local webserver will also be started and will serve the edited resources"))
    .arg(Arg::new("TEMPLATES")
        .short('t')
        .long("templates")
        .about("Path to the directory where the pages templates are located")
        .default_value(env::current_dir().unwrap().join("templates").to_str().unwrap())
        .validator(|p|{
            if !Path::new(p).exists(){
                return Err(format!("Unable to find '{}'. Please make sure the 'templates' argument points to a valid directory", p));
            }
            Ok(())
        }))
    .get_matches();

    let root_path = match matches.value_of("SOURCE") {
        Some(s) => PathBuf::from(s),
        None => panic!("Source path cannot be found"),
    };
    let templates_path = match matches.value_of("TEMPLATES") {
        Some(s) => PathBuf::from(s),
        None => panic!("Templates path cannot be found"),
    };

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

                let extension = match file_path.extension() {
                    Some(e) => e,
                    None => OsStr::new(""),
                };

                if file_path.starts_with(templates_path.to_path_buf()) {
                    if extension == "html" {}
                    // Will have to deal with template changes and maybe issue a reload command
                    continue;
                } else if extension == "md" {
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
