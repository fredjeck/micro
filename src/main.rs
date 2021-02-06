#![warn(clippy::all)]

mod convert;
mod devserver;
mod filesystem;

use clap::{App, Arg};
use convert::publish;
use devserver::{Clients, DevServer};
use filesystem::{make_fs_watcher, walk_dir};
use log::info;
use pretty_env_logger;
use tokio::{sync::mpsc::{Receiver, Sender, channel}, task};
use std::{path::{Path, PathBuf}};
use std::{env, error::Error, process::Command};
use futures::future::lazy;

static mut SITE_ROOT: Option<PathBuf> = None;
static mut TEMPLATES_ROOT: Option<PathBuf> = None;
static mut CLIENTS: Option<Clients> = None;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    unsafe {
        let cwd = env::current_dir().unwrap();
        SITE_ROOT = Some(cwd.join("blog"));
        TEMPLATES_ROOT = Some(cwd.join("templates"));
    }

    let matches = App::new("micro")
    .author("FredJeck")
    .about("A super simple static website generator")
    .arg(Arg::new("SOURCE")
        .short('s')
        .long("src")
        .about("Path to the directory where the markdown source files are stored")
        .default_value(env::current_dir().unwrap().join("blog").to_str().unwrap())
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

    let src = Path::new(matches.value_of("SOURCE").unwrap()).to_path_buf();
    let src2 = Path::new(matches.value_of("SOURCE").unwrap()).to_path_buf();
    let templates = Path::new(matches.value_of("TEMPLATES").unwrap()).to_path_buf();

    if !matches.is_present("DEV") {
        return;
    }
    info!("Starting development server");
    let obj = Box::leak(Box::new(DevServer::new(src2, 4200, true)));
    unsafe {
        CLIENTS = Some(obj.clients());
    }

    let (mut sender, mut receiver): (Sender<String>, Receiver<String>)=channel(100);

    tokio::task::spawn(async move{
        loop{
            let p= match receiver.recv().await {
                Some(a) => a,
                None => return
            };
            handle_path_change(Path::new(&p));
        
        }
    });

    make_fs_watcher(src, sender, true, 1000);
    //make_fs_watcher(templates, handle_template_change, true, 1000);

    obj.start().await;
}

fn handle_template_change(p: &Path) {
    || -> Result<(), Box<dyn Error>> {
        info!(
            "The template {} changed, re-publishing pages",
            p.to_str().unwrap()
        );
        if p.extension().unwrap().to_str().unwrap() != "html" {
            return Ok(());
        };
        // TODO make this a little bit cleaner and only republish the ones using the template which changed
        walk_dir(
            env::current_dir()?.as_path(),
            "md",
            handle_path_change,
            true,
        )?;

        Ok(())
    }()
    .unwrap();
}
fn handle_path_change(p: &Path) {
    || -> Result<(), Box<dyn Error>> {
        info!("Processing {}", p.to_str().unwrap());
        if p.extension().unwrap().to_str().unwrap() != "md" {
            return Ok(());
        };

        let html = publish(p)?;
        print!("{:#?}", html);

        Command::new("explorer")
            .arg(html.as_os_str())
            .output()
            .unwrap();

        Ok(())
    }()
    .unwrap();
}
