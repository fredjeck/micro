#![warn(clippy::all)]

mod convert;
mod filesystem;

use convert::publish;
use filesystem::make_watcher;
use log::info;
use simple_logger::SimpleLogger;
use std::path::Path;
use std::{env, error::Error, process::Command};

fn main() -> Result<(), Box<dyn Error>> {
    SimpleLogger::new().init()?;

    make_watcher(env::current_dir()?, handle_path_change, true, 1000)
        .join()
        .unwrap();

        make_watcher(env::current_dir()?.join("templates"), handle_template_change, true, 1000)
        .join()
        .unwrap();

    Ok(())
}

fn handle_template_change(p: &Path) {
    || -> Result<(), Box<dyn Error>> {
        info!("Processing {}", p.to_str().unwrap());
        if p.extension().unwrap().to_str().unwrap() != "html" {
            return Ok(());
        };

        

        Ok(())
    }().unwrap();
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
    }().unwrap();
}
