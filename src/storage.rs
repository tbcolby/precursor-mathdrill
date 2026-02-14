//! PDDB storage for Math Drill.
//!
//! Dictionary: mathdrill.stats
//! Keys: best_{difficulty} — JSON with best streak, score, avg time

extern crate alloc;
use alloc::vec::Vec;

use crate::problems::Difficulty;

const DICT: &str = "mathdrill.stats";

pub struct Storage {
    pddb: pddb::Pddb,
}

/// Best stats for a difficulty level.
#[derive(Debug, Clone)]
pub struct BestStats {
    pub streak: u32,
    pub correct: u32,
    pub total: u32,
    pub avg_ms: u32,
}

impl Storage {
    pub fn new() -> Result<Self, ()> {
        let pddb = pddb::Pddb::new();
        pddb.is_mounted_blocking();
        Ok(Self { pddb })
    }

    fn read_key(&mut self, key: &str) -> Option<Vec<u8>> {
        let mut handle = self
            .pddb
            .get(DICT, key, None, false, false, None, None::<fn()>)
            .ok()?;
        let mut buf = Vec::new();
        use std::io::Read;
        handle.read_to_end(&mut buf).ok()?;
        if buf.is_empty() {
            None
        } else {
            Some(buf)
        }
    }

    fn write_key(&mut self, key: &str, data: &[u8]) {
        if let Ok(mut handle) = self.pddb.get(
            DICT, key, None, true, true, Some(data.len()), None::<fn()>,
        ) {
            use std::io::{Seek, Write};
            handle.seek(std::io::SeekFrom::Start(0)).ok();
            handle.write_all(data).ok();
            handle.set_len(data.len() as u64).ok();
        }
        self.pddb.sync().ok();
    }

    pub fn load_best(&mut self, diff: &Difficulty) -> Option<BestStats> {
        let key = alloc::format!("best_{}", diff.key());
        let buf = self.read_key(&key)?;
        let json: serde_json::Value = serde_json::from_slice(&buf).ok()?;
        Some(BestStats {
            streak: json.get("streak").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            correct: json.get("correct").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            total: json.get("total").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            avg_ms: json.get("avg_ms").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        })
    }

    pub fn save_best(&mut self, diff: &Difficulty, stats: &BestStats) {
        let key = alloc::format!("best_{}", diff.key());
        let json = serde_json::json!({
            "streak": stats.streak,
            "correct": stats.correct,
            "total": stats.total,
            "avg_ms": stats.avg_ms,
        });
        let data = serde_json::to_vec(&json).unwrap_or_default();
        self.write_key(&key, &data);
    }
}
