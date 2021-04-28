extern crate log;
use std::fs;
use log::*;
// use std::collections::HashMap;

use crate::opts::Opts;
use crate::raftlog::RaftLog;

pub fn check(opts: Opts, logpathbase: String) {
    info!("Checking Raft run: {} servers, {} requests * {} clients", opts.n_servers, opts.n_request, opts.n_clients);

    // let mut logs = HashMap::new();
    // for id in 0..opts.n_servers {
        // let path = format!("{}/server{}.log", logpathbase, id);
        // let log = RaftLog::from_file(path);
        // logs.insert(id, log);
    // }

    let mut logs: Vec<RaftLog> = fs::read_dir(logpathbase)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path().to_string_lossy().into_owned())
        .filter(|p| p.ends_with(".log"))
        .map(|p| RaftLog::from_file(p))
        .collect();

    let lock0 = logs.remove(0).arc();
    let log0 = lock0.lock().unwrap();
    assert!(log0.len() >= (opts.n_request * (opts.n_clients as i64) + 1) as usize);
    for (i, l) in logs.iter().enumerate() {
        let lock = l.arc();
        let log = lock.lock().unwrap();
        assert!(log.len() == log0.len());
        for j in 0..log.len() {
            assert!(log[j] == log0[j], "log {} doesn't match at index {}", i, j);
        }
    }
}
