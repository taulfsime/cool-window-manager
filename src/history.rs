//! undo/redo history management
//!
//! provides in-memory history with debounced persistence to disk.
//! history is managed by the daemon and accessed via IPC.

use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::config;

/// stored JSON-RPC command for replay
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredCommand {
    pub method: String,
    pub params: serde_json::Value,
}

/// single history entry containing commands to restore previous state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub commands: Vec<StoredCommand>,
    pub timestamp: DateTime<Utc>,
}

/// persisted history structure
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HistoryData {
    #[serde(default)]
    pub undo: Vec<HistoryEntry>,
    #[serde(default)]
    pub redo: Vec<HistoryEntry>,
}

/// in-memory history manager with dirty tracking
pub struct History {
    data: HistoryData,
    dirty: bool,
    limit: usize,
    path: PathBuf,
}

impl History {
    /// create new empty history
    pub fn new(limit: usize, path: PathBuf) -> Self {
        Self {
            data: HistoryData::default(),
            dirty: false,
            limit,
            path,
        }
    }

    /// load history from disk, or create empty if file doesn't exist
    pub fn load(limit: usize, path: PathBuf) -> Result<Self> {
        let data = if path.exists() {
            let content = fs::read_to_string(&path)?;
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            HistoryData::default()
        };

        Ok(Self {
            data,
            dirty: false,
            limit,
            path,
        })
    }

    /// save history to disk using atomic write (write to temp, then rename)
    pub fn save(&mut self) -> Result<()> {
        if !self.dirty {
            return Ok(());
        }

        let content = serde_json::to_string_pretty(&self.data)?;

        // ensure parent directory exists
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }

        // atomic write: write to temp file, then rename
        let temp_path = self.path.with_extension("json.tmp");
        fs::write(&temp_path, &content)?;
        fs::rename(&temp_path, &self.path)?;

        self.dirty = false;
        Ok(())
    }

    /// push entry to undo stack, clearing redo stack
    pub fn push_undo(&mut self, entry: HistoryEntry) {
        self.data.undo.push(entry);
        self.data.redo.clear();

        // trim to limit
        while self.data.undo.len() > self.limit {
            self.data.undo.remove(0);
        }

        self.dirty = true;
    }

    /// pop entry from undo stack
    pub fn pop_undo(&mut self) -> Option<HistoryEntry> {
        let entry = self.data.undo.pop();
        if entry.is_some() {
            self.dirty = true;
        }
        entry
    }

    /// push entry to redo stack
    pub fn push_redo(&mut self, entry: HistoryEntry) {
        self.data.redo.push(entry);

        // trim to limit
        while self.data.redo.len() > self.limit {
            self.data.redo.remove(0);
        }

        self.dirty = true;
    }

    /// pop entry from redo stack
    pub fn pop_redo(&mut self) -> Option<HistoryEntry> {
        let entry = self.data.redo.pop();
        if entry.is_some() {
            self.dirty = true;
        }
        entry
    }

    /// clear all history
    pub fn clear(&mut self) {
        self.data.undo.clear();
        self.data.redo.clear();
        self.dirty = true;
    }

    /// check if history has been modified since last save
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// get undo stack length
    pub fn undo_len(&self) -> usize {
        self.data.undo.len()
    }

    /// get redo stack length
    pub fn redo_len(&self) -> usize {
        self.data.redo.len()
    }

    /// get reference to undo stack for listing
    pub fn undo_stack(&self) -> &[HistoryEntry] {
        &self.data.undo
    }

    /// get reference to redo stack for listing
    pub fn redo_stack(&self) -> &[HistoryEntry] {
        &self.data.redo
    }
}

/// thread-safe history manager with debounced flush
pub struct HistoryManager {
    history: Arc<Mutex<History>>,
    flush_delay: Duration,
    last_modification: Arc<Mutex<Option<Instant>>>,
}

impl HistoryManager {
    /// create new history manager, loading from disk
    pub fn new(limit: usize, flush_delay_ms: u64) -> Result<Self> {
        let path = get_history_path()?;
        let history = History::load(limit, path)?;

        Ok(Self {
            history: Arc::new(Mutex::new(history)),
            flush_delay: Duration::from_millis(flush_delay_ms),
            last_modification: Arc::new(Mutex::new(None)),
        })
    }

    /// push entry to undo stack
    pub fn push_undo(&self, entry: HistoryEntry) -> Result<()> {
        {
            let mut history = self
                .history
                .lock()
                .map_err(|e| anyhow!("lock error: {}", e))?;
            history.push_undo(entry);
        }
        self.schedule_flush();
        Ok(())
    }

    /// execute undo: pop from undo, return entry, caller must push to redo after executing
    pub fn pop_undo(&self) -> Result<Option<HistoryEntry>> {
        let entry = {
            let mut history = self
                .history
                .lock()
                .map_err(|e| anyhow!("lock error: {}", e))?;
            history.pop_undo()
        };
        if entry.is_some() {
            self.schedule_flush();
        }
        Ok(entry)
    }

    /// push entry to redo stack
    pub fn push_redo(&self, entry: HistoryEntry) -> Result<()> {
        {
            let mut history = self
                .history
                .lock()
                .map_err(|e| anyhow!("lock error: {}", e))?;
            history.push_redo(entry);
        }
        self.schedule_flush();
        Ok(())
    }

