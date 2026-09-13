use chrono::{DateTime, Local, Utc};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::path::Path;
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

pub fn project_from_title(app: &str, title: &str) -> Option<String> {
    // This explicit convention works with any terminal that supports OSC titles.
    if let Some(project) = project_from_terminal_title(title) {
        return Some(project);
    }

    match app_kind(app)? {
        AppKind::VsCode => project_from_vscode_title(title),
        AppKind::Zed => project_from_zed_title(title),
        AppKind::JetBrains => project_from_jetbrains_title(title),
        AppKind::Xcode => project_from_xcode_title(title),
        AppKind::Obsidian => project_from_obsidian_title(title),
        AppKind::Figma => project_from_figma_title(title),
        AppKind::SublimeText => project_from_sublime_title(title),
        AppKind::Office => project_from_office_title(title),
        AppKind::IWork => project_from_iwork_title(title),
        AppKind::Sketch => {
            project_from_document_title(title, &["Sketch"], &["Untitled", "名称未設定"])
        }
        AppKind::AdobePhotoshop | AppKind::AdobeIllustrator => {
            project_from_adobe_raster_title(title)
        }
        AppKind::AdobeXd => project_from_adobe_xd_title(title),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AppKind {
    VsCode,
    Zed,
    JetBrains,
    Xcode,
    Obsidian,
    Figma,
    SublimeText,
    Office,
    IWork,
    Sketch,
    AdobePhotoshop,
    AdobeIllustrator,
    AdobeXd,
}

fn app_kind(app: &str) -> Option<AppKind> {
    let app = app.trim();
    if is_exact_app(
        app,
        &[
            "Code",
            "Visual Studio Code",
            "Code - Insiders",
            "Visual Studio Code - Insiders",
            "VSCodium",
            "Cursor",
            "Windsurf",
        ],
    ) {
        return Some(AppKind::VsCode);
    }
    if is_exact_app(app, &["Zed", "Zed Preview"]) {
        return Some(AppKind::Zed);
    }
    if is_product_app(
        app,
        &[
            "IntelliJ IDEA",
            "WebStorm",
            "PyCharm",
            "GoLand",
            "RustRover",
            "PhpStorm",
            "CLion",
            "Rider",
            "RubyMine",
            "DataGrip",
            "Android Studio",
        ],
    ) {
        return Some(AppKind::JetBrains);
    }
    if is_exact_app(app, &["Xcode"]) {
        return Some(AppKind::Xcode);
    }
    if is_exact_app(app, &["Obsidian"]) {
        return Some(AppKind::Obsidian);
    }
    if is_exact_app(app, &["Figma"]) {
        return Some(AppKind::Figma);
    }
    if is_exact_app(app, &["Sublime Text"]) {
        return Some(AppKind::SublimeText);
    }
    if is_exact_app(
        app,
        &["Microsoft Word", "Microsoft Excel", "Microsoft PowerPoint"],
    ) {
        return Some(AppKind::Office);
    }
    if is_exact_app(
        app,
        &["Numbers", "Numbers Creator Studio", "Pages", "Keynote"],
    ) {
        return Some(AppKind::IWork);
    }
    if is_exact_app(app, &["Sketch"]) {
        return Some(AppKind::Sketch);
    }
    if is_product_app(app, &["Adobe Photoshop"]) {
        return Some(AppKind::AdobePhotoshop);
    }
    if is_product_app(app, &["Adobe Illustrator"]) {
        return Some(AppKind::AdobeIllustrator);
    }
    if is_exact_app(app, &["Adobe XD"]) {
        return Some(AppKind::AdobeXd);
    }
    None
}

fn is_exact_app(app: &str, names: &[&str]) -> bool {
    names.iter().any(|name| app.eq_ignore_ascii_case(name))
}

fn is_product_app(app: &str, names: &[&str]) -> bool {
    names.iter().any(|name| {
        app.eq_ignore_ascii_case(name)
            || app
                .get(..name.len())
                .is_some_and(|prefix| prefix.eq_ignore_ascii_case(name))
                && app[name.len()..].starts_with(' ')
    })
}

fn cleaned_title(title: &str) -> String {
    let mut out = title.trim();
    loop {
        let trimmed = out
            .trim_start_matches(|c: char| c.is_whitespace())
            .trim_start_matches(|c| matches!(c, '●' | '•' | '◦' | '*'))
            .trim_start_matches(|c: char| c.is_whitespace());
        if trimmed.len() == out.len() {
            break;
        }
        out = trimmed;
    }
    out.trim().to_string()
}

fn strip_app_suffix<'a>(title: &'a str, app_names: &[&str]) -> &'a str {
    let mut out = title.trim();
    loop {
        let before = out;
        for separator in [" — ", " – ", " - "] {
            for app_name in app_names {
                let suffix = format!("{separator}{app_name}");
                let start = out.len().saturating_sub(suffix.len());
                if out.len() > suffix.len()
                    && out
                        .get(start..)
                        .is_some_and(|tail| tail.eq_ignore_ascii_case(&suffix))
                {
                    out = out.get(..start).unwrap_or(out).trim();
                    break;
                }
            }
        }
        if out == before {
            return out;
        }
    }
}

fn strip_remote_suffix(project: &str) -> &str {
    let project = project.trim();
    if let Some((prefix, suffix)) = project.rsplit_once(" [") {
        let suffix = suffix.trim_end_matches(']');
        if ["SSH:", "WSL:", "Dev Container:", "Codespaces:"]
            .iter()
            .any(|marker| suffix.starts_with(marker))
        {
            return prefix.trim();
        }
    }
    project
}

fn non_empty_project(project: &str) -> Option<String> {
    Some(project.trim().to_string()).filter(|s| !s.is_empty())
}

fn strip_known_extension(name: &str) -> &str {
    let name = name.trim();
    if let Some((stem, extension)) = name.rsplit_once('.') {
        let extension = extension.to_ascii_lowercase();
        if matches!(
            extension.as_str(),
            "doc"
                | "docx"
                | "xls"
                | "xlsx"
                | "xlsm"
                | "ppt"
                | "pptx"
                | "key"
                | "numbers"
                | "pages"
                | "sketch"
                | "psd"
                | "psb"
                | "ai"
                | "xd"
                | "xcodeproj"
                | "xcworkspace"
        ) {
            return stem.trim();
        }
    }
    name
}

fn project_from_vscode_title(title: &str) -> Option<String> {
    let title = cleaned_title(title);
    let title = strip_app_suffix(
        &title,
        &[
            "Code",
            "Visual Studio Code",
            "Code - Insiders",
            "Visual Studio Code - Insiders",
            "VSCodium",
            "Cursor",
            "Windsurf",
        ],
    );
    if title.contains(" — ") {
        let parts: Vec<_> = title.split(" — ").collect();
        if parts.len() == 2 {
            return non_empty_project(strip_remote_suffix(parts[1]));
        }
        return None;
    }
    if title.contains(" - ") {
        let parts: Vec<_> = title.split(" - ").collect();
        if parts.len() == 2 {
            return non_empty_project(strip_remote_suffix(parts[1]));
        }
    }
    None
}

