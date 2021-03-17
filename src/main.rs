mod convert;
mod devserver;
mod filesystem;
mod watcher;

use std::{
    env,
    error::Error,
    ffi::OsStr,
    path::{Path, PathBuf, MAIN_SEPARATOR},
};

use clap::{App, Arg};
use convert::{markdown_to_html, metadata, template};
use devserver::DevServer;
use filesystem::walk_dir;
use log::info;
use simple_error::bail;
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
    .subcommand(App::new("verify").about("Scans for out of date published files"))
    .subcommand(App::new("publish").about("Scans for out of date published files and republish them if needed"))
    .get_matches();

    let root_path = match matches.value_of("SOURCE") {
        Some(s) => PathBuf::from(s),
        None => panic!("Source path cannot be found"),
    };
    let templates_path = match matches.value_of("TEMPLATES") {
        Some(s) => PathBuf::from(s),
        None => panic!("Templates path cannot be found"),
    };

    if let Some(_) = matches.subcommand_matches("verify") {
        verify(root_path.clone(), templates_path.clone());
    }

    if let Some(_) = matches.subcommand_matches("publish") {
        verify(root_path.clone(), templates_path.clone());
    }

    if 1 == matches.occurrences_of("DEV") {
        start_dev_server(root_path, templates_path).await;
    }
}

async fn verify(root_path: PathBuf, templates_path: PathBuf) -> Result<(), Box<dyn Error>> {
    let templates_ts = match template::last_changed(&templates_path) {
        Ok(t) => t,
        Err(e) => bail!(e),
    };

    walk_dir(root_path, "md", true, &move |p: &Path| {
        let markdown = p.metadata().unwrap();
        let html = p.with_extension("html").metadata().unwrap();

        let mdchange = markdown.modified().unwrap();
        let htchange = html.modified().unwrap();

        let mut publish = false;
        if mdchange > htchange {
            publish = true;
        } else {
            // Check if the template changed
            let md = metadata::MarkdownMetaData::from_file(p);
            if let Some(metadata) = md {
                if let Some(tplchange) = templates_ts.get(&metadata.layout) {
                    publish = *tplchange > mdchange;
                }
            }
        }

        if publish {
            convert::markdown_to_html(p.to_owned(), None, templates_path.to_owned());
        }
    });

    Ok(())
}

async fn start_dev_server(root_path: PathBuf, templates_path: PathBuf) {
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
                    if extension == "html" {
                        let mut matches: Vec<PathBuf> = vec![];
                        let layout = convert::metadata::Layout::from(
                            file_path.file_name().unwrap().to_str().unwrap(),
                        );
                        convert::template::find_usage(&root_path, &layout, &mut matches);
                        for file in matches {
                            if let Ok(html) =
                                markdown_to_html(file, None, templates_path.to_path_buf())
                            {
                                if let Ok(p) = html.strip_prefix(&root_path) {
                                    let str = p.to_str().unwrap();
                                    devserver::send_message(
                                        &clients,
                                        str.replace(MAIN_SEPARATOR, "/"),
                                    )
                                    .await;
                                }
                            }
                        }
                    }
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
