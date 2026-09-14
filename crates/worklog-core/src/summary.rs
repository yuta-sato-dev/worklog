use crate::model::{Block, DayReport, ProjectReport, ReportEntry, Sample};
use chrono::{DateTime, Utc};

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{at, old_sample_at, sample_at};

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
}
