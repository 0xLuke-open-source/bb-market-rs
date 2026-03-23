use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};

use anyhow::Result;
use chrono::Local;
use serde::Serialize;

use super::types::ArchiveEvent;
use super::SpotTradingService;

impl SpotTradingService {
    pub(super) fn log_json<T: Serialize>(&self, file_name: &str, value: &T) -> Result<()> {
        let path = self.log_dir.join(file_name);
        let mut file = OpenOptions::new().create(true).append(true).open(path)?;
        serde_json::to_writer(&mut file, value)?;
        file.write_all(b"\n")?;
        Ok(())
    }

    pub(super) fn log_archive_events(&self, events: &[ArchiveEvent]) -> Result<()> {
        for event in events {
            let day = Local::now().format("%Y-%m-%d").to_string();
            let dir = self.log_dir.join("archive").join(day);
            fs::create_dir_all(&dir)?;
            let path = dir.join("events.jsonl");
            let mut file = OpenOptions::new().create(true).append(true).open(path)?;
            serde_json::to_writer(&mut file, event)?;
            file.write_all(b"\n")?;
        }
        Ok(())
    }

    pub(super) fn load_archive_events(&self) -> Result<Vec<ArchiveEvent>> {
        let archive_root = self.log_dir.join("archive");
        if !archive_root.exists() {
            return Ok(Vec::new());
        }

        let mut events = Vec::new();
        for day_entry in fs::read_dir(archive_root)? {
            let day_entry = day_entry?;
            if !day_entry.file_type()?.is_dir() {
                continue;
            }
            let file_path = day_entry.path().join("events.jsonl");
            if !file_path.exists() {
                continue;
            }
            let file = OpenOptions::new().read(true).open(file_path)?;
            let reader = BufReader::new(file);
            for line in reader.lines() {
                let line = line?;
                if line.trim().is_empty() {
                    continue;
                }
                if let Ok(event) = serde_json::from_str::<ArchiveEvent>(&line) {
                    events.push(event);
                }
            }
        }

        Ok(events)
    }
}