fn project_from_zed_title(title: &str) -> Option<String> {
    let title = cleaned_title(title);
    let title = strip_app_suffix(&title, &["Zed", "Zed Preview"]);
    let parts: Vec<_> = title.split(" — ").collect();
    if parts.len() < 2 {
        return None;
    }
    let project = parts[0].trim();
    if project.eq_ignore_ascii_case("empty project") {
        return None;
    }
    non_empty_project(project)
}

fn project_from_jetbrains_title(title: &str) -> Option<String> {
    let title = cleaned_title(title);
    if title.to_ascii_lowercase().starts_with("welcome to ") {
        return None;
    }
    let title = strip_app_suffix(
        &title,
        &[
            "IntelliJ IDEA",
            "WebStorm",
            "PyCharm",
            "GoLand",
            "RustRover",
            "PhpStorm",
            "CLion",
            "Rider",
            "RubyMine",
            "DataGrip",
            "Android Studio",
        ],
    );
    let parts: Vec<_> = title.split(" – ").collect();
    if parts.len() < 2 {
        return None;
    }
    let project = parts[0].trim();
    let project = project
        .rfind(" [")
        .map(|index| &project[..index])
        .unwrap_or(project);
    non_empty_project(project)
}

fn project_from_xcode_title(title: &str) -> Option<String> {
    let title = cleaned_title(title);
    if title.eq_ignore_ascii_case("Welcome to Xcode") {
        return None;
    }
    let title = strip_app_suffix(&title, &["Xcode"]);
    let parts: Vec<_> = title.split(" — ").collect();
    if parts.len() < 2 {
        return None;
    }
    non_empty_project(strip_known_extension(parts[0]))
}

fn project_from_obsidian_title(title: &str) -> Option<String> {
    let title = cleaned_title(title);
    let mut parts = title.rsplitn(3, " - ");
    let app = parts.next()?.trim();
    let vault = parts.next()?.trim();
    parts.next()?;
    if app.starts_with("Obsidian") {
        non_empty_project(vault)
    } else {
        None
    }
}

fn project_from_figma_title(title: &str) -> Option<String> {
    let title = cleaned_title(title);
    let project = title
        .strip_suffix(" – Figma")
        .or_else(|| title.strip_suffix(" - Figma"))
        .unwrap_or(&title)
        .trim();
    non_empty_project(project).filter(|s| s != "Figma")
}

fn project_from_terminal_title(title: &str) -> Option<String> {
    let rest = terminal_worklog_path(title)?;
    let path = rest
        .split(" — ")
        .next()
        .unwrap_or(rest)
        .trim()
        .trim_end_matches(['/', '\\']);
    path.rsplit(|c| c == '/' || c == '\\')
        .next()
        .map(str::to_string)
        .filter(|s| !s.is_empty())
}

fn terminal_worklog_path(title: &str) -> Option<&str> {
    for (index, _) in title.match_indices("worklog:") {
        let prefix = &title[..index];
        let allowed_prefix = prefix
            .chars()
            .next_back()
            .is_none_or(|previous| previous.is_whitespace());
        let rest = &title[index + "worklog:".len()..];
        if allowed_prefix && looks_like_path_start(rest) {
            return Some(rest);
        }
    }
    None
}

fn looks_like_path_start(rest: &str) -> bool {
    if rest.starts_with('/') || rest.starts_with('~') {
        return true;
    }
    let mut chars = rest.chars();
    matches!(
        (chars.next(), chars.next(), chars.next()),
        (Some(drive), Some(':'), Some('\\' | '/')) if drive.is_ascii_alphabetic()
    )
}

fn project_from_sublime_title(title: &str) -> Option<String> {
    let title = cleaned_title(title);
    let title = strip_app_suffix(&title, &["Sublime Text"]);
    let close = title.rfind(')')?;
    let before_close = &title[..close];
    let open = before_close.rfind('(')?;
    non_empty_project(&before_close[open + 1..])
}

fn project_from_office_title(title: &str) -> Option<String> {
    project_from_document_title(
        title,
        &[
            "Word",
            "Excel",
            "PowerPoint",
            "Microsoft Word",
            "Microsoft Excel",
            "Microsoft PowerPoint",
        ],
        &[
            "Book1",
            "Document1",
            "Presentation1",
            "ブック1",
            "文書1",
            "プレゼンテーション1",
        ],
    )
}

fn project_from_iwork_title(title: &str) -> Option<String> {
    project_from_document_title(
        title,
        &["Numbers", "Numbers Creator Studio", "Pages", "Keynote"],
        &["Untitled", "名称未設定"],
    )
}

fn project_from_document_title(
    title: &str,
    app_names: &[&str],
    untitled: &[&str],
) -> Option<String> {
    let title = cleaned_title(title);
    let title = strip_app_suffix(&title, app_names);
    let title = title
        .strip_prefix("Unsaved - ")
        .or_else(|| title.strip_prefix("未保存 - "))
        .unwrap_or(title)
        .trim();
    let name = strip_known_extension(title);
    if untitled
        .iter()
        .any(|candidate| name.eq_ignore_ascii_case(candidate))
        || app_names
            .iter()
            .any(|candidate| name.eq_ignore_ascii_case(candidate))
    {
        return None;
    }
    non_empty_project(name)
}

fn project_from_adobe_raster_title(title: &str) -> Option<String> {
    let title = cleaned_title(title);
    let (name, _) = title.split_once(" @ ")?;
    non_empty_project(strip_known_extension(name))
}

fn project_from_adobe_xd_title(title: &str) -> Option<String> {
    project_from_document_title(title, &["Adobe XD"], &["Untitled", "名称未設定"])
}

pub fn capture() -> Result<Sample> {
    capture_with_settings(&Settings::default())
}

pub fn capture_with_settings(settings: &Settings) -> Result<Sample> {
    let now = Utc::now();
    debug_capture("checking idle state");
    if mac::idle_seconds() >= f64::from(settings.idle_minutes) * 60.0 {
        return Ok(Sample {
            id: 0,
            timestamp: now,
            interval_minutes: Some(settings.interval_minutes),
            app: String::new(),
            title: String::new(),
            project: None,
            source: "idle".into(),
            status: "idle".into(),
        });
    }
    debug_capture("getting active window");
    let window = platform_active_window()?;
    debug_capture("checking accessibility trust");
    let trusted = mac::accessibility_trusted(false);
    debug_capture("getting accessibility title");
    let ax_title = if trusted {
        mac::title(window.process_id)
    } else {
        None
    };
    debug_capture("building sample");
    let source = if ax_title.is_some() {
        "accessibility"
    } else if trusted {
        "window_title"
    } else {
        "accessibility_denied"
    };
    let title = ax_title.unwrap_or(window.title);
    let status = if !trusted && title.is_empty() {
        "permission_required"
    } else if title.is_empty() {
        "title_unavailable"
    } else {
        "captured"
    };
    Ok(Sample {
        id: 0,
        timestamp: now,
        interval_minutes: Some(settings.interval_minutes),
        project: project_from_title(&window.app, &title),
        app: window.app,
        title,
        source: source.into(),
        status: status.into(),
    })
}

fn explicit_interval_seconds(interval_minutes: u32) -> i64 {
    i64::from(interval_minutes.max(1)) * 60
}

