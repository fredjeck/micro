use std::{fmt::Display, fs::File, path::Path, io::{prelude::*}};

use chrono::{DateTime, Utc};
use log::{error, warn};
use regex::Regex;
use serde_yaml::Value;

/// Template to apply to a markdown file during its rendering.
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum Layout {
    Article,
    Index,
    Undefined
}

impl Display for Layout {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Layout::Article => "Article",
                Layout::Index => "Index",
                Layout::Undefined => "Undefined",
            }
        )
    }
}

impl From<&str> for Layout {
    fn from(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "index" => Self::Index,
            "article" => Self::Article,
            _ => return Self::Undefined,
        }
    }
}

/// Optional metadata segment to be used in markdown files.
pub struct MarkdownMetaData {
    pub layout: Layout,
    pub title: Option<String>,
    pub description: Option<String>,
    pub source: Option<String>,
    pub published: DateTime<Utc>,
}

impl Display for MarkdownMetaData {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            r#"Metadata{{Layout:"{:#?}", Title:"{:#?}", Description:"{:#?}", Published:"{}"}}"#,
            self.layout, self.title, self.description, self.published
        )
    }
}

impl MarkdownMetaData  {
    /// Extracts (and removes) the Meta Data from the markdown content.
    pub fn extract(document: &mut String) -> Option<MarkdownMetaData> {
        let re = Regex::new(r"(?s)---(.*?)---").unwrap();
        if !re.is_match(&document) {
            return None;
        }

        let metadata_text = match re.captures(&document){
            Some(m) => m[1].to_string(), 
            None => return None
        };

        let yaml: Value = match serde_yaml::from_str(&metadata_text) {
            Ok(v) => v,
            Err(_) => Value::default(),
        };

        let mdobj = MarkdownMetaData::from_yaml(yaml);

        *document = re.replace(document, "").to_string();

        mdobj
    }

    /// Parses the provided YAML Meta Data content into a MarkdownMetaData structure.
    pub fn from_yaml(yaml: serde_yaml::Value) -> Option<MarkdownMetaData> {
        Some(MarkdownMetaData {
            layout: match yaml["layout"].as_str() {
                Some(s) => Layout::from(s),
                _ => {
                    // Unlikely to happen
                    error!(r#"Missing mandatory metadata field "layout" "#);
                    return None;
                }
            },
            title: match yaml["title"].as_str() {
                Some(s) => Some(String::from(s)),
                _ => None,
            },
            description: match yaml["description"].as_str() {
                Some(s) => Some(s.to_string()),
                _ => None,
            },
            source: None,
            published: match yaml["published-on"].as_str() {
                Some(s) => match DateTime::parse_from_rfc3339(s) {
                    Ok(d) => DateTime::from(d),
                    Err(e) => {
                        warn!(
                            "Unable to parse publication date {}: {} ...defaulting to today",
                            s, e
                        );
                        Utc::now()
                    }
                },
                _ => {
                    warn!(r#"Missing metadata field "published-on ...defaulting to today" "#);
                    Utc::now()
                }
            },
        })
    }

    // Gets the markdown metadata from the given file
    pub fn from_file(source:&Path) -> Option<MarkdownMetaData>{
        let mut content = Vec::new();
        let mut source_file = match File::open(&source) {
            Ok(handle) => handle,
            Err(error) => {
                error!("The following error occurred while opening {:#?} : {:#?}", error, &source);
                return None;
            }
        };
        match source_file.read_to_end(&mut content) {
            Ok(_) => {}
            Err(error) => {
                error!("The following error occurred while reading {:#?} : {:#?}", error, &source);
                return None;
            }
        };
    
        let mut utf8_content = match String::from_utf8(content) {
            Ok(m) => m,
            Err(error) => {
                error!("UTF8 conversion error occured for {:#?} ...skipping : {:#?}", &source, error);
                return None;
            }
        };

        MarkdownMetaData::extract(&mut utf8_content)
    }
}