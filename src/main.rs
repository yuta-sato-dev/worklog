use fetch_focused_window::{Store, capture_with_settings};
use std::path::PathBuf;
fn main() -> Result<(), String> {
    let path = match std::env::var_os("WORKLOG_DB") {
        Some(path) => PathBuf::from(path),
        None => PathBuf::from(
            std::env::var_os("HOME").ok_or("HOMEがありません。WORKLOG_DBを指定してください。")?,
        )
        .join("Library/Application Support/jp.local.worklog/worklog.sqlite3"),
    };
    let store = Store::open(&path)?;
    if store.paused()? {
        return Ok(());
    }
    let settings = store.settings()?;
    let sample = capture_with_settings(&settings)?;
    if settings.excludes(&sample.app) {
        return Ok(());
    }
    store.append(&sample)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&sample).map_err(|e| e.to_string())?
    );
    Ok(())
}
