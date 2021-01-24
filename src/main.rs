#![warn(clippy::all)]

mod convert;
mod filesystem;

use convert::publish;
use filesystem::{make_watcher, walk_dir};
use log::info;
use pretty_env_logger;
use std::path::Path;
use std::{env, error::Error, process::Command};
use warp::Filter;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    make_watcher(
        env::current_dir().unwrap().join("blog"),
        handle_path_change,
        true,
        1000,
    );

    make_watcher(
        env::current_dir().unwrap().join("templates"),
        handle_template_change,
        true,
        1000,
    );

    // dir already requires GET...
    let blog = warp::get()
        .and(warp::fs::dir("./blog/"));

    warp::serve(blog).run(([127, 0, 0, 1], 3030)).await;
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

        let html = publish(p).unwrap();
        print!("{:#?}", html);
        Command::new("explorer")
            .arg(html.as_os_str())
            .output()
            .unwrap();

        Ok(())
    }()
    .unwrap();
}
