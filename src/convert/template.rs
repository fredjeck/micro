use std::{env, error, fs::File, io::Read};

use log::{trace};
use simple_error::bail;
use crate::convert::Metadata;

pub fn load_from(name: &str, buffer: &mut Vec<u8>) -> Result<usize, Box<dyn error::Error>> {
    let cwd = env::current_dir()?.join("templates");

    let template_path = cwd.join(name).with_extension("html");
    trace!("Loading template from {:#?}", &template_path);

    let mut template_file = match File::open(&template_path) {
        Ok(handle) => handle,
        Err(error) => bail!(
            "An error occured while trying to load the template {:#?} : {}",
            &template_path,
            error
        ),
    };

    let bytes = match template_file.read_to_end(buffer) {
        Ok(b) => b,
        Err(error) => bail!(
            "An error occured loading a template from {:#?} : {}",
            template_path,
            error
        ),
    };

    Ok(bytes)
}

pub fn merge_template(template: &str, metadata: &Metadata, html_content: &str) -> String {
    let document = template
        .replace("{{content}}", html_content)
        .replace("{{title}}", &metadata.title)
        .replace("{{description}}", &metadata.description)
        .replace("{{publication_status}}", &metadata.published.to_string());

    document
}