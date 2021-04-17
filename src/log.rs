extern crate serde;
extern crate serde_json;
extern crate shellexpand;
use std::sync::{Arc, Mutex};
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;

#[derive(serde::Serialize, serde::Deserialize, Debug, Copy, Clone, PartialEq, Eq)]
pub struct LogEntry {
    pub opid: i32,
    pub term: i64,
    pub applied: bool,
}

impl LogEntry {
    pub fn new(
        opid: i32,
        term: i64,
    ) -> LogEntry {
        LogEntry {
            opid: opid,
            term: term,
            applied: false,
        }
    }

    pub fn from_string(s: &str) -> LogEntry {
        serde_json::from_str(s).unwrap()
    }
}

#[derive(Debug)]
pub struct Log {
    log_arc: Arc<Mutex<Vec<LogEntry>>>,
    path: String,
    lfile: File,
}

impl Log {
    pub fn new(path: String) -> Log {
        let log = vec![LogEntry::new(0, 0)];
        let lock = Mutex::new(log);
        let arc = Arc::new(lock);
        Log {
            log_arc: arc,
            path: path.to_string(),
            lfile: File::create(path).unwrap(),
        }
    }

    pub fn from_file(path: String) -> Log {
        let mut log = vec![LogEntry::new(0, 0)];
        let _path = path.clone();
        let lfile = File::open(path).unwrap();
        let mut reader = BufReader::new(&lfile);
        let mut line = String::new();
        let mut len = reader.read_line(&mut line).unwrap();
        while len > 0 {
            let entry = LogEntry::from_string(&line);
            log.push(entry);
            line.clear();
            len = reader.read_line(&mut line).unwrap();
        }
        let lock = Mutex::new(log);
        let arc = Arc::new(lock);
        Log {
            log_arc: arc,
            path: _path,
            lfile: lfile,
        }
    }

    pub fn append(&mut self, entry: LogEntry) {
        let lock = Arc::clone(&self.log_arc);
        let mut log = lock.lock().unwrap();
        log.push(entry);
    }

    pub fn apply(&mut self, idx: usize) {
        let lock = Arc::clone(&self.log_arc);
        let log = lock.lock().unwrap();
        if idx < log.len() {
            let mut entry = log[idx];
            entry.applied = true;
            serde_json::to_writer(&mut self.lfile, &entry).unwrap();
            writeln!(&mut self.lfile).unwrap();
            self.lfile.flush().unwrap();
        }
    }

    pub fn get(&self, idx: usize) -> Option<LogEntry> {
        let lock = Arc::clone(&self.log_arc);
        let log = lock.lock().unwrap();
        let ret: Option<LogEntry>;
        if idx < log.len() {
            ret = Some(log[idx]);
        } else {
            ret = None;
        }
        ret
    }

    pub fn get_last(&self) -> LogEntry {
        let lock = Arc::clone(&self.log_arc);
        let log = lock.lock().unwrap();
        log[log.len() - 1]
    }

    pub fn len(&self) -> usize {
        let lock = Arc::clone(&self.log_arc);
        let log = lock.lock().unwrap();
        log.len()
    }

    pub fn delete_from(&mut self, idx: usize) {
        let lock = Arc::clone(&self.log_arc);
        let mut log = lock.lock().unwrap();
        let len = log.len();
        log.drain(idx..len);
    }
}
