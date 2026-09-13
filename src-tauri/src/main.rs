#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use fetch_focused_window::{Rule, Sample, Settings, Store};
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};
use tauri::Manager;
use tauri_plugin_autostart::ManagerExt;

struct Recorder {
    store: Store,
    last_error: Option<String>,
    last_capture: Option<String>,
    next_capture: Instant,
}
struct AppState(Mutex<Recorder>);
#[derive(serde::Serialize)]
struct Status {
    paused: bool,
    last_error: Option<String>,
    last_capture: Option<String>,
    database: String,
    next_capture: Option<String>,
}
#[tauri::command]
fn day(date: String, state: tauri::State<AppState>) -> Result<Vec<Sample>, String> {
    state.0.lock().map_err(|e| e.to_string())?.store.day(&date)
}
#[tauri::command]
fn status(app: tauri::AppHandle, state: tauri::State<AppState>) -> Result<Status, String> {
    let recorder = state.0.lock().map_err(|e| e.to_string())?;
    Ok(Status {
        paused: recorder.store.paused()?,
        next_capture: if recorder.store.paused()? {
            None
        } else {
            Some(
                (chrono::Utc::now()
                    + chrono::Duration::from_std(
                        recorder
                            .next_capture
                            .saturating_duration_since(Instant::now()),
                    )
                    .unwrap_or_default())
                .to_rfc3339(),
            )
        },
        last_error: recorder.last_error.clone(),
        last_capture: recorder.last_capture.clone(),
        database: app
            .path()
            .app_data_dir()
            .map_err(|e| e.to_string())?
            .join("worklog.sqlite3")
            .display()
            .to_string(),
    })
}
fn update_pause(paused: bool, app: &tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let mut recorder = state.0.lock().map_err(|e| e.to_string())?;
    recorder.store.set_paused(paused)?;
    recorder.next_capture = Instant::now() + Duration::from_secs(10);
    drop(recorder);
    if let Some(menu) = app.try_state::<PauseMenu>() {
        menu.0
            .set_text(if paused {
                "記録を再開"
            } else {
                "記録を一時停止"
            })
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
#[tauri::command]
fn set_paused(paused: bool, app: tauri::AppHandle) -> Result<(), String> {
    update_pause(paused, &app)
}
#[tauri::command]
fn get_settings(state: tauri::State<AppState>) -> Result<Settings, String> {
    state.0.lock().map_err(|e| e.to_string())?.store.settings()
}
#[tauri::command]
fn save_settings(settings: Settings, state: tauri::State<AppState>) -> Result<Settings, String> {
    let mut recorder = state.0.lock().map_err(|e| e.to_string())?;
    let previous = recorder.store.settings()?;
    let settings = recorder.store.save_settings(settings)?;
    if previous.interval_minutes != settings.interval_minutes {
        recorder.next_capture =
            Instant::now() + Duration::from_secs(u64::from(settings.interval_minutes) * 60);
    }
    Ok(settings)
}
#[tauri::command]
fn get_autostart(app: tauri::AppHandle) -> Result<bool, String> {
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}
#[tauri::command]
fn set_autostart(enabled: bool, app: tauri::AppHandle) -> Result<(), String> {
    if enabled {
        app.autolaunch().enable()
    } else {
        app.autolaunch().disable()
    }
    .map_err(|e| e.to_string())
}
#[tauri::command]
fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}
struct PauseMenu(tauri::menu::MenuItem<tauri::Wry>);
fn show_main(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
#[tauri::command]
fn rules(state: tauri::State<AppState>) -> Result<Vec<Rule>, String> {
    state.0.lock().map_err(|e| e.to_string())?.store.rules()
}
#[tauri::command]
fn add_rule(
    app: String,
    contains: String,
    project: String,
    state: tauri::State<AppState>,
) -> Result<(), String> {
    state
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .store
        .add_rule(&app, &contains, &project)
}
#[tauri::command]
fn delete_rule(id: i64, state: tauri::State<AppState>) -> Result<(), String> {
    state
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .store
        .delete_rule(id)
}
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _| { if !args.iter().any(|arg| arg == "--hidden") { show_main(app); } }))
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, Some(vec!["--hidden"])))
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event { api.prevent_close(); let _ = window.hide(); }
        })
        .setup(|app| {
            let path = app.path().app_data_dir()?.join("worklog.sqlite3");
            let store = Store::open(&path).map_err(std::io::Error::other)?;
            let paused = store.paused().map_err(std::io::Error::other)?;
            app.manage(AppState(Mutex::new(Recorder { store, last_error: None, last_capture: None, next_capture: Instant::now() + Duration::from_secs(10) })));
            let show = tauri::menu::MenuItem::with_id(app, "show", "Worklogを開く", true, None::<&str>)?;
            let pause = tauri::menu::MenuItem::with_id(app, "pause", if paused { "記録を再開" } else { "記録を一時停止" }, true, None::<&str>)?;
            let quit = tauri::menu::MenuItem::with_id(app, "quit", "Worklogを終了", true, None::<&str>)?;
            let menu = tauri::menu::Menu::with_items(app, &[&show, &pause, &quit])?;
            app.manage(PauseMenu(pause));
            let mut tray = tauri::tray::TrayIconBuilder::with_id("worklog").tooltip("Worklog").menu(&menu).on_menu_event(|app, event| match event.id.as_ref() {
                "show" => show_main(app),
                "pause" => {
                    let state = app.state::<AppState>();
                    let paused = state.0.lock().ok().and_then(|r| r.store.paused().ok());
                    if let Some(paused) = paused
                        && let Err(error) = update_pause(!paused, app)
                        && let Ok(mut recorder) = state.0.lock()
                    {
                        recorder.last_error = Some(error);
                    }
                }
                "quit" => app.exit(0),
                _ => {}
            });
            if let Some(icon) = app.default_window_icon() { tray = tray.icon(icon.clone()); }
            tray.build(app)?;
            if !std::env::args().any(|arg| arg == "--hidden") { show_main(app.handle()); }
            else if let Some(window) = app.get_webview_window("main") { window.hide()?; }
            let handle = app.handle().clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(Duration::from_secs(1));
                let state = handle.state::<AppState>();
                let settings = {
                    let Ok(mut recorder) = state.0.lock() else { continue; };
                    match recorder.store.paused() { Ok(true) => continue, Err(e) => { recorder.last_error = Some(e); continue; }, Ok(false) => {} }
                    if Instant::now() < recorder.next_capture { continue; }
                    match recorder.store.settings() {
                        Ok(settings) => { recorder.next_capture = Instant::now() + Duration::from_secs(u64::from(settings.interval_minutes) * 60); settings }
                        Err(e) => { recorder.last_error = Some(e); continue; }
                    }
                };
                let captured = fetch_focused_window::capture_with_settings(&settings);
                let Ok(mut recorder) = state.0.lock() else { continue; };
                match recorder.store.paused() { Ok(true) => continue, Err(e) => { recorder.last_error = Some(e); continue; }, Ok(false) => {} }
                // Settings or a pause can change while Accessibility is responding.
                let current = match recorder.store.settings() { Ok(settings) => settings, Err(e) => { recorder.last_error = Some(e); continue; } };
                match captured {
                    Ok(sample) => {
                        if current.excludes(&sample.app) { recorder.last_error = None; continue; }
                        match recorder.store.append(&sample) {
                            Ok(()) => { recorder.last_capture = Some(sample.timestamp.to_rfc3339()); recorder.last_error = if sample.status == "title_unavailable" { Some("タイトルを取得できません。アクセシビリティの権限を確認してください。".into()) } else { None }; }
                            Err(e) => recorder.last_error = Some(e),
                        }
                    }
                    Err(e) => recorder.last_error = Some(e),
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![day, status, set_paused, rules, add_rule, delete_rule, get_settings, save_settings, get_autostart, set_autostart, quit_app])
        .run(tauri::generate_context!())
        .expect("Worklogを起動できませんでした");
}
