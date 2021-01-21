#![warn(clippy::all)]

mod convert;
mod filesystem;

use convert::publish;
use filesystem::{make_watcher, walk_dir};
use log::info;
use simple_logger::SimpleLogger;
use std::path::Path;
use std::{env, error::Error, process::Command};

fn main() -> Result<(), Box<dyn Error>> {
    SimpleLogger::new().init()?;

    make_watcher(env::current_dir()?, handle_path_change, true, 1000);

    make_watcher(
        env::current_dir()?.join("templates"),
        handle_template_change,
        true,
        1000,
    )
    .join()
    .unwrap();

    Ok(())
}

fn handle_template_change(p: &Path) {
    || -> Result<(), Box<dyn Error>> {
        info!("The template {} changed, re-publishing pages", p.to_str().unwrap());
        if p.extension().unwrap().to_str().unwrap() != "html" {
            return Ok(());
        };
        // TODO make this a little bit cleaner and only republish the ones using the template which changed
        walk_dir(env::current_dir()?.as_path(), "md", handle_path_change, true)?;

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
