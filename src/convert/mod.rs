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

use template::{merge_template};

pub async fn create_markdown_pipeline(
    mut receiver: Receiver<PathBuf>,
    templates_location: PathBuf,
) -> tokio::task::JoinHandle<()> {
    if !templates_location.exists() {
        panic!(format!("Cannot initialize markdown pipeline : templates location {:#?} leads to a non existing path", templates_location).as_str());
    }

    tokio::task::spawn(async move {
        loop {
            let source = match receiver.recv().await {
                Some(s) => s,
                None => continue,
            };

            debug!("Markdown pipeline : publishing {:#?}", &source);

            if source.extension() != Some(OsStr::new("md")) {
                debug!("{:#?} is not markdown...skipping", source);
                continue;
            }

            let mut markdown = Vec::new();
            let mut source_file = match File::open(&source) {
                Ok(handle) => handle,
                Err(error) => {
                    error!("The following error occurred while opening {:#?} : {:#?}, this file will be skipped", error, &source);
                    continue;
                }
            };
            match source_file.read_to_end(&mut markdown) {
                Ok(_) => {}
                Err(error) => {
                    error!("The following error occurred while reading {:#?} : {:#?}, this file will be skipped", error, &source);
                    continue;
                }
            };

            let mut markdown_content = match String::from_utf8(markdown) {
                Ok(m) => m,
                Err(error) => {
                    error!("UTF8 conversion error occured for {:#?} ...skipping : {:#?}, this file will be skipped", &source, error);
                    continue;
                }
            };

            let metadata = match MarkdownMetaData::extract(&mut markdown_content) {
                Some(meta) => meta,
                None => {
                    error!("Unable to extract metadata from {:#?}, this file will be skipped", &source);
                    continue;
                }
            };

            // TODO : improve that, and keep a reference instead of re-instanciating it every time
            let parser = Parser::new_ext( markdown_content.as_str(), Options::all());

            let mut html = String::new();
            html::push_html(&mut html, parser);

            let mut template = Vec::new();
            match template::load_template(&metadata.layout.to_string(), None,  &mut template){
                Ok(_) => {}
                Err(_) => {
                    error!("Unable to load template {:#?} for {:#?}, this file will be skipped", metadata.layout, &source);
                    continue;
                }
            };

            let content = str::from_utf8(&template).unwrap_or("");

            let document = merge_template(content, &metadata, &html);

            let html_file = match File::create(source.with_extension("html")) {
                Ok(handle) => handle,
                Err(_) => continue
            };

            let mut writer = BufWriter::new(html_file);

            match writer.write(document.as_bytes()){
                Ok(_) => {}
                Err(_) => {
                    error!("Unable to write the rendered HTML file for {:#?}", &source);
                    continue;
                }
            };
        }
    })
}

pub fn publish(source: PathBuf) -> Result<PathBuf, Box<dyn error::Error + Send + Sync>> {
    if source.extension() != Some(OsStr::new("md")) {
        bail!("For now only markdown files are supported");
    }

    let mut markdown = Vec::new();

    let mut source_file = match File::open(&source) {
        Ok(handle) => handle,
        Err(error) => bail!(
            "An error occured while accessing {:#?} for reading {}",
            source,
            error
        ),
    };

    match source_file.read_to_end(&mut markdown) {
        Ok(_) => {}
        Err(error) => bail!("An error occured reading {:#?} content {}", source, error),
    };

    let mut markdown_content = str::from_utf8(&markdown)?.to_string();
    let metadata = match MarkdownMetaData::extract(&mut markdown_content) {
        Some(meta) => meta,
        None => {
            bail!("Missing or incomplete metadata for file  {:#?} ", source)
        }
    };

    println!("Metadata {}", metadata);

    let parser = Parser::new_ext(markdown_content.as_str(), Options::all());

    let mut html = String::new();
    html::push_html(&mut html, parser);

    let mut template = Vec::new();
    template::load_template(&metadata.layout.to_string(), None,  &mut template)?;

    let content = str::from_utf8(&template)?;

    let document = merge_template(content, &metadata, &html);

    let html_file = match File::create(source.with_extension("html")) {
        Ok(handle) => handle,
        Err(error) => bail!(
            "An error occured while creating the destination HTML file for {:#?} : {}",
            source,
            error
        ),
    };

    let mut writer = BufWriter::new(html_file);

    writer.write(document.as_bytes())?;

    Ok(source.with_extension("html"))
}