fn estimated_interval_seconds(samples: &[Sample], fallback_interval_minutes: u32) -> Vec<i64> {
    let fallback_seconds = explicit_interval_seconds(fallback_interval_minutes);
    samples
        .iter()
        .enumerate()
        .map(|(index, sample)| {
            if let Some(interval_minutes) = sample.interval_minutes {
                return explicit_interval_seconds(interval_minutes);
            }

            let previous_gap = index
                .checked_sub(1)
                .map(|previous_index| sample.timestamp - samples[previous_index].timestamp)
                .map(|gap| gap.num_seconds())
                .filter(|&seconds| seconds > 0);
            let next_gap = samples
                .get(index + 1)
                .map(|next| next.timestamp - sample.timestamp)
                .map(|gap| gap.num_seconds())
                .filter(|&seconds| seconds > 0);

            match [previous_gap, next_gap].into_iter().flatten().min() {
                Some(seconds) if seconds <= 60 * 60 => {
                    let minutes = ((seconds + 30) / 60).clamp(1, 60);
                    minutes * 60
                }
                _ => fallback_seconds,
            }
        })
        .collect()
}

pub fn group_samples(
    samples: &[Sample],
    fallback_interval_minutes: u32,
    now: DateTime<Utc>,
) -> Vec<Block> {
    let intervals = estimated_interval_seconds(samples, fallback_interval_minutes);
    let mut runs = Vec::new();
    let mut start = 0;
    for index in 1..samples.len() {
        let prev = &samples[index - 1];
        let next = &samples[index];
        let prev_interval = intervals[index - 1];
        let threshold = chrono::Duration::seconds(prev_interval * 3 / 2);
        let same_record =
            prev.status == next.status && prev.app == next.app && prev.title == next.title;
        if !same_record || next.timestamp - prev.timestamp > threshold {
            runs.push((start, index));
            start = index;
        }
    }
    if !samples.is_empty() {
        runs.push((start, samples.len()));
    }

    runs.iter()
        .enumerate()
        .map(|(run_index, &(start_index, end_index))| {
            let first = &samples[start_index];
            let last = &samples[end_index - 1];
            let interval_end = last.timestamp + chrono::Duration::seconds(intervals[end_index - 1]);
            let next_start = runs
                .get(run_index + 1)
                .map(|&(next_index, _)| samples[next_index].timestamp);
            let end = [Some(interval_end), next_start, Some(now)]
                .into_iter()
                .flatten()
                .min()
                .unwrap_or(first.timestamp)
                .max(first.timestamp);
            Block {
                start: first.timestamp,
                end,
                seconds: (end - first.timestamp).num_seconds().max(0),
                app: first.app.clone(),
                title: first.title.clone(),
                project: first.project.clone(),
                source: first.source.clone(),
                status: first.status.clone(),
                sample_count: end_index - start_index,
            }
        })
        .collect()
}

pub fn day_report(blocks: &[Block]) -> DayReport {
    let mut idle_seconds = 0;
    let mut projects = std::collections::BTreeMap::<
        Option<String>,
        std::collections::BTreeMap<(String, String), i64>,
    >::new();
    for block in blocks {
        if block.status == "idle" {
            idle_seconds += block.seconds;
            continue;
        }
        *projects
            .entry(block.project.clone())
            .or_default()
            .entry((block.app.clone(), block.title.clone()))
            .or_default() += block.seconds;
    }
    let mut projects: Vec<_> = projects
        .into_iter()
        .map(|(project, entries)| {
            let mut entries: Vec<_> = entries
                .into_iter()
                .map(|((app, title), seconds)| ReportEntry {
                    app,
                    title,
                    seconds,
                })
                .collect();
            entries.sort_by(|a, b| {
                b.seconds
                    .cmp(&a.seconds)
                    .then_with(|| a.app.cmp(&b.app))
                    .then_with(|| a.title.cmp(&b.title))
            });
            ProjectReport {
                project,
                seconds: entries.iter().map(|entry| entry.seconds).sum(),
                entries,
            }
        })
        .collect();
    projects.sort_by(|a, b| {
        b.seconds.cmp(&a.seconds).then_with(|| {
            a.project
                .as_deref()
                .unwrap_or("未分類")
                .cmp(b.project.as_deref().unwrap_or("未分類"))
        })
    });
    DayReport {
        work_seconds: projects.iter().map(|project| project.seconds).sum(),
        idle_seconds,
        projects,
    }
}

fn debug_capture(message: &str) {
    if std::env::var_os("WORKLOG_DEBUG_CAPTURE").is_some() {
        eprintln!("worklog capture: {message}");
    }
}

struct ActiveWindow {
    app: String,
    title: String,
    process_id: i32,
}

fn platform_active_window() -> Result<ActiveWindow> {
    #[cfg(target_os = "macos")]
    {
        mac::active_window().ok_or_else(|| "前面ウィンドウを取得できません。".to_string())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let window = active_win_pos_rs::get_active_window()
            .map_err(|_| "前面ウィンドウを取得できません。".to_string())?;
        Ok(ActiveWindow {
            app: window.app_name,
            title: window.title,
            process_id: window.process_id as i32,
        })
    }
}

/// Returns whether this process is allowed to use the macOS Accessibility API.
/// Passing `prompt = true` asks macOS to register and prompt for this exact
/// executable. Other platforms do not require this permission.
pub fn accessibility_trusted(prompt: bool) -> bool {
    mac::accessibility_trusted(prompt)
}

