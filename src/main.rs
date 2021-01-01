mod utils;

use std::fs::File;
use std::io::prelude::*;
use std::error::Error;
use std::path::Path;
use utils::make_watcher;
use simple_logger::SimpleLogger;
use pulldown_cmark::{html, Options, Parser};
use std::str;
use std::io::BufWriter;
use log::{info, trace};

fn main() -> Result<(), Box<dyn Error>>  {
    SimpleLogger::new().init()?;


    make_watcher(Path::new("G:\\Workspaces\\Rust\\micro\\blog\\2020"), handle_path_change, true, 1000).join().unwrap();

    Ok(())
}

fn handle_path_change(p: &Path) {
    info!("Processing {}", p.to_str().unwrap());
    trace!("Extension {}", p.extension().unwrap().to_str().unwrap());
    if p.extension().unwrap().to_str().unwrap() != "md" {return};

    let mut f = File::open(p).unwrap();

    let mut buffer = Vec::new();
    f.read_to_end(&mut buffer).unwrap();

    let parser = Parser::new_ext(str::from_utf8(&buffer).unwrap(), Options::all());

    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

    let f = File::create(p.with_extension("html")).unwrap(); 
    let mut writer = BufWriter::new(f);
    writer.write(html_output.as_bytes()).unwrap();
}
