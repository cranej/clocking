//! Rocket request handlers.
use crate::{new_sqlite_store, types::EntryId, views, ClockingStore};
use rocket::{
    get,
    http::{ContentType, Status},
    post,
    serde::json::Json,
    State,
};
use rust_embed::RustEmbed;
use std::path::PathBuf;

#[derive(RustEmbed)]
#[folder = "asset/"]
struct Asset;

pub struct ServerConfig {
    pub db_file: String,
}

impl ServerConfig {
    fn new_store(&self) -> Box<dyn ClockingStore> {
        Box::new(new_sqlite_store(&self.db_file))
    }
}

#[get("/recent")]
pub fn api_recent(config: &State<ServerConfig>) -> Json<Vec<String>> {
    // TODO: remove unwrap
    Json(config.new_store().recent_titles(5).unwrap())
}

#[get("/latest/<title>")]
pub fn api_latest(title: &str, config: &State<ServerConfig>) -> String {
    // TODO: remove unwrap
    config
        .new_store()
        .latest_finished(title)
        .unwrap()
        .map(|entity| entity.html_segment())
        .unwrap_or_else(String::new)
}

#[get("/unfinished")]
pub fn api_unfinished(config: &State<ServerConfig>) -> Json<Vec<EntryId>> {
    // TODO: remove unwrap
    let r: Vec<EntryId> = config
        .new_store()
        .unfinished(10)
        .unwrap()
        .into_iter()
        .map(|x| x.id)
        .collect();
    Json(r)
}

#[post("/start/<title>")]
pub fn api_start(title: &str, config: &State<ServerConfig>) -> Status {
    if title.is_empty() {
        Status::BadRequest
    } else {
        match config.new_store().start(title) {
            Ok(_) => Status::Ok,
            Err(_) => Status::InternalServerError,
        }
    }
}

#[post("/finish/<title>", data = "<notes>")]
pub fn api_finish(title: &str, notes: String, config: &State<ServerConfig>) -> Status {
    match config.new_store().try_finish_title(title, &notes) {
        Ok(true) => Status::Ok,
        Ok(false) => Status::NotFound,
        Err(_) => Status::InternalServerError,
    }
}

#[get("/report/<offset>/<days>?<view_type>")]
pub fn api_report(
    offset: u64,
    days: Option<u64>,
    view_type: &str,
    config: &State<ServerConfig>,
) -> String {
    // TODO: remove unwrap
    let entries = config.new_store().finished_by_offset(offset, days).unwrap();
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
pub fn api_report_by_date(
    start: &str,
    end: &str,
    view_type: &str,
    config: &State<ServerConfig>,
) -> (Status, String) {
    match config.new_store().finished_by_date_str(start, end) {
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
pub fn index() -> (ContentType, String) {
    // TODO: get rid of unwrap
    let page = Asset::get("index.html").unwrap();
    (
        ContentType::HTML,
        String::from_utf8_lossy(page.data.as_ref()).to_string(),
    )
}

#[get("/favicon.png")]
pub fn favicon() -> (ContentType, Vec<u8>) {
    // TODO: get rid of unwrap
    let page = Asset::get("favicon.png").unwrap();
    (ContentType::PNG, page.data.into_owned())
}

#[get("/<file..>")]
pub fn anyfile(file: PathBuf) -> (ContentType, String) {
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
