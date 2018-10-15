extern crate clap;
#[macro_use]
extern crate askama;
extern crate pretty_bytes;
extern crate url;
extern crate chrono;
extern crate actix_web;
extern crate actix_shs;

use std::fs;
use std::path;
use std::convert::From;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use url::percent_encoding::{utf8_percent_encode, PATH_SEGMENT_ENCODE_SET};
use chrono::{DateTime, Local, TimeZone};
use pretty_bytes::converter::convert;
use askama::Template;
use actix_web::{server, http, error, App, Error, Path, Query, HttpResponse};

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

fn encode_link_path(path: &[String]) -> String {
    path.iter().map(|s| {
        utf8_percent_encode(s, PATH_SEGMENT_ENCODE_SET).to_string()
    }).collect::<Vec<String>>().join("/")
}

struct RowItem {
    filename: String,
    linkstyle: &'static str,
    link: String,
    label: String,
    modified: String,
    filesize: String,
}

impl<'a> From<&'a fs::DirEntry> for RowItem {
    fn from(entry: &'a fs::DirEntry) -> Self {
        let filename = entry.file_name().to_string_lossy().into_owned();
        let metadata = entry.metadata().unwrap();

        let modified = system_time_to_date_time(metadata.modified().unwrap())
            .format("%Y-%m-%d %H:%M:%S").to_string();

        let filesize = if metadata.is_dir() {
            "-".to_owned()
        } else {
            convert(metadata.len() as f64)
        };

        let label = if metadata.is_dir() {
            format!("{}/", &filename)
        } else { filename.clone() };

        let mut link_parts = vec![];
        link_parts.push(filename.clone());
        if metadata.is_dir() {
            link_parts.push("".to_owned());
        }
        let link = encode_link_path(&link_parts);

        let linkstyle = if metadata.is_dir() {
            "font-weight: bold;"
        } else {
            ""
        };

        RowItem {
            filename,
            linkstyle,
            link,
            label,
            modified,
            filesize,
        }
    }
}

struct SimpleLink<'a> {
    link: &'a str,
    label: &'a str,
}

struct UpLink<'a> {
    exists: bool,
    link: &'a str,
    label: &'a str,
}

#[derive(Template)]
#[template(path = "index.jinja2")]
struct IndexPage<'a> {
    path: &'a str,
    breadcrumb: Vec<SimpleLink<'a>>,
    up_link: UpLink<'a>,
    current_link: &'a str,
    rows: Vec<RowItem>
}

fn index(
    (path, query): (Path<String>, Query<HashMap<String, String>>)
) -> Result<HttpResponse, Error> {
    let path = path.into_inner();
    let path_buf = path::Path::new(".").join(path.as_str());

    let rows = fs::read_dir(&path_buf)?
        .map(|result| result.unwrap())
        .map(|dir_entry| RowItem::from(&dir_entry))
        .collect::<Vec<RowItem>>();

    let rendered = IndexPage {
        path: path.as_str(),
        breadcrumb: vec![],
        up_link: UpLink{ exists: false, link: "", label: "" },
        current_link: "/",
        rows
    }.render()
        .unwrap();
    Ok(HttpResponse::Ok().content_type("text/html").body(rendered))
}

fn main() {
    server::new(|| {
        App::new()
            .resource("/{path:.*}", |r| r.method(http::Method::GET).with(index))
    })
        .bind("127.0.0.1:8080")
        .unwrap()
        .run();
}
