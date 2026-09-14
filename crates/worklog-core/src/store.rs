use crate::model::{Result, Rule, Sample, Settings};
use crate::project::project_from_title;
use chrono::{Local, Utc};
use rusqlite::{Connection, params};
use std::path::Path;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::summary::group_samples;
    use crate::test_support::sample_at;

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
