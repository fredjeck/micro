pub mod metadata;
pub mod template;

use core::panic;
use log::{debug, error};
use metadata::MarkdownMetaData;
use pulldown_cmark::{html, Options, Parser};
use simple_error::bail;
use std::{error, ffi::OsStr};
use std::{
    fs::File,
    io::{prelude::*, BufWriter},
    path::PathBuf,
    str,
};
use tokio::sync::mpsc::Receiver;

use template::merge_template;

// Unused for now, leave it here for documentation
pub async fn _create_markdown_pipeline(
    mut receiver: Receiver<PathBuf>,
    templates_location: PathBuf,
) -> tokio::task::JoinHandle<()> {
    if !templates_location.exists() {
        panic!(format!("Cannot initialize markdown pipeline : templates location {:#?} leads to a non existing path", &templates_location).as_str());
    }

    tokio::task::spawn(async move {
        loop {
            let source = match receiver.recv().await {
                Some(s) => s,
                None => continue,
            };

            match markdown_to_html(source,None, templates_location.clone()){
                Ok(_) => {}
                Err(e) => {
                    error!("{}", e);
                    continue;
                }
            }
        }
    })
}

/// Converts the source markdown file (which needs to have a .md file extension) to HTML using a layout template specified via Metadata 
/// If no destination is specified, the generated html file will be placed in the same path as the source file with the ".html" extension
pub fn markdown_to_html(source: PathBuf, destination: Option<PathBuf>, templates_location: PathBuf) 
-> Result<PathBuf, Box<dyn error::Error + Send + Sync>> 
{
    debug!("Publishing {:#?}", &source);

    if source.extension() != Some(OsStr::new("md")) {
        bail!("{:#?} is not markdown...skipping", source);
    }

    let mut markdown = Vec::new();
    let mut source_file = match File::open(&source) {
        Ok(handle) => handle,
        Err(error) => {
            bail!("The following error occurred while opening {:#?} : {:#?}", error, &source);
        }
    };
    match source_file.read_to_end(&mut markdown) {
        Ok(_) => {}
        Err(error) => {
            bail!("The following error occurred while reading {:#?} : {:#?}", error, &source);
        }
    };

    let mut markdown_content = match String::from_utf8(markdown) {
        Ok(m) => m,
        Err(error) => {
            bail!("UTF8 conversion error occured for {:#?} ...skipping : {:#?}", &source, error);
        }
    };

    let metadata = match MarkdownMetaData::extract(&mut markdown_content) {
        Some(meta) => meta,
        None => {
            bail!(
                "Unable to extract metadata from {:#?}",
                &source
            );
        }
    };

    // TODO : improve that, and keep a reference instead of re-instanciating it every time
    let parser = Parser::new_ext(markdown_content.as_str(), Options::all());

    let mut html = String::new();
    html::push_html(&mut html, parser);

    let mut template = Vec::new();
    match template::load_template(&metadata.layout.to_string(), Some(templates_location), &mut template) {
        Ok(_) => {}
        Err(_) => {
            bail!(
                "Unable to load template [{:#?}] for {:#?}",
                metadata.layout, &source
            );
        }
    };

    let content = str::from_utf8(&template).unwrap_or("");
 

    let target = match destination {
        Some(p) => p.with_extension("html"),
        None => source.with_extension("html")
    };

    let document = merge_template(content, &metadata, &html);

    let html_file = match File::create(&target) {
        Ok(handle) => handle,
        Err(e) => {
            bail!(
                "Unable to create the destination file to {:#?} : {:#?}",
                &target, e
            );
        }
    };

    let mut writer = BufWriter::new(html_file);

    match writer.write(document.as_bytes()) {
        Ok(_) => {}
        Err(e) => {
            bail!(
                "Unable to write the rendered file to {:#?} : {:#?}",
                &target, e
            );
        }
    };

    Ok(source.with_extension("html"))
}
