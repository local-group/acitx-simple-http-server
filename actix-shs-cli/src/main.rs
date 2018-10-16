#[macro_use]
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
use std::sync::Arc;
use std::convert::From;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use url::percent_encoding::{utf8_percent_encode, PATH_SEGMENT_ENCODE_SET};
use chrono::{DateTime, Local, TimeZone};
use pretty_bytes::converter::convert;
use askama::Template;
use actix_web::{server, http, error, App, Error, Path, Query, HttpResponse};

mod args;
mod handlers;

use handlers::{MethodGetHandler, MethodHeadHandler};

fn main() {
    let args = Arc::new(args::parse_args());
    println!("{:#?}", args);
    server::new(move || {
        let method_get_handler = MethodGetHandler::new(Arc::clone(&args));
        let method_head_handler = MethodHeadHandler::new(Arc::clone(&args));
        App::new()
            .resource("/{path:.*}", move |r| {
                r.get().h(method_get_handler);
                r.get().h(method_head_handler);
            })
    })
        .bind("127.0.0.1:8080")
        .unwrap()
        .run();
}
