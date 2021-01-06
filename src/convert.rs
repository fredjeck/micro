// use chrono::{DateTime, Utc};
use log::trace;
use pulldown_cmark::{html, Options, Parser};
use simple_error::bail;
use std::{
    env,
    // error::Error,
    fs::File,
    io::{prelude::*, BufWriter},
    path::Path,
    str,
};
use std::{
    error, 
    ffi::OsStr,
    //  time::SystemTime
    };

// pub fn md_to_html(p: &Path) -> Result<(), Box<dyn Error>> {
//     let mut f = File::open(p).unwrap();

//     let mut buffer = Vec::new();
//     f.read_to_end(&mut buffer).unwrap();

//     let parser = Parser::new_ext(str::from_utf8(&buffer).unwrap(), Options::all());

//     let mut html_output = String::new();
//     html::push_html(&mut html_output, parser);

//     let f = File::create(p.with_extension("html")).unwrap();
//     let mut writer = BufWriter::new(f);

//     let pbuf = std::env::current_dir()?;
//     let current_dir = pbuf.as_path();
//     trace!("Current Directory {}", current_dir.to_str().unwrap());
//     let mut header = Vec::new();
//     let hpath = current_dir.join("templates").join("header.html");
//     trace!("Header {}", hpath.to_str().unwrap());
//     let mut hfile = File::open(hpath)?;
//     hfile.read_to_end(&mut header)?;

//     let system_time = SystemTime::now();
//     let datetime: DateTime<Utc> = system_time.into();
//     writer.write(&header)?;
//     writer.write(
//         format!(
//             r#"<div class="updated">{}</div>
//     <article>"#,
//             datetime.format("%+")
//         )
//         .as_bytes(),
//     )?;
//     writer.write(html_output.as_bytes())?;
//     writer.write("</article>".as_bytes())?;

//     Ok(())
// }

pub fn publish(source: &Path) -> Result<(), Box<dyn error::Error>> {
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
    }

    let parser = Parser::new_ext(str::from_utf8(&markdown)?, Options::all());

    let mut html = String::new();
    html::push_html(&mut html, parser);

    let mut template = Vec::new();
    load_template("article.html", &mut template)?;

    let content = str::from_utf8(&template)?;

    let document = content.replace("{{article}}", &html);

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

    //    let f = File::create(p.with_extension("html")).unwrap();
    //    let mut writer = BufWriter::new(f);

    //    let pbuf = std::env::current_dir()?;
    //    let current_dir = pbuf.as_path();
    //    trace!("Current Directory {}", current_dir.to_str().unwrap());
    //    let mut header = Vec::new();
    //    let hpath = current_dir.join("templates").join("header.html");
    //    trace!("Header {}", hpath.to_str().unwrap());
    //    let mut hfile = File::open(hpath)?;
    //    hfile.read_to_end(&mut header)?;

    //    let system_time = SystemTime::now();
    //    let datetime: DateTime<Utc> = system_time.into();
    //    writer.write(&header)?;
    //    writer.write(
    //        format!(
    //            r#"<div class="updated">{}</div>
    //    <article>"#,
    //            datetime.format("%+")
    //        )
    //        .as_bytes(),
    //    )?;
    //    writer.write(html_output.as_bytes())?;
    //    writer.write("</article>".as_bytes())?;

    Ok(())
}

fn load_template(name: &'static str, buffer: &mut Vec<u8>) -> Result<usize, Box<dyn error::Error>> {
    let cwd = env::current_dir()?.join("templates");

    let template_path = cwd.join(name);
    trace!("Loading template from {:#?}", &template_path);

    let mut template_file = match File::open(&template_path) {
        Ok(handle) => handle,
        Err(error) => bail!(
            "An error occured while trying to load the template {:#?} for reading {}",
            &template_path,
            error
        ),
    };

    let bytes = match template_file.read_to_end(buffer) {
        Ok(b) => b,
        Err(error) => bail!(
            "An error occured reading {:#?} content {}",
            template_path,
            error
        ),
    };

    Ok(bytes)
}
