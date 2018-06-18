extern crate clap;
#[macro_use]
extern crate tera;
#[macro_use]
extern crate serde_json;
extern crate actix_web;
extern crate actix_shs;


use std::path;
use std::collections::HashMap;

use serde_json::Value;
use actix_web::{server, http, error, App, Error, State, Path, Query, HttpResponse};

struct AppState {
    template: tera::Tera,
}

fn index(
    (state, path, query): (
        State<AppState>,
        Path<String>,
        Query<HashMap<String, String>>
    )
) -> Result<HttpResponse, Error> {
    let path = path.into_inner();
    let path_buf = path::Path::new(".").join(path.as_str());
    let mut ctx = tera::Context::new();
    ctx.add("path", &Value::String(path.clone()));
    ctx.add("breadcrumb", &Value::Array(vec![]));
    ctx.add("currentLink", &Value::String("/".to_owned()));
    ctx.add("upLink", &Value::String("/".to_owned()));
    ctx.add("isTop", &Value::Bool(true));
    ctx.add("rows", &Value::Array(vec![]));

    let rendered = state.template
        .render("index.jinja2", &ctx)
        .unwrap();
    Ok(HttpResponse::Ok().content_type("text/html").body(rendered))
}

fn main() {
    server::new(|| {
        let tera =
            compile_templates!(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/**/*"));
        App::with_state(AppState{template: tera})
            .resource("/{path:.*}", |r| r.method(http::Method::GET).with(index))
    })
        .bind("127.0.0.1:8080")
        .unwrap()
        .run();
}
