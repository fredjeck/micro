#![warn(clippy::all)]

mod utils;
mod convert;

use std::error::Error;
use std::path::Path;
use convert::{ publish};
use simple_logger::SimpleLogger;
use log::{info};
use utils::make_watcher;

fn main() -> Result<(), Box<dyn Error>>  {
    SimpleLogger::new().init()?;


    make_watcher(Path::new("G:\\Workspaces\\Rust\\micro\\blog\\2020"), handle_path_change, true, 1000).join().unwrap();

    Ok(())
}

fn handle_path_change(p: &Path) {
    info!("Processing {}", p.to_str().unwrap());
    if p.extension().unwrap().to_str().unwrap() != "md" {return};

    publish(p).unwrap();
}
