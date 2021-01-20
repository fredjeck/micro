pub mod metadata;
pub mod template;

use metadata::Metadata;
use pulldown_cmark::{html, Options, Parser};
use simple_error::bail;
use std::{fs::File, io::{prelude::*, BufWriter}, path::{Path, PathBuf}, str};
use std::{error, ffi::OsStr};

use template::{load_from, merge_template};

pub fn publish(source: &Path) -> Result<PathBuf, Box<dyn error::Error>> {
    if source.extension() != Some(OsStr::new("md")) {
        bail!("For now only markdown files are supported");
    }

    let mut markdown = Vec::new();

    let mut source_file = match File::open(source) {
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
    let metadata = match Metadata::extract(&mut markdown_content) {
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
    load_from(&metadata.layout, &mut template)?;

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