    /// execute redo: pop from redo, return entry, caller must push to undo after executing
    pub fn pop_redo(&self) -> Result<Option<HistoryEntry>> {
        let entry = {
            let mut history = self
                .history
                .lock()
                .map_err(|e| anyhow!("lock error: {}", e))?;
            history.pop_redo()
        };
        if entry.is_some() {
            self.schedule_flush();
        }
        Ok(entry)
    }

    /// clear all history
    pub fn clear(&self) -> Result<()> {
        {
            let mut history = self
                .history
                .lock()
                .map_err(|e| anyhow!("lock error: {}", e))?;
            history.clear();
        }
        self.schedule_flush();
        Ok(())
    }

    /// get stack lengths
    pub fn stack_lengths(&self) -> Result<(usize, usize)> {
        let history = self
            .history
            .lock()
            .map_err(|e| anyhow!("lock error: {}", e))?;
        Ok((history.undo_len(), history.redo_len()))
    }

    /// get history data for listing
    pub fn get_data(&self) -> Result<HistoryData> {
        let history = self
            .history
            .lock()
            .map_err(|e| anyhow!("lock error: {}", e))?;
        Ok(history.data.clone())
    }

    /// schedule a flush after the debounce delay
    fn schedule_flush(&self) {
        let mut last_mod = self.last_modification.lock().unwrap();
        *last_mod = Some(Instant::now());
    }

    /// check if flush is due and perform it if needed
    /// call this periodically from daemon main loop
    pub fn maybe_flush(&self) -> Result<()> {
        let should_flush = {
            let last_mod = self.last_modification.lock().unwrap();
            if let Some(instant) = *last_mod {
                instant.elapsed() >= self.flush_delay
            } else {
                false
            }
        };

        if should_flush {
            self.flush()?;
            let mut last_mod = self.last_modification.lock().unwrap();
            *last_mod = None;
        }

        Ok(())
    }

    /// force flush to disk immediately
    pub fn flush(&self) -> Result<()> {
        let mut history = self
            .history
            .lock()
            .map_err(|e| anyhow!("lock error: {}", e))?;
        history.save()
    }
}

/// get path to history file (~/.cwm/history.json)
pub fn get_history_path() -> Result<PathBuf> {
    let cwm_dir = config::ensure_cwm_dir()?;
    Ok(cwm_dir.join("history.json"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_test_entry(method: &str) -> HistoryEntry {
        HistoryEntry {
            commands: vec![StoredCommand {
                method: method.to_string(),
                params: serde_json::json!({"app": ["TestApp"]}),
            }],
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn test_history_push_pop_undo() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("history.json");
        let mut history = History::new(50, path);

        history.push_undo(create_test_entry("resize"));
        assert_eq!(history.undo_len(), 1);
        assert!(history.is_dirty());

        let entry = history.pop_undo();
        assert!(entry.is_some());
        assert_eq!(history.undo_len(), 0);
    }

    #[test]
    fn test_history_push_clears_redo() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("history.json");
        let mut history = History::new(50, path);

        // push to undo, then pop and push to redo
        history.push_undo(create_test_entry("resize"));
        let entry = history.pop_undo().unwrap();
        history.push_redo(entry);
        assert_eq!(history.redo_len(), 1);

        // new undo should clear redo
        history.push_undo(create_test_entry("move"));
        assert_eq!(history.redo_len(), 0);
    }

    #[test]
    fn test_history_limit_enforcement() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("history.json");
        let mut history = History::new(3, path);

        for i in 0..5 {
            history.push_undo(create_test_entry(&format!("action{}", i)));
        }

        assert_eq!(history.undo_len(), 3);
    }

    #[test]
    fn test_history_save_load() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("history.json");

        // create and save
        {
            let mut history = History::new(50, path.clone());
            history.push_undo(create_test_entry("resize"));
            history.push_undo(create_test_entry("move"));
            history.save().unwrap();
        }

        // load and verify
        {
            let history = History::load(50, path).unwrap();
            assert_eq!(history.undo_len(), 2);
            assert!(!history.is_dirty());
        }
    }

    #[test]
    fn test_history_clear() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("history.json");
        let mut history = History::new(50, path);

        history.push_undo(create_test_entry("resize"));
        history.push_undo(create_test_entry("move"));
        let entry = history.pop_undo().unwrap();
        history.push_redo(entry);

        history.clear();
        assert_eq!(history.undo_len(), 0);
        assert_eq!(history.redo_len(), 0);
    }

    #[test]
    fn test_history_load_nonexistent_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nonexistent.json");

        let history = History::load(50, path).unwrap();
        assert_eq!(history.undo_len(), 0);
        assert_eq!(history.redo_len(), 0);
    }

    #[test]
    fn test_stored_command_serialization() {
        let cmd = StoredCommand {
            method: "resize".to_string(),
            params: serde_json::json!({"app": ["Safari"], "to": "800x600px"}),
        };

        let json = serde_json::to_string(&cmd).unwrap();
        let parsed: StoredCommand = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.method, "resize");
    }

    #[test]
    fn test_history_entry_serialization() {
        let entry = create_test_entry("maximize");

        let json = serde_json::to_string(&entry).unwrap();
        let parsed: HistoryEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.commands.len(), 1);
        assert_eq!(parsed.commands[0].method, "maximize");
    }

    #[test]
    fn test_history_data_serialization() {
        let data = HistoryData {
            undo: vec![create_test_entry("resize"), create_test_entry("move")],
            redo: vec![create_test_entry("maximize")],
        };

        let json = serde_json::to_string_pretty(&data).unwrap();
        let parsed: HistoryData = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.undo.len(), 2);
        assert_eq!(parsed.redo.len(), 1);
    }
}
