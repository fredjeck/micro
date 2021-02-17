use std::fmt::Display;

use chrono::{DateTime, Utc};
use log::{error, warn};
use regex::Regex;
use serde_yaml::Value;

/// Template to apply to a markdown file during its rendering.
#[derive(Debug)]
pub enum Layout {
    Article,
    Index,
}

impl Display for Layout {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Layout::Article => "Article",
                Layout::Index => "Index",
            }
        )
    }
}

impl From<&str> for Layout {
    fn from(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "index" => Self::Index,
            _ => return Self::Article,
        }
    }
}

/// Optional metadata segment to be used in markdown files.
pub struct MarkdownMetaData {
    pub layout: Layout,
    pub title: Option<String>,
    pub description: Option<String>,
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
}