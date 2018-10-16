
use std::fs;
use std::path;
use std::convert::From;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use url::percent_encoding::{utf8_percent_encode, percent_decode, PATH_SEGMENT_ENCODE_SET};
use chrono::{DateTime, Local, TimeZone};
use pretty_bytes::converter::convert;
use askama::Template;
use actix_web::{server, http, error, App, Error, Path, Query, HttpRequest, HttpResponse};
use actix_web::dev::Handler;

use args::Args;

fn system_time_to_date_time(t: SystemTime) -> DateTime<Local> {
    let (sec, nsec) = match t.duration_since(UNIX_EPOCH) {
        Ok(dur) => (dur.as_secs() as i64, dur.subsec_nanos()),
        Err(e) => { // unlikely but should be handled
            let dur = e.duration();
            let (sec, nsec) = (dur.as_secs() as i64, dur.subsec_nanos());
            if nsec == 0 {
                (-sec, 0)
            } else {
                (-sec - 1, 1_000_000_000 - nsec)
            }
        },
    };
    Local.timestamp(sec, nsec)
}

fn encode_link_path(path: &[String], with_root: bool) -> String {
    let link = path.iter().map(|s| {
        utf8_percent_encode(s, PATH_SEGMENT_ENCODE_SET).to_string()
    }).collect::<Vec<String>>().join("/");
    if with_root {
        format!("/{}", link)
    } else {
        link
    }
}

#[derive(Eq, PartialEq)]
enum FileType {
    Directory,
    File,
}

impl FileType {
    pub fn to_str(&self) -> &'static str {
        match self {
            FileType::Directory => "directory",
            FileType::File => "file"
        }
    }
}

struct RowItem {
    file_type: &'static str,
    filename: String,
    link: String,
    link_class: String,
    modified: String,
    filesize: String,
}

impl RowItem {
    fn new(entry: &fs::DirEntry, path_prefix: &[String]) -> RowItem {
        let filename = entry.file_name().to_string_lossy().into_owned();
        let metadata = entry.metadata().unwrap();

        let modified = system_time_to_date_time(metadata.modified().unwrap())
            .format("%Y-%m-%d %H:%M:%S").to_string();

        let filesize = if metadata.is_dir() {
            "-".to_owned()
        } else {
            convert(metadata.len() as f64)
        };

        let file_type = if metadata.is_dir() {
            FileType::Directory
        } else {
            FileType::File
        };

        let mut link_parts = path_prefix.to_owned();
        link_parts.push(filename.clone());
        if metadata.is_dir() {
            link_parts.push("".to_owned());
        }
        let link = encode_link_path(&link_parts, true);
        let link_class = format!("link-{}", file_type.to_str());

        RowItem {
            file_type: file_type.to_str(),
            filename,
            link,
            link_class,
            modified,
            filesize,
        }
    }
}

struct SimpleLink {
    link: String,
    label: String,
}

#[derive(Template)]
#[template(path = "index.jinja2", print = "all")]
struct IndexPage {
    directory: String,
    breadcrumb: Vec<SimpleLink>,
    current_link: String,
    rows: Vec<RowItem>
}


#[derive(Clone)]
pub struct MethodGetHandler {
    args: Arc<Args>
}

impl MethodGetHandler {
    pub fn new(args: Arc<Args>) -> MethodGetHandler {
        MethodGetHandler { args }
    }
}

impl<S> Handler<S> for MethodGetHandler {
    type Result = Result<HttpResponse, Error>;

    fn handle(&self, req: &HttpRequest<S>) -> Self::Result {
        let mut fs_path = self.args.root.clone();
        let path_prefix = req
            .path()
            .split('/')
            .filter(|s| !s.is_empty())
            .map(|s| {
                percent_decode(s.as_bytes())
                    .decode_utf8().unwrap()
                    .to_string()
            })
            .collect::<Vec<String>>();
        for part in &path_prefix {
            fs_path.push(part);
        }

        let mut breadcrumb = Vec::new();
        if !path_prefix.is_empty() {
            let mut parts = path_prefix.to_owned();
            breadcrumb.push(SimpleLink {
                link: String::new(),
                label: parts.pop().unwrap().to_string(),
            });
            while !parts.is_empty() {
                breadcrumb.push(SimpleLink {
                    link: encode_link_path(&parts, true),
                    label: parts.pop().unwrap().to_string(),
                });
            }
            breadcrumb.reverse();
        }

        let rows = fs::read_dir(&fs_path)?
            .map(|result| result.unwrap())
            .map(|dir_entry| RowItem::new(&dir_entry, &path_prefix))
            .collect::<Vec<RowItem>>();

        let rendered = IndexPage {
            directory: encode_link_path(&path_prefix, true),
            current_link: format!("{}/", encode_link_path(&path_prefix, true)),
            breadcrumb,
            rows,
        }.render()
            .unwrap();
        Ok(HttpResponse::Ok().content_type("text/html").body(rendered))
    }
}


#[derive(Clone)]
pub struct MethodHeadHandler {
    args: Arc<Args>
}

impl MethodHeadHandler {
    pub fn new(args: Arc<Args>) -> MethodHeadHandler {
        MethodHeadHandler { args }
    }
}

impl<S> Handler<S> for MethodHeadHandler {
    type Result = Result<HttpResponse, Error>;

    fn handle(&self, req: &HttpRequest<S>) -> Self::Result {
        Ok(HttpResponse::Ok().content_type("text/html").finish())
    }
}
