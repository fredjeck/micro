#![warn(clippy::all)]

mod convert;
mod utils;

use convert::publish;
use log::info;
use simple_logger::SimpleLogger;
use std::error::Error;
use std::path::Path;
use utils::make_watcher;

fn main() -> Result<(), Box<dyn Error>> {
    SimpleLogger::new().init()?;

    make_watcher(
        Path::new("G:\\Workspaces\\Rust\\micro\\blog\\2020"),
        handle_path_change,
        true,
        1000,
    )
    .join()
    .unwrap();

    Ok(())
}

fn handle_path_change(p: &Path) {
    info!("Processing {}", p.to_str().unwrap());
    if p.extension().unwrap().to_str().unwrap() != "md" {
        return;
    };

    publish(p).unwrap();
}
