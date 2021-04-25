extern crate serde;
extern crate serde_json;
extern crate shellexpand;
use std::sync::{Arc, Mutex};
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;

#[derive(serde::Serialize, serde::Deserialize, Debug, Copy, Clone, PartialEq, Eq)]
pub struct RaftLogEntry {
    pub opid: i32,
    pub term: i64,
    pub applied: bool,
}

impl RaftLogEntry {
    pub fn new(
        opid: i32,
        term: i64,
    ) -> RaftLogEntry {
        RaftLogEntry {
            opid: opid,
            term: term,
            applied: false,
        }
    }

    pub fn from_string(s: &str) -> RaftLogEntry {
        serde_json::from_str(s).unwrap()
    }
}

#[derive(Debug)]
pub struct RaftLog {
    log_arc: Arc<Mutex<Vec<RaftLogEntry>>>,
    path: String,
    lfile: File,
}

impl RaftLog {
    pub fn new(path: String) -> RaftLog {
        let log = vec![RaftLogEntry::new(0, 0)];
        let lock = Mutex::new(log);
        let arc = Arc::new(lock);
        RaftLog {
            log_arc: arc,
            path: path.to_string(),
            lfile: File::create(path).unwrap(),
        }
    }

    pub fn from_file(path: String) -> RaftLog {
        let mut log = vec![RaftLogEntry::new(0, 0)];
        let _path = path.clone();
        let lfile = File::open(path).unwrap();
        let mut reader = BufReader::new(&lfile);
        let mut line = String::new();
        let mut len = reader.read_line(&mut line).unwrap();
        while len > 0 {
            let entry = RaftLogEntry::from_string(&line);
            log.push(entry);
            line.clear();
            len = reader.read_line(&mut line).unwrap();
        }
        let lock = Mutex::new(log);
        let arc = Arc::new(lock);
        RaftLog {
            log_arc: arc,
            path: _path,
            lfile: lfile,
        }
    }

    pub fn append(&mut self, entry: RaftLogEntry) {
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

    pub fn get(&self, idx: usize) -> Option<RaftLogEntry> {
        let lock = Arc::clone(&self.log_arc);
        let log = lock.lock().unwrap();
        let ret: Option<RaftLogEntry>;
        if idx < log.len() {
            ret = Some(log[idx]);
        } else {
            ret = None;
        }
        ret
    }

    pub fn get_vec(&self) -> Vec<RaftLogEntry> {
        let lock = Arc::clone(&self.log_arc);
        let log = lock.lock().unwrap();
        log.clone()
    }

    pub fn get_last(&self) -> RaftLogEntry {
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

    pub fn contains(&self, opid: i32) -> Option<bool> {
        let lock = Arc::clone(&self.log_arc);
        let log = lock.lock().unwrap();
        match log.iter().find(|&e| e.opid == opid) {
            Some(e) => Some(e.applied),
            None => None,
        }
    }

    pub fn idxof(&self, opid: i32) -> Option<usize> {
        let lock = Arc::clone(&self.log_arc);
        let log = lock.lock().unwrap();
        let mut ret = None;
        for (i, e) in log.iter().enumerate() {
            if e.opid == opid { ret = Some(i); break; }
        }
        ret
    }

    pub fn arc(&self) -> Arc<Mutex<Vec<RaftLogEntry>>> {
        Arc::clone(&self.log_arc)
    }
}
