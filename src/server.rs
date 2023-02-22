//! Rocket request handlers.
use crate::{types::EntryId, views, ClockingStore};
use rocket::{
    get,
    http::{ContentType, Status},
    post,
    serde::json::Json,
    State,
};
use rust_embed::RustEmbed;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(RustEmbed)]
#[folder = "asset/"]
struct Asset;

type ServerConfig = Arc<Mutex<dyn ClockingStore + Send>>;

pub async fn launch_server(
    port: u16,
    address: std::net::IpAddr,
    mount_base: Option<&str>,
    store: ServerConfig,
) -> Result<rocket::Rocket<rocket::Ignite>, rocket::Error> {
    let config = rocket::config::Config {
        port,
        address,
        ..rocket::config::Config::default()
    };

    let (api_mount, root_mount) = match mount_base {
        Some("/") | Some("") | None => ("/api".to_string(), "/".to_string()),
        Some(point) => (format!("{point}/api"), point.to_string()),
    };
    let rocket = rocket::custom(&config)
        .manage(store)
        .mount(
            api_mount,
            rocket::routes![
                api_recent,
                api_latest,
                api_unfinished,
                api_start,
                api_finish,
                api_report,
                api_report_by_date,
            ],
        )
        .mount(root_mount, rocket::routes![index, favicon, anyfile,]);

    rocket.ignite().await.unwrap().launch().await
}

#[get("/recent")]
fn api_recent(config: &State<ServerConfig>) -> Json<Vec<String>> {
    let store = config.lock().unwrap();
    // TODO: remove unwrap
    Json(store.recent_titles(5).unwrap())
}

#[get("/latest/<title>")]
fn api_latest(title: &str, config: &State<ServerConfig>) -> String {
    let store = config.lock().unwrap();
    // TODO: remove unwrap
    store
        .latest_finished(title)
        .unwrap()
        .map(|entity| entity.html_segment())
        .unwrap_or_else(String::new)
}

#[get("/unfinished")]
fn api_unfinished(config: &State<ServerConfig>) -> Json<Vec<EntryId>> {
    let store = config.lock().unwrap();
    // TODO: remove unwrap
    let r: Vec<EntryId> = store
        .unfinished(10)
        .unwrap()
        .into_iter()
        .map(|x| x.id)
        .collect();
    Json(r)
}

#[post("/start/<title>")]
fn api_start(title: &str, config: &State<ServerConfig>) -> Status {
    if title.is_empty() {
        Status::BadRequest
    } else {
        let mut store = config.lock().unwrap();
        match store.start(title) {
            Ok(_) => Status::Ok,
            Err(_) => Status::InternalServerError,
        }
    }
}

#[post("/finish/<title>", data = "<notes>")]
fn api_finish(title: &str, notes: String, config: &State<ServerConfig>) -> Status {
    let mut store = config.lock().unwrap();
    match store.try_finish_title(title, &notes) {
        Ok(true) => Status::Ok,
        Ok(false) => Status::NotFound,
        Err(_) => Status::InternalServerError,
    }
}

#[get("/report/<offset>/<days>?<view_type>")]
fn api_report(
    offset: u64,
    days: Option<u64>,
    view_type: &str,
    config: &State<ServerConfig>,
) -> String {
    let store = config.lock().unwrap();
    // TODO: remove unwrap
    let entries = store.finished_by_offset(offset, days).unwrap();
    if view_type == "daily" {
        let view = views::DailySummaryView::new(&entries);
        view.to_string()
    } else if view_type == "detail" {
        let view = views::EntryDetailView::new(&entries);
        view.to_string()
    } else if view_type == "dist" {
        let view = views::DailyDistributionView::new(&entries);
        view.to_string()
    } else {
        // default to view type 'daily_detail'
        let view = views::DailyDetailView::new(&entries);
        view.to_string()
    }
}

#[get("/report-by-date/<start>/<end>?<view_type>")]
fn api_report_by_date(
    start: &str,
    end: &str,
    view_type: &str,
    config: &State<ServerConfig>,
) -> (Status, String) {
    let store = config.lock().unwrap();
    match store.finished_by_date_str(start, end) {
        Ok(entries) => {
            let resp = if view_type == "daily" {
                let view = views::DailySummaryView::new(&entries);
                view.to_string()
            } else if view_type == "detail" {
                let view = views::EntryDetailView::new(&entries);
                view.to_string()
            } else if view_type == "dist" {
                let view = views::DailyDistributionView::new(&entries);
                view.to_string()
            } else {
                // default to view type 'daily_detail'
                let view = views::DailyDetailView::new(&entries);
                view.to_string()
            };

            (Status::Ok, resp)
        }
        Err(err) => (Status::BadRequest, err.to_string()),
    }
}

#[get("/")]
fn index() -> (ContentType, String) {
    // TODO: get rid of unwrap
    let page = Asset::get("index.html").unwrap();
    (
        ContentType::HTML,
        String::from_utf8_lossy(page.data.as_ref()).to_string(),
    )
}

#[get("/favicon.png")]
fn favicon() -> (ContentType, Vec<u8>) {
    // TODO: get rid of unwrap
    let page = Asset::get("favicon.png").unwrap();
    (ContentType::PNG, page.data.into_owned())
}

#[get("/<file..>")]
fn anyfile(file: PathBuf) -> (ContentType, String) {
    let content_type = match file.as_path().extension() {
        Some(o) => {
            if o == "html" {
                ContentType::HTML
            } else if o == "js" {
                ContentType::JavaScript
            } else if o == "css" {
                ContentType::CSS
            } else if o == "png" {
                ContentType::PNG
            } else {
                ContentType::Binary
            }
        }
        None => ContentType::Binary,
    };
    // TODO: get rid of unwrap
    let page = Asset::get(file.to_str().unwrap()).unwrap();
    (
        content_type,
        String::from_utf8_lossy(page.data.as_ref()).to_string(),
    )
}
