use chrono::{DateTime, Local, Utc};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::path::Path;
pub type Result<T> = std::result::Result<T, String>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Sample {
    pub id: i64,
    pub timestamp: DateTime<Utc>,
    pub app: String,
    pub title: String,
    pub project: Option<String>,
    pub source: String,
    pub status: String,
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
    let lower = app.to_lowercase();
    if lower.contains("code") || lower.contains("cursor") {
        let separator = if title.contains(" — ") {
            " — "
        } else {
            " - "
        };
        let parts: Vec<_> = title.split(separator).collect();
        if parts.len() >= 2
            && matches!(
                parts.last(),
                Some(&"Visual Studio Code" | &"Code" | &"Cursor")
            )
        {
            return Some(parts[parts.len() - 2].trim().to_string()).filter(|s| !s.is_empty());
        }
        // macOS commonly omits the app name: filename — workspace.
        if separator == " — " && parts.len() == 2 {
            return Some(parts[1].trim().to_string()).filter(|s| !s.is_empty());
        }
    }
    if lower.contains("figma") {
        return Some(
            title
                .strip_suffix(" – Figma")
                .or_else(|| title.strip_suffix(" - Figma"))
                .unwrap_or(title)
                .trim()
                .to_string(),
        )
        .filter(|s| !s.is_empty() && s != "Figma");
    }
    // This explicit convention works with any terminal that supports OSC titles.
    if let Some(rest) = title.strip_prefix("worklog:") {
        let path = rest
            .split(" — ")
            .next()
            .unwrap_or(rest)
            .trim()
            .trim_end_matches('/');
        return path
            .rsplit('/')
            .next()
            .map(str::to_string)
            .filter(|s| !s.is_empty());
    }
    None
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
        project: project_from_title(&window.app, &title),
        app: window.app,
        title,
        source: source.into(),
        status: status.into(),
    })
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
                if sample.status != "idle"
                    && let Some(rule) = rules
                        .iter()
                        .find(|r| sample.app == r.app && sample.title.contains(&r.contains))
                {
                    sample.project = Some(rule.project.clone());
                    sample.source = "rule".into();
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
    fn project_evidence() {
        assert_eq!(
            project_from_title("Code", "main.rs - my-project - Visual Studio Code").as_deref(),
            Some("my-project")
        );
        assert_eq!(
            project_from_title("Code", "main.rs — workspace").as_deref(),
            Some("workspace")
        );
        assert_eq!(
            project_from_title("Cursor", "main.rs — workspace — Cursor").as_deref(),
            Some("workspace")
        );
        assert_eq!(
            project_from_title("Firefox", "ChatGPT — Mozilla Firefox"),
            None
        );
        assert_eq!(
            project_from_title("Terminal", "worklog:/Users/me/日本語 — zsh").as_deref(),
            Some("日本語")
        );
        assert_eq!(project_from_title("Terminal", "zsh — 80×24"), None);
        assert_eq!(
            project_from_title("Figma", "Product / Home – Figma").as_deref(),
            Some("Product / Home")
        );
    }
    #[test]
    fn history_rules_and_pause() {
        let store = Store::open(Path::new(":memory:")).unwrap();
        let sample = Sample {
            id: 0,
            timestamp: Utc::now(),
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
}
