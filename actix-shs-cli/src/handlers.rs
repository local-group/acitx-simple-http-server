
use std::fs;
use std::path;
use std::convert::From;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use std::cmp::Ordering;

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

struct Entry {
    filename: String,
    metadata: fs::Metadata
}

impl RowItem {
    fn new(entry: Entry, path_prefix: &[String]) -> RowItem {
        let Entry { filename, metadata } = entry;

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
    current_directory: String,
    breadcrumb: Vec<SimpleLink>,
    rows: Vec<RowItem>,
    name_order: String,
    modified_order: String,
    size_order: String,
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

        let mut entries = Vec::new();
        for entry_result in fs::read_dir(&fs_path)? {
            let entry = entry_result?;
            entries.push(Entry {
                filename: entry.file_name().to_string_lossy().into_owned(),
                metadata: entry.metadata()?
            });
        }

        let query = req.query();
        let sort_field = query.get("sort").map(|s| s.as_str()).unwrap_or("name");
        let order = query.get("order").map(|s| s.as_str()).unwrap_or("asc");
        entries.sort_by(|a, b| {
            match sort_field {
                "name" => {
                    a.filename.cmp(&b.filename)
                }
                "modified" => {
                    let a = a.metadata.modified().unwrap();
                    let b = b.metadata.modified().unwrap();
                    a.cmp(&b)
                }
                "size" => {
                    if a.metadata.is_dir() == b.metadata.is_dir()
                        || a.metadata.is_file() == b.metadata.is_file() {
                            a.metadata.len().cmp(&b.metadata.len())
                        } else if a.metadata.is_dir() {
                            Ordering::Less
                        } else {
                            Ordering::Greater
                        }
                }
                _ => {
                    a.filename.cmp(&b.filename)
                }
            }
        });
        if order == "desc" {
            entries.reverse();
        }
        let name_order = match (sort_field, order) {
            ("name", "asc") => "desc",
            _ => "asc",
        };
        let modified_order = match (sort_field, order) {
            ("modified", "asc") => "desc",
            _ => "asc",
        };
        let size_order = match (sort_field, order) {
            ("size", "asc") => "desc",
            _ => "asc",
        };

        let rows = entries
            .into_iter()
            .map(|entry| RowItem::new(entry, &path_prefix))
            .collect::<Vec<RowItem>>();

        let mut current_directory = path_prefix.to_owned();
        current_directory.push("".to_owned());
        let rendered = IndexPage {
            current_directory: encode_link_path(&current_directory, true),
            breadcrumb,
            rows,
            name_order: name_order.to_string(),
            modified_order: modified_order.to_string(),
            size_order: size_order.to_string(),
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
