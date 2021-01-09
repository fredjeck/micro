use std::fmt::Display;

use chrono::{DateTime, FixedOffset};
use log::error;
use regex::Regex;
use serde_yaml::Value;

pub struct Metadata {
    layout: String,
    title: String,
    description: String,
    published: DateTime<FixedOffset>,
}

impl Display for Metadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            r#"Metadata{{Layout:"{}", Title:"{}", Description:"{}", Published:"{}"}}"#,
            self.layout, self.title, self.description, self.published
        )
    }
}

impl Metadata {
    pub fn extract(document: &mut String) -> Option<Metadata> {
        let re = Regex::new(r"(?s)---(.*?)---").unwrap();
        if !re.is_match(&document) {
            return None;
        }
    
        let metadata_text = re.captures(&document).unwrap()[1].to_string();
        let yaml: Value = match serde_yaml::from_str(&metadata_text) {
            Ok(v) => v,
            Err(_) => Value::default(),
        };
    
        let mdobj = Metadata::from_yaml(&yaml);
    
        *document = re.replace(document, "").to_string();
    
        mdobj
    }

    pub fn from_yaml(yaml: &serde_yaml::Value) -> Option<Metadata> {
        Some(Metadata {
            layout : match yaml["layout"].as_str() {
                Some(s) => String::from(s),
                _ =>  {
                    error!( r#"Missing metadata field "layout" "#);
                    return None;
                },
            },
            title: match yaml["title"].as_str() {
                Some(s) => String::from(s),
                _ => {
                    error!( r#"Missing metadata field "title" "#);
                    return None;
                },
            },
            description:  match yaml["description"].as_str() {
                Some(s) => String::from(s),
                _ => {
                    error!( r#"Missing metadata field "description" "#);
                    return None;
                },
            },
            published:  match yaml["published-on"].as_str() {
                Some(s) => match DateTime::parse_from_rfc3339(s) {
                    Ok(d) => d,
                    Err(e) => {
                        error!("Unable to parse publication date {}: {}", s, e);
                        return None;
                    }
                },
                _ => {
                    error!( r#"Missing metadata field "published-on" "#);
                    return None;
                },
            },
        })
    }
}