use crate::model::{Result, Sample, Settings};
use crate::project::project_from_title;
use chrono::Utc;

#[cfg(target_os = "macos")]
#[path = "macos.rs"]
mod platform;
#[cfg(not(target_os = "macos"))]
#[path = "other.rs"]
mod platform;

pub fn capture() -> Result<Sample> {
    capture_with_settings(&Settings::default())
}

pub fn capture_with_settings(settings: &Settings) -> Result<Sample> {
    let now = Utc::now();
    debug_capture("checking idle state");
    if platform::idle_seconds() >= f64::from(settings.idle_minutes) * 60.0 {
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
    let trusted = platform::accessibility_trusted(false);
    debug_capture("getting accessibility title");
    let ax_title = if trusted {
        platform::title(window.process_id)
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
        platform::active_window().ok_or_else(|| "前面ウィンドウを取得できません。".to_string())
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
    platform::accessibility_trusted(prompt)
}
