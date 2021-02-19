use std::{env, error, fs::File, io::Read, path::{Path, PathBuf}};

use log::{trace, warn};
use simple_error::bail;
use crate::convert::{MarkdownMetaData, metadata::Layout};

/// Loads an HTML template with the given name (without file extension) and returns its contents
pub fn load_template(name: &str, templates_root: Option<PathBuf>, buffer: &mut Vec<u8>) -> Result<usize, Box<dyn error::Error+Sync+Send>> {
   
    let cwd = match templates_root {
        Some(path) => path, 
        None => env::current_dir()?.join("templates")
    };

    let template_path = cwd.join(name).with_extension("html");
    trace!("Loading template {} from {:#?}", &name, &template_path);

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

/// Merges a template with the provided metadata
pub fn merge_template(template: &str, metadata: &MarkdownMetaData, html_content: &str) -> String {
    let document = template
        .replace("{{content}}", html_content)
        .replace("{{title}}", &metadata.title.as_ref().unwrap_or(&"".to_string()))
        .replace("{{description}}", &metadata.description.as_ref().unwrap_or(&"".to_string()))
        .replace("{{publication_status}}", &metadata.published.to_string());

    document
}

pub fn find_usage(root_path: &Path, layout: &Layout, matches: &mut Vec<PathBuf>){

    let directory = match std::fs::read_dir(&root_path) { 
        Ok(d) => d,
        Err(e) => {
            warn!("Unable to read '{:#?}' content, the directory will be skipped : {}", &root_path, e);
            return ();
        }
    };

    for entry in directory {
        let item_path = match entry {
            Ok(p) => p.path(),
            Err(e) => {
                warn!(
                    "An error occured while iterating through '{:#?}', the faulty item will be skipped : {}",
                    &root_path, e
                );
                continue;
            }
        };

        let metadata = match std::fs::metadata(&item_path) {
            Ok(md) => md,
            Err(err) => {
                warn!("Unable to stat '{:#?}', this item will be skipped\n{}", &item_path, err);
                continue;
            }
        };

        if metadata.is_file() {
            if let Some(metadata) = MarkdownMetaData::from_file(&item_path){
                if metadata.layout == *layout {
                    matches.push(item_path);
                }
            }
        } else  {
            find_usage(&item_path, layout, matches);
        }
    }
}