pub struct Store(Connection);
impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let conn = Connection::open(path).map_err(|e| e.to_string())?;
        conn.busy_timeout(std::time::Duration::from_secs(5))
            .map_err(|e| e.to_string())?;
        conn.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE IF NOT EXISTS samples (id INTEGER PRIMARY KEY, timestamp TEXT NOT NULL, data TEXT NOT NULL); CREATE INDEX IF NOT EXISTS samples_timestamp ON samples(timestamp); CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL); CREATE TABLE IF NOT EXISTS rules (id INTEGER PRIMARY KEY, app TEXT NOT NULL, contains TEXT NOT NULL, project TEXT NOT NULL);").map_err(|e| e.to_string())?;
        Ok(Self(conn))
    }
    pub fn settings(&self) -> Result<Settings> {
        use rusqlite::OptionalExtension;
        let json: Option<String> = self
            .0
            .query_row(
                "SELECT value FROM settings WHERE key='recorder'",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        match json {
            Some(json) => serde_json::from_str::<Settings>(&json)
                .map_err(|e| e.to_string())?
                .validated(),
            None => Ok(Settings::default()),
        }
    }
    pub fn save_settings(&self, settings: Settings) -> Result<Settings> {
        let settings = settings.validated()?;
        let json = serde_json::to_string(&settings).map_err(|e| e.to_string())?;
        self.0.execute("INSERT INTO settings(key,value) VALUES ('recorder',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value", [json]).map_err(|e| e.to_string())?;
        Ok(settings)
    }
    pub fn append(&self, sample: &Sample) -> Result<()> {
        self.0
            .execute(
                "INSERT INTO samples(timestamp,data) VALUES (?1,?2)",
                params![
                    sample.timestamp.to_rfc3339(),
                    serde_json::to_string(sample).map_err(|e| e.to_string())?
                ],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    pub fn day(&self, date: &str) -> Result<Vec<Sample>> {
        use chrono::TimeZone;
        let parsed =
            chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").map_err(|e| e.to_string())?;
        let next = parsed.succ_opt().ok_or("日付が範囲外です")?;
        let boundary = |d: chrono::NaiveDate| {
            Local
                .from_local_datetime(&d.and_hms_opt(0, 0, 0).unwrap())
                .earliest()
                .map(|t| t.with_timezone(&Utc).to_rfc3339())
                .ok_or("このタイムゾーンの日付境界を解決できません".to_string())
        };
        let start = boundary(parsed)?;
        let end = boundary(next)?;
        let rules = self.rules()?;
        let mut stmt = self.0.prepare("SELECT id,data FROM samples WHERE timestamp >= ?1 AND timestamp < ?2 ORDER BY timestamp,id").map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![start, end], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for row in rows {
            let (id, data) = row.map_err(|e| e.to_string())?;
            let mut sample: Sample = serde_json::from_str(&data).map_err(|e| e.to_string())?;
            if sample
                .timestamp
                .with_timezone(&Local)
                .format("%Y-%m-%d")
                .to_string()
                == date
            {
                sample.id = id;
                if sample.status != "idle" {
                    sample.project = project_from_title(&sample.app, &sample.title);
                    if let Some(rule) = rules
                        .iter()
                        .find(|r| sample.app == r.app && sample.title.contains(&r.contains))
                    {
                        sample.project = Some(rule.project.clone());
                        sample.source = "rule".into();
                    }
                }
                out.push(sample);
            }
        }
        Ok(out)
    }
    pub fn rules(&self) -> Result<Vec<Rule>> {
        let mut stmt = self
            .0
            .prepare("SELECT id,app,contains,project FROM rules ORDER BY id DESC")
            .map_err(|e| e.to_string())?;
        stmt.query_map([], |r| {
            Ok(Rule {
                id: r.get(0)?,
                app: r.get(1)?,
                contains: r.get(2)?,
                project: r.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
    }
    pub fn add_rule(&self, app: &str, contains: &str, project: &str) -> Result<()> {
        if app.trim().is_empty() || contains.trim().is_empty() || project.trim().is_empty() {
            return Err("アプリ名・タイトルの条件・プロジェクト名を入力してください。".into());
        }
        self.0
            .execute(
                "INSERT INTO rules(app,contains,project) VALUES (?1,?2,?3)",
                params![app, contains.trim(), project.trim()],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    pub fn delete_rule(&self, id: i64) -> Result<()> {
        self.0
            .execute("DELETE FROM rules WHERE id=?1", [id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    pub fn paused(&self) -> Result<bool> {
        use rusqlite::OptionalExtension;
        Ok(self
            .0
            .query_row("SELECT value FROM settings WHERE key='paused'", [], |row| {
                row.get::<_, String>(0)
            })
            .optional()
            .map_err(|e| e.to_string())?
            .as_deref()
            == Some("true"))
    }
    pub fn set_paused(&self, paused: bool) -> Result<()> {
        self.0.execute("INSERT INTO settings(key,value) VALUES ('paused',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value", [paused.to_string()]).map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[cfg(target_os = "macos")]
mod mac {
    use std::{
        ffi::{CString, c_char, c_void},
        ptr,
    };
    type CF = *const c_void;
    #[link(name = "ApplicationServices", kind = "framework")]
    unsafe extern "C" {
        static kAXTrustedCheckOptionPrompt: CF;
        fn AXUIElementCreateApplication(pid: i32) -> CF;
        fn AXUIElementCopyAttributeValue(element: CF, attribute: CF, value: *mut CF) -> i32;
        fn AXUIElementSetMessagingTimeout(element: CF, timeout: f32) -> i32;
        fn AXIsProcessTrustedWithOptions(options: CF) -> u8;
    }
    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        static kCFBooleanTrue: CF;
        fn CFDictionaryCreate(
            allocator: CF,
            keys: *const CF,
            values: *const CF,
            count: isize,
            key_callbacks: CF,
            value_callbacks: CF,
        ) -> CF;
        fn CFStringCreateWithCString(allocator: CF, text: *const c_char, encoding: u32) -> CF;
        fn CFStringGetCString(string: CF, buffer: *mut c_char, size: isize, encoding: u32) -> bool;
        fn CFArrayGetCount(array: CF) -> isize;
        fn CFArrayGetTypeID() -> usize;
        fn CFArrayGetValueAtIndex(array: CF, idx: isize) -> CF;
        fn CFBooleanGetTypeID() -> usize;
        fn CFBooleanGetValue(boolean: CF) -> bool;
        fn CFDictionaryGetTypeID() -> usize;
        fn CFDictionaryGetValueIfPresent(dictionary: CF, key: CF, value: *mut CF) -> bool;
        fn CFNumberGetTypeID() -> usize;
        fn CFNumberGetValue(number: CF, number_type: i32, value: *mut c_void) -> bool;
        fn CFGetTypeID(value: CF) -> usize;
        fn CFStringGetTypeID() -> usize;
        fn CFRelease(value: CF);
    }
    #[link(name = "IOKit", kind = "framework")]
    unsafe extern "C" {
        fn IOServiceMatching(name: *const c_char) -> CF;
        fn IOServiceGetMatchingService(master_port: u32, matching: CF) -> u32;
        fn IORegistryEntryCreateCFProperty(entry: u32, key: CF, allocator: CF, options: u32) -> CF;
        fn IOObjectRelease(object: u32) -> i32;
    }
    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGWindowListCopyWindowInfo(option: u32, relative_to_window: u32) -> CF;
    }
    const UTF8: u32 = 0x08000100;
    const K_CF_NUMBER_SINT64_TYPE: i32 = 4;
    const K_CG_WINDOW_LIST_OPTION_ON_SCREEN_ONLY: u32 = 1;
    pub fn accessibility_trusted(prompt: bool) -> bool {
        unsafe {
            if !prompt {
                return AXIsProcessTrustedWithOptions(ptr::null()) != 0;
            }
            let keys = [kAXTrustedCheckOptionPrompt];
            let values = [kCFBooleanTrue];
            let options = CFDictionaryCreate(
                ptr::null(),
                keys.as_ptr(),
                values.as_ptr(),
                1,
                ptr::null(),
                ptr::null(),
            );
            if options.is_null() {
                return false;
            }
            let trusted = AXIsProcessTrustedWithOptions(options) != 0;
            CFRelease(options);
            trusted
        }
    }
    unsafe fn attribute(element: CF, key: &str) -> Option<CF> {
        let key = CString::new(key).ok()?;
        unsafe {
            let name = CFStringCreateWithCString(ptr::null(), key.as_ptr(), UTF8);
            let mut value = ptr::null();
            let result = AXUIElementCopyAttributeValue(element, name, &mut value);
            CFRelease(name);
            if result == 0 && !value.is_null() {
                Some(value)
            } else {
                None
            }
        }
    }
    unsafe fn string_value(value: CF) -> Option<String> {
        unsafe {
            let mut buffer = vec![0u8; 32768];
            let valid = CFGetTypeID(value) == CFStringGetTypeID()
                && CFStringGetCString(
                    value,
                    buffer.as_mut_ptr().cast(),
                    buffer.len() as isize,
                    UTF8,
                );
            if !valid {
                return None;
            }
            let end = buffer.iter().position(|&b| b == 0)?;
            String::from_utf8(buffer[..end].to_vec())
                .ok()
                .filter(|s| !s.is_empty())
        }
    }
    unsafe fn title_attribute(element: CF) -> Option<String> {
        unsafe {
            let title = attribute(element, "AXTitle")?;
            let value = string_value(title);
            CFRelease(title);
            value
        }
    }
    unsafe fn number_value(value: CF) -> Option<i64> {
        unsafe {
            if CFGetTypeID(value) != CFNumberGetTypeID() {
                return None;
            }
            let mut out = 0_i64;
            CFNumberGetValue(
                value,
                K_CF_NUMBER_SINT64_TYPE,
                (&mut out as *mut i64).cast(),
            )
            .then_some(out)
        }
    }
    unsafe fn dictionary_value(dictionary: CF, key: CF) -> Option<CF> {
        unsafe {
            if CFGetTypeID(dictionary) != CFDictionaryGetTypeID() {
                return None;
            }
            let mut value = ptr::null();
            CFDictionaryGetValueIfPresent(dictionary, key, &mut value)
                .then_some(value)
                .filter(|value| !value.is_null())
        }
    }
    unsafe fn cf_string(text: &str) -> Option<CF> {
        let text = CString::new(text).ok()?;
        unsafe {
            let value = CFStringCreateWithCString(ptr::null(), text.as_ptr(), UTF8);
            (!value.is_null()).then_some(value)
        }
    }
    pub fn active_window() -> Option<super::ActiveWindow> {
        unsafe {
            let windows = CGWindowListCopyWindowInfo(K_CG_WINDOW_LIST_OPTION_ON_SCREEN_ONLY, 0);
            if windows.is_null() || CFGetTypeID(windows) != CFArrayGetTypeID() {
                if !windows.is_null() {
                    CFRelease(windows);
                }
                return None;
            }
            if std::env::var_os("WORKLOG_DEBUG_CAPTURE").is_some() {
                eprintln!(
                    "worklog capture: coregraphics windows={}",
                    CFArrayGetCount(windows)
                );
            }
            let owner_pid_key = cf_string("kCGWindowOwnerPID")?;
            let owner_name_key = cf_string("kCGWindowOwnerName")?;
            let window_name_key = cf_string("kCGWindowName")?;
            let layer_key = cf_string("kCGWindowLayer")?;
            let mut out = None;
            for i in 0..CFArrayGetCount(windows) {
                let window = CFArrayGetValueAtIndex(windows, i);
                if window.is_null() {
                    continue;
                }
                let layer = dictionary_value(window, layer_key).and_then(|v| number_value(v));
                if layer != Some(0) {
                    continue;
                }
                let app = dictionary_value(window, owner_name_key).and_then(|v| string_value(v));
                let process_id =
                    dictionary_value(window, owner_pid_key).and_then(|v| number_value(v));
                let Some(app) = app else {
                    continue;
                };
                let Some(process_id) = process_id else {
                    continue;
                };
                let title = dictionary_value(window, window_name_key).and_then(|v| string_value(v));
                if std::env::var_os("WORKLOG_DEBUG_CAPTURE").is_some() {
                    eprintln!(
                        "worklog capture: coregraphics candidate app={app} pid={process_id} title={:?}",
                        title
                    );
                }
                out = Some(super::ActiveWindow {
                    app,
                    title: title.unwrap_or_default(),
                    process_id: process_id as i32,
                });
                break;
            }
            CFRelease(owner_pid_key);
            CFRelease(owner_name_key);
            CFRelease(window_name_key);
            CFRelease(layer_key);
            CFRelease(windows);
            out
        }
    }
    unsafe fn bool_attribute(element: CF, key: &str) -> Option<bool> {
        unsafe {
            let value = attribute(element, key)?;
            let valid = CFGetTypeID(value) == CFBooleanGetTypeID();
            let out = valid.then(|| CFBooleanGetValue(value));
            CFRelease(value);
            out
        }
    }
    pub fn title(pid: i32) -> Option<String> {
        unsafe {
            let app = AXUIElementCreateApplication(pid);
            if app.is_null() {
                return None;
            }
            AXUIElementSetMessagingTimeout(app, 1.0);
            let window = attribute(app, "AXFocusedWindow");
            if let Some(window) = window {
                let title = title_attribute(window);
                CFRelease(window);
                if title.is_some() {
                    CFRelease(app);
                    return title;
                }
            }
            let windows = attribute(app, "AXWindows");
            CFRelease(app);
            let windows = windows?;
            if CFGetTypeID(windows) != CFArrayGetTypeID() {
                CFRelease(windows);
                return None;
            }
            let mut first_title = None;
            for i in 0..CFArrayGetCount(windows) {
                let window = CFArrayGetValueAtIndex(windows, i);
                if window.is_null() {
                    continue;
                }
                let title = title_attribute(window);
                if first_title.is_none() {
                    first_title = title.clone();
                }
                if title.is_some()
                    && (bool_attribute(window, "AXFocused").unwrap_or(false)
                        || bool_attribute(window, "AXMain").unwrap_or(false))
                {
                    CFRelease(windows);
                    return title;
                }
            }
            CFRelease(windows);
            first_title
        }
    }
    pub fn idle_seconds() -> f64 {
        unsafe {
            let service_name = CString::new("IOHIDSystem").ok();
            let Some(service_name) = service_name else {
                return 0.0;
            };
            let matching = IOServiceMatching(service_name.as_ptr());
            if matching.is_null() {
                return 0.0;
            }
            let service = IOServiceGetMatchingService(0, matching);
            if service == 0 {
                return 0.0;
            }
            let key = CString::new("HIDIdleTime").ok();
            let Some(key) = key else {
                let _ = IOObjectRelease(service);
                return 0.0;
            };
            let key = CFStringCreateWithCString(ptr::null(), key.as_ptr(), UTF8);
            if key.is_null() {
                let _ = IOObjectRelease(service);
                return 0.0;
            }
            let value = IORegistryEntryCreateCFProperty(service, key, ptr::null(), 0);
            CFRelease(key);
            let _ = IOObjectRelease(service);
            if value.is_null() {
                return 0.0;
            }
            let mut idle_ns: i64 = 0;
            let valid = CFGetTypeID(value) == CFNumberGetTypeID()
                && CFNumberGetValue(
                    value,
                    K_CF_NUMBER_SINT64_TYPE,
                    (&mut idle_ns as *mut i64).cast(),
                );
            CFRelease(value);
            if valid && idle_ns > 0 {
                idle_ns as f64 / 1_000_000_000.0
            } else {
                0.0
            }
        }
    }
}
#[cfg(not(target_os = "macos"))]
mod mac {
    pub fn accessibility_trusted(_: bool) -> bool {
        true
    }
    pub fn title(_: i32) -> Option<String> {
        None
    }
    #[cfg(not(target_os = "windows"))]
    pub fn idle_seconds() -> f64 {
        0.0
    }
    #[cfg(target_os = "windows")]
    pub fn idle_seconds() -> f64 {
        #[repr(C)]
        struct LastInputInfo {
            size: u32,
            time: u32,
        }
        #[link(name = "user32")]
        unsafe extern "system" {
            fn GetLastInputInfo(info: *mut LastInputInfo) -> i32;
        }
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GetTickCount() -> u32;
        }
        let mut info = LastInputInfo {
            size: std::mem::size_of::<LastInputInfo>() as u32,
            time: 0,
        };
        unsafe {
            if GetLastInputInfo(&mut info) != 0 {
                f64::from(GetTickCount().wrapping_sub(info.time)) / 1000.0
            } else {
                0.0
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(hour: u32, minute: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 9, 13, hour, minute, 0).unwrap()
    }

    fn sample_at(hour: u32, minute: u32, app: &str, title: &str) -> Sample {
        Sample {
            id: 0,
            timestamp: at(hour, minute),
            interval_minutes: Some(5),
            app: app.into(),
            title: title.into(),
            project: Some("Project A".into()),
            source: "title".into(),
            status: "captured".into(),
        }
    }

    fn old_sample_at(hour: u32, minute: u32, app: &str, title: &str) -> Sample {
        Sample {
            interval_minutes: None,
            ..sample_at(hour, minute, app, title)
        }
    }

    #[test]
    fn settings_roundtrip_and_validation() {
        let store = Store::open(Path::new(":memory:")).unwrap();
        assert_eq!(store.settings().unwrap(), Settings::default());
        let updated = store
            .save_settings(Settings {
                interval_minutes: 1,
                idle_minutes: 120,
                excluded_apps: vec![" Firefox ".into(), "firefox".into(), "".into()],
            })
            .unwrap();
        assert_eq!(updated.excluded_apps, vec!["Firefox"]);
        assert_eq!(store.settings().unwrap(), updated);
        assert!(updated.excludes("FIREFOX"));
        assert!(!updated.excludes("Firefox Developer Edition"));
        assert!(
            store
                .save_settings(Settings {
                    interval_minutes: 0,
                    ..updated.clone()
                })
                .is_err()
        );
        assert!(
            store
                .save_settings(Settings {
                    idle_minutes: 121,
                    ..updated.clone()
                })
                .is_err()
        );
        assert_eq!(store.settings().unwrap(), updated);
    }
    #[test]
    fn project_evidence_vscode_like_titles() {
        assert_eq!(
            project_from_title("Code", "main.rs - my-project - Visual Studio Code").as_deref(),
            Some("my-project")
        );
        assert_eq!(
            project_from_title("Code", "main.rs — workspace").as_deref(),
            Some("workspace")
        );
        assert_eq!(
            project_from_title(
                "Visual Studio Code",
                "main.rs - my-project - Visual Studio Code"
            )
            .as_deref(),
            Some("my-project")
        );
        assert_eq!(
            project_from_title(
                "Visual Studio Code - Insiders",
                "main.rs - my-project - Visual Studio Code - Insiders"
            )
            .as_deref(),
            Some("my-project")
        );
        assert_eq!(
            project_from_title("Cursor", "● main.rs — workspace [SSH: host]").as_deref(),
            Some("workspace")
        );
        assert_eq!(
            project_from_title("Windsurf", "main.rs - worklog - Windsurf").as_deref(),
            Some("worklog")
        );
        assert_eq!(project_from_title("Code", "brian"), None);
        assert_eq!(project_from_title("Code", "Welcome"), None);
        assert_eq!(
            project_from_title("Code", "main.rs - worklog - Firefox"),
            None
        );
    }

    #[test]
    fn project_evidence_does_not_partially_match_vscode_apps() {
        assert_eq!(
            project_from_title("Xcode", "MyApp — AppDelegate.swift"),
            Some("MyApp".into())
        );
        assert_eq!(
            project_from_title("Codex", "main.rs - my-project - Visual Studio Code"),
            None
        );
        assert_eq!(
            project_from_title("Visual Studio", "main.rs - my-project - Visual Studio Code"),
            None
        );
    }

    #[test]
    fn project_evidence_zed_titles() {
        assert_eq!(
            project_from_title("Zed", "worklog — src/lib.rs").as_deref(),
            Some("worklog")
        );
        assert_eq!(
            project_from_title("Zed Preview", "worklog — src/lib.rs").as_deref(),
            Some("worklog")
        );
        assert_eq!(project_from_title("Zed", "empty project — untitled"), None);
        assert_eq!(project_from_title("Zed", "worklog"), None);
        assert_eq!(project_from_title("Zed", "Settings"), None);
    }

    #[test]
    fn project_evidence_jetbrains_titles() {
        assert_eq!(
            project_from_title("IntelliJ IDEA", "worklog – lib.rs").as_deref(),
            Some("worklog")
        );
        assert_eq!(
            project_from_title(
                "IntelliJ IDEA CE",
                "worklog [~/code/worklog] – lib.rs – IntelliJ IDEA"
            )
            .as_deref(),
            Some("worklog")
        );
        assert_eq!(
            project_from_title("PyCharm Community Edition", "api – main.py").as_deref(),
            Some("api")
        );
        assert_eq!(
            project_from_title("Android Studio", "MobileApp – MainActivity.kt").as_deref(),
            Some("MobileApp")
        );
        assert_eq!(
            project_from_title("IntelliJ IDEA", "Welcome to IntelliJ IDEA"),
            None
        );
        assert_eq!(project_from_title("WebStorm", "Settings"), None);
    }

    #[test]
    fn project_evidence_xcode_titles() {
        assert_eq!(
            project_from_title("Xcode", "MyApp — AppDelegate.swift").as_deref(),
            Some("MyApp")
        );
        assert_eq!(
            project_from_title("Xcode", "MyApp.xcworkspace — ContentView.swift").as_deref(),
            Some("MyApp")
        );
        assert_eq!(project_from_title("Xcode", "Welcome to Xcode"), None);
        assert_eq!(project_from_title("Xcode", "Organizer"), None);
    }

    #[test]
    fn project_evidence_obsidian_titles() {
        assert_eq!(
            project_from_title("Obsidian", "Daily - note - Personal - Obsidian v1.8.0").as_deref(),
            Some("Personal")
        );
        assert_eq!(
            project_from_title("Obsidian", "Note - Vault - Obsidian").as_deref(),
            Some("Vault")
        );
        assert_eq!(project_from_title("Obsidian", "Vault"), None);
        assert_eq!(project_from_title("Obsidian", "Settings - Obsidian"), None);
    }

    #[test]
    fn project_evidence_terminal_titles() {
        assert_eq!(
            project_from_title("Terminal", "worklog:/Users/me/日本語 — zsh").as_deref(),
            Some("日本語")
        );
        assert_eq!(
            project_from_title("iTerm2", "1. zsh  worklog:/Users/me/worklog — zsh — 80×24")
                .as_deref(),
            Some("worklog")
        );
        assert_eq!(
            project_from_title(
                "Windows Terminal",
                "pwsh worklog:C:\\Users\\me\\project — pwsh"
            )
            .as_deref(),
            Some("project")
        );
        assert_eq!(project_from_title("Terminal", "zsh — 80×24"), None);
        assert_eq!(
            project_from_title(
                "Firefox",
                "yutasato/worklog: Mac・Windowsで前面のアプリを記録"
            ),
            None
        );
        assert_eq!(
            project_from_title("Google Chrome", "GitHub - yutasato/worklog: 作業日誌"),
            None
        );
        assert_eq!(
            project_from_title("Terminal", "xworklog:/Users/me/project"),
            None
        );
        assert_eq!(project_from_title("Terminal", "worklog:project"), None);
    }

    #[test]
    fn project_evidence_figma_titles() {
        assert_eq!(
            project_from_title("Figma", "Product / Home – Figma").as_deref(),
            Some("Product / Home")
        );
        assert_eq!(project_from_title("Figma", "Figma"), None);
        assert_eq!(
            project_from_title("Firefox", "Product / Home – Figma"),
            None
        );
    }

    #[test]
    fn project_evidence_other_document_apps() {
        assert_eq!(
            project_from_title("Sublime Text", "/repo/src/lib.rs (worklog) - Sublime Text")
                .as_deref(),
            Some("worklog")
        );
        assert_eq!(
            project_from_title("Sublime Text", "src/lib.rs - Sublime Text"),
            None
        );
        assert_eq!(
            project_from_title("Microsoft Excel", "Budget.xlsx - Excel").as_deref(),
            Some("Budget")
        );
        assert_eq!(
            project_from_title("Microsoft Excel", "Microsoft Excel"),
            None
        );
        assert_eq!(project_from_title("Microsoft Excel", "Excel"), None);
        assert_eq!(project_from_title("Microsoft Excel", "Book1 - Excel"), None);
        assert_eq!(
            project_from_title("Numbers Creator Studio", "Revenue.numbers").as_deref(),
            Some("Revenue")
        );
        assert_eq!(project_from_title("Numbers", "Numbers"), None);
        assert_eq!(project_from_title("Numbers", "名称未設定"), None);
        assert_eq!(
            project_from_title("Sketch", "Landing.sketch").as_deref(),
            Some("Landing")
        );
        assert_eq!(project_from_title("Sketch", "Sketch"), None);
        assert_eq!(project_from_title("Sketch", "Untitled"), None);
        assert_eq!(
            project_from_title("Adobe Photoshop 2025", "Mockup.psd @ 100% (RGB/8)").as_deref(),
            Some("Mockup")
        );
        assert_eq!(
            project_from_title("Adobe Photoshop 2025", "Adobe Photoshop 2025"),
            None
        );
        assert_eq!(project_from_title("Adobe Photoshop 2025", "ホーム"), None);
        assert_eq!(
            project_from_title("Adobe Illustrator 2025", "Logo.ai @ 66.7%").as_deref(),
            Some("Logo")
        );
        assert_eq!(
            project_from_title("Adobe Illustrator 2025", "Adobe Illustrator 2025"),
            None
        );
        assert_eq!(
            project_from_title("Adobe XD", "Prototype.xd – Adobe XD").as_deref(),
            Some("Prototype")
        );
        assert_eq!(project_from_title("Adobe XD", "Adobe XD"), None);
    }
    #[test]
    fn history_rules_and_pause() {
        let store = Store::open(Path::new(":memory:")).unwrap();
        let sample = Sample {
            id: 0,
            timestamp: Utc::now(),
            interval_minutes: Some(5),
            app: "Any App".into(),
            title: "<script>日本語</script>".into(),
            project: None,
            source: "window_title".into(),
            status: "captured".into(),
        };
        store.append(&sample).unwrap();
        store.append(&sample).unwrap();
        let date = Local::now().format("%Y-%m-%d").to_string();
        assert_eq!(store.day(&date).unwrap().len(), 2);
        assert!(store.day("invalid").is_err());
        store.add_rule("Any App", "日本語", "Project A").unwrap();
        assert_eq!(
            store.day(&date).unwrap()[0].project.as_deref(),
            Some("Project A")
        );
        store.add_rule("Other App", "日本語", "Wrong").unwrap();
        assert_eq!(
            store.day(&date).unwrap()[0].project.as_deref(),
            Some("Project A")
        );
        assert!(store.add_rule("Any App", "", "Wrong").is_err());
        store.delete_rule(store.rules().unwrap()[1].id).unwrap();
        assert!(store.day(&date).unwrap()[0].project.is_none());
        store.set_paused(true).unwrap();
        assert!(store.paused().unwrap());
        store.set_paused(false).unwrap();
        assert!(!store.paused().unwrap());
    }

    #[test]
    fn day_recomputes_saved_title_project_before_rules() {
        let store = Store::open(Path::new(":memory:")).unwrap();
        let mut sample = sample_at(10, 0, "Code", "ARCHITECTURE.md — fetch-focused-window");
        sample.project = Some("ARCHITECTURE.md".into());
        sample.source = "accessibility".into();
        store.append(&sample).unwrap();
        let date = sample
            .timestamp
            .with_timezone(&Local)
            .format("%Y-%m-%d")
            .to_string();

        let samples = store.day(&date).unwrap();

        assert_eq!(samples[0].project.as_deref(), Some("fetch-focused-window"));
        assert_eq!(samples[0].source, "accessibility");
    }

    #[test]
    fn day_prefers_rules_over_recomputed_title_project() {
        let store = Store::open(Path::new(":memory:")).unwrap();
        let sample = sample_at(10, 0, "Code", "ARCHITECTURE.md — fetch-focused-window");
        store.append(&sample).unwrap();
        store
            .add_rule("Code", "ARCHITECTURE.md", "Documentation")
            .unwrap();
        let date = sample
            .timestamp
            .with_timezone(&Local)
            .format("%Y-%m-%d")
            .to_string();

        let samples = store.day(&date).unwrap();

        assert_eq!(samples[0].project.as_deref(), Some("Documentation"));
        assert_eq!(samples[0].source, "rule");
    }

    #[test]
    fn day_recomputes_unknown_titles_to_none() {
        let store = Store::open(Path::new(":memory:")).unwrap();
        let mut sample = sample_at(10, 0, "Code", "brian");
        sample.project = Some("brian".into());
        sample.source = "accessibility".into();
        store.append(&sample).unwrap();
        let date = sample
            .timestamp
            .with_timezone(&Local)
            .format("%Y-%m-%d")
            .to_string();

        let samples = store.day(&date).unwrap();

        assert_eq!(samples[0].project, None);
        assert_eq!(samples[0].source, "accessibility");
    }

    #[test]
    fn group_samples_merges_continuous_same_records_until_next_record() {
        let mut samples = vec![
            sample_at(10, 0, "Code", "main.rs"),
            sample_at(10, 5, "Code", "main.rs"),
            sample_at(10, 10, "Code", "main.rs"),
            sample_at(10, 15, "Code", "main.rs"),
            sample_at(10, 20, "Code", "main.rs"),
            sample_at(10, 25, "Code", "main.rs"),
            sample_at(10, 30, "Firefox", "Docs"),
        ];
        samples[6].project = Some("Project B".into());

        let blocks = group_samples(&samples, 5, at(11, 0));

        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].start, at(10, 0));
        assert_eq!(blocks[0].end, at(10, 30));
        assert_eq!(blocks[0].seconds, 1800);
        assert_eq!(blocks[0].sample_count, 6);
    }

    #[test]
    fn group_samples_splits_when_title_changes() {
        let samples = vec![
            sample_at(10, 0, "Code", "main.rs"),
            sample_at(10, 5, "Code", "lib.rs"),
        ];

        let blocks = group_samples(&samples, 5, at(11, 0));

        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].title, "main.rs");
        assert_eq!(blocks[0].end, at(10, 5));
        assert_eq!(blocks[1].title, "lib.rs");
    }

    #[test]
    fn group_samples_splits_after_gap_larger_than_interval_grace() {
        let samples = vec![
            sample_at(10, 0, "Code", "main.rs"),
            sample_at(10, 8, "Code", "main.rs"),
        ];

        let blocks = group_samples(&samples, 5, at(11, 0));

        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].end, at(10, 5));
        assert_eq!(blocks[0].seconds, 300);
    }

    #[test]
    fn group_samples_caps_last_block_at_now() {
        let samples = vec![sample_at(10, 0, "Code", "main.rs")];

        let blocks = group_samples(&samples, 5, at(10, 3));

        assert_eq!(blocks[0].end, at(10, 3));
        assert_eq!(blocks[0].seconds, 180);
    }

    #[test]
    fn group_samples_uses_fallback_interval_for_old_records() {
        let mut sample = sample_at(10, 0, "Code", "main.rs");
        sample.interval_minutes = None;

        let blocks = group_samples(&[sample], 10, at(11, 0));

        assert_eq!(blocks[0].end, at(10, 10));
        assert_eq!(blocks[0].seconds, 600);
    }

    #[test]
    fn group_samples_estimates_old_five_minute_records_when_fallback_is_one() {
        let samples = vec![
            old_sample_at(10, 0, "Code", "main.rs"),
            old_sample_at(10, 5, "Code", "main.rs"),
            old_sample_at(10, 10, "Code", "main.rs"),
            old_sample_at(10, 15, "Code", "main.rs"),
            old_sample_at(10, 20, "Code", "main.rs"),
            old_sample_at(10, 25, "Code", "main.rs"),
            old_sample_at(10, 30, "Firefox", "Docs"),
        ];

        let blocks = group_samples(&samples, 1, at(11, 0));

        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].start, at(10, 0));
        assert_eq!(blocks[0].end, at(10, 30));
        assert_eq!(blocks[0].seconds, 1800);
        assert_eq!(blocks[0].sample_count, 6);
    }

    #[test]
    fn group_samples_estimates_different_old_record_intervals() {
        let samples = vec![
            old_sample_at(10, 0, "Code", "main.rs"),
            old_sample_at(10, 5, "Code", "main.rs"),
            old_sample_at(10, 10, "Code", "main.rs"),
            old_sample_at(10, 15, "Firefox", "Docs"),
            old_sample_at(10, 16, "Firefox", "Docs"),
            old_sample_at(10, 17, "Firefox", "Docs"),
        ];

        let blocks = group_samples(&samples, 1, at(11, 0));

        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].start, at(10, 0));
        assert_eq!(blocks[0].end, at(10, 15));
        assert_eq!(blocks[0].seconds, 900);
        assert_eq!(blocks[1].start, at(10, 15));
        assert_eq!(blocks[1].end, at(10, 18));
        assert_eq!(blocks[1].seconds, 180);
    }

    #[test]
    fn group_samples_does_not_join_old_records_across_long_gap() {
        let samples = vec![
            old_sample_at(10, 0, "Code", "main.rs"),
            old_sample_at(10, 5, "Code", "main.rs"),
            old_sample_at(10, 10, "Code", "main.rs"),
            old_sample_at(12, 16, "Code", "main.rs"),
            old_sample_at(12, 21, "Code", "main.rs"),
        ];

        let blocks = group_samples(&samples, 1, at(13, 0));

        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].start, at(10, 0));
        assert_eq!(blocks[0].end, at(10, 15));
        assert_eq!(blocks[0].seconds, 900);
        assert_eq!(blocks[1].start, at(12, 16));
    }

    #[test]
    fn group_samples_uses_fallback_for_isolated_old_record() {
        let samples = vec![
            sample_at(10, 0, "Firefox", "Docs"),
            old_sample_at(12, 10, "Code", "main.rs"),
            sample_at(14, 20, "Terminal", "zsh"),
        ];

        let blocks = group_samples(&samples, 7, at(15, 0));

        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[1].start, at(12, 10));
        assert_eq!(blocks[1].end, at(12, 17));
        assert_eq!(blocks[1].seconds, 420);
    }

    #[test]
    fn day_report_counts_idle_only_as_idle_seconds() {
        let mut samples = vec![
            sample_at(10, 0, "", ""),
            sample_at(10, 5, "", ""),
            sample_at(10, 10, "Code", "main.rs"),
        ];
        samples[0].status = "idle".into();
        samples[0].source = "idle".into();
        samples[0].project = None;
        samples[1].status = "idle".into();
        samples[1].source = "idle".into();
        samples[1].project = None;

        let blocks = group_samples(&samples, 5, at(11, 0));
        let report = day_report(&blocks);

        assert_eq!(blocks[0].status, "idle");
        assert_eq!(blocks[0].seconds, 600);
        assert_eq!(report.idle_seconds, 600);
        assert_eq!(report.work_seconds, 300);
        assert_eq!(report.projects.len(), 1);
    }

    #[test]
    fn day_report_groups_and_sorts_projects_and_entries() {
        let blocks = vec![
            Block {
                start: at(10, 0),
                end: at(10, 10),
                seconds: 600,
                app: "Code".into(),
                title: "main.rs".into(),
                project: Some("B".into()),
                source: "title".into(),
                status: "captured".into(),
                sample_count: 2,
            },
            Block {
                start: at(10, 10),
                end: at(10, 25),
                seconds: 900,
                app: "Firefox".into(),
                title: "Docs".into(),
                project: None,
                source: "title".into(),
                status: "captured".into(),
                sample_count: 3,
            },
            Block {
                start: at(10, 25),
                end: at(10, 35),
                seconds: 600,
                app: "Code".into(),
                title: "lib.rs".into(),
                project: Some("B".into()),
                source: "title".into(),
                status: "captured".into(),
                sample_count: 2,
            },
        ];

        let report = day_report(&blocks);

        assert_eq!(report.work_seconds, 2100);
        assert_eq!(report.projects[0].project.as_deref(), Some("B"));
        assert_eq!(report.projects[0].seconds, 1200);
        assert_eq!(report.projects[0].entries[0].title, "lib.rs");
        assert_eq!(report.projects[1].project, None);
        assert_eq!(report.projects[1].seconds, 900);
    }

    #[test]
    fn day_rules_are_reflected_in_grouped_blocks() {
        let store = Store::open(Path::new(":memory:")).unwrap();
        let sample = Sample {
            id: 0,
            timestamp: Utc::now(),
            interval_minutes: Some(5),
            app: "Code".into(),
            title: "main.rs".into(),
            project: None,
            source: "title".into(),
            status: "captured".into(),
        };
        store.append(&sample).unwrap();
        store.add_rule("Code", "main", "Worklog").unwrap();
        let date = Local::now().format("%Y-%m-%d").to_string();

        let samples = store.day(&date).unwrap();
        let blocks = group_samples(&samples, 5, Utc::now());

        assert_eq!(blocks[0].project.as_deref(), Some("Worklog"));
        assert_eq!(blocks[0].source, "rule");
    }
}
