use crate::model::Sample;
use chrono::{DateTime, TimeZone, Utc};

pub(crate) fn at(hour: u32, minute: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 9, 13, hour, minute, 0).unwrap()
}

pub(crate) fn sample_at(hour: u32, minute: u32, app: &str, title: &str) -> Sample {
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

pub(crate) fn old_sample_at(hour: u32, minute: u32, app: &str, title: &str) -> Sample {
    Sample {
        interval_minutes: None,
        ..sample_at(hour, minute, app, title)
    }
}
