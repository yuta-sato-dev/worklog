use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub type Result<T> = std::result::Result<T, String>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Sample {
    pub id: i64,
    pub timestamp: DateTime<Utc>,
    pub interval_minutes: Option<u32>,
    pub app: String,
    pub title: String,
    pub project: Option<String>,
    pub source: String,
    pub status: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct Block {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub seconds: i64,
    pub app: String,
    pub title: String,
    pub project: Option<String>,
    pub source: String,
    pub status: String,
    pub sample_count: usize,
}
#[derive(Clone, Debug, Serialize)]
pub struct DayReport {
    pub work_seconds: i64,
    pub idle_seconds: i64,
    pub projects: Vec<ProjectReport>,
}
#[derive(Clone, Debug, Serialize)]
pub struct ProjectReport {
    pub project: Option<String>,
    pub seconds: i64,
    pub entries: Vec<ReportEntry>,
}
#[derive(Clone, Debug, Serialize)]
pub struct ReportEntry {
    pub app: String,
    pub title: String,
    pub seconds: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rule {
    pub id: i64,
    pub app: String,
    pub contains: String,
    pub project: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub interval_minutes: u32,
    pub idle_minutes: u32,
    pub excluded_apps: Vec<String>,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            interval_minutes: 5,
            idle_minutes: 5,
            excluded_apps: Vec::new(),
        }
    }
}
impl Settings {
    pub fn validated(mut self) -> Result<Self> {
        if !(1..=60).contains(&self.interval_minutes) {
            return Err("記録間隔は1〜60分で指定してください。".into());
        }
        if !(1..=120).contains(&self.idle_minutes) {
            return Err("離席判定は1〜120分で指定してください。".into());
        }
        if self.excluded_apps.len() > 100 || self.excluded_apps.iter().any(|app| app.len() > 256) {
            return Err("除外アプリは100件以内、各256バイト以内で指定してください。".into());
        }
        let mut seen = std::collections::HashSet::new();
        self.excluded_apps = self
            .excluded_apps
            .into_iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty() && seen.insert(s.to_lowercase()))
            .collect();
        Ok(self)
    }
    pub fn excludes(&self, app: &str) -> bool {
        app.eq_ignore_ascii_case("worklog")
            || self
                .excluded_apps
                .iter()
                .any(|excluded| excluded.to_lowercase() == app.to_lowercase())
    }
}
