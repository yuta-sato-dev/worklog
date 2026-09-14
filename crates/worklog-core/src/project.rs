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

#[cfg(test)]
mod tests {
    use super::*;

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
}
