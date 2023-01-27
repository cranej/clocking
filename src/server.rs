use crate::sqlite_store::SqliteStore;
use crate::{views, ClockingItem, ClockingItemId, ClockingStore};
use rocket::{
    get,
    http::{ContentType, Status},
    post,
    serde::json::Json,
    State,
};
use rust_embed::RustEmbed;
use std::ffi::OsStr;
use std::path::PathBuf;

#[derive(RustEmbed)]
#[folder = "asset/"]
struct Asset;

pub struct ServerConfig {
    pub db_file: String,
}

impl ServerConfig {
    fn new_store(&self) -> Box<dyn ClockingStore> {
        Box::new(SqliteStore::new(&self.db_file))
    }
}

#[get("/recent")]
pub fn api_recent(config: &State<ServerConfig>) -> Json<Vec<String>> {
    Json(config.new_store().recent_titles(5))
}

#[get("/latest/<title>")]
pub fn api_latest(title: &str, config: &State<ServerConfig>) -> Json<Option<ClockingItem>> {
    Json(config.new_store().latest(title))
}

#[get("/unfinished")]
pub fn api_unfinished(config: &State<ServerConfig>) -> Json<Vec<ClockingItemId>> {
    Json(config.new_store().unfinished(10))
}

#[post("/start/<title>")]
pub fn api_start(title: &str, config: &State<ServerConfig>) -> Status {
    dbg!(title);
    if title.is_empty() {
        Status::BadRequest
    } else {
        match config.new_store().start_clocking(title) {
            Ok(_) => Status::Ok,
            Err(_) => Status::InternalServerError,
        }
    }
}

#[post("/finish/<title>", data = "<notes>")]
pub fn api_finish(title: &str, notes: String, config: &State<ServerConfig>) -> Status {
    match config
        .new_store()
        .finish_latest_unfinished_by_title(title, &notes)
    {
        Ok(true) => Status::Ok,
        Ok(false) => Status::NotFound,
        Err(_) => Status::InternalServerError,
    }
}

#[get("/report/<offset>/<days>")]
pub fn api_report(offset: u64, days: Option<u64>, config: &State<ServerConfig>) -> String {
    let items = config.new_store().query_clocking_offset(offset, days);
    let view = views::ItemView::new(&items);
    view.daily_summary_detail()
}
/*
#[get("/api/report")]
#[get("/api/report/daily")]
#[get("/api/report/detail")]
*/

#[get("/")]
pub fn index() -> (ContentType, String) {
    let page = Asset::get("index.html").unwrap();
    (
        ContentType::HTML,
        String::from_utf8_lossy(page.data.as_ref()).to_string(),
    )
}

#[get("/favicon.png")]
pub fn favicon() -> (ContentType, Vec<u8>) {
    let page = Asset::get("favicon.png").unwrap();
    (ContentType::PNG, page.data.into_owned())
}

#[get("/<file..>")]
pub fn anyfile(file: PathBuf) -> (ContentType, String) {
    let content_type = match file.as_path().extension() {
        Some(o) => {
            if o == OsStr::new("html") {
                ContentType::HTML
            } else if o == OsStr::new("js") {
                ContentType::JavaScript
            } else if o == OsStr::new("css") {
                ContentType::CSS
            } else if o == OsStr::new("png") {
                ContentType::PNG
            } else {
                ContentType::Binary
            }
        }
        None => ContentType::Binary,
    };
    let page = Asset::get(file.to_str().unwrap()).unwrap();
    (
        content_type,
        String::from_utf8_lossy(page.data.as_ref()).to_string(),
    )
}
