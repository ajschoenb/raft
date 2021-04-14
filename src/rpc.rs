extern crate serde;
extern crate serde_json;
use serde::{Serialize, Deserialize};

pub trait RPC {
    fn serialize<'a>(&self) -> String
        where Self: Serialize + Deserialize<'a>
    {
        serde_json::to_string(self).unwrap()
    }

    fn deserialize<'a>(&mut self, s: &'a str) 
        where Self: Serialize + Deserialize<'a>
    {
        *self = serde_json::from_str(s).unwrap();
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum RPCType {
    AppendEntries,
    RequestVote,
    Response,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct AppendEntries {
    rtype: RPCType,         // always RPCAppendEntries
    term: i64,              // leader's term
    leader_id: i32,         // leader's ID
    prev_log_idx: i64,      // index of log entry immediately preceding new ones
    prev_log_term: i64,     // term of prev_log_idx entry
    // entries: Vec<LogEntry>  // log entries to add
    leader_commit_idx: i64, // leader's commit_idx
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct RequestVote {
    rtype: RPCType,         // always RPCRequestVote
    term: i64,              // candidate's term
    candidate_id: i32,      // candidate's ID
    last_log_idx: i64,      // index of candidate's last log entry
    last_log_term: i64,     // term of candidate's last log entry
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct Response {
    rtype: RPCType,         // always RPCResponse
    term: i64,              // current term
    result: bool,           // depends on request type
}

impl RPC for AppendEntries {}
impl RPC for RequestVote {}
impl RPC for Response {}

impl AppendEntries {
    pub fn new(
        term: i64,
        leader_id: i32,
        prev_log_idx: i64,
        prev_log_term: i64,
        // entries: Vec<LogEntry>
        leader_commit_idx: i64) -> AppendEntries {
        AppendEntries {
            rtype: RPCType::AppendEntries,
            term: term,
            leader_id: leader_id,
            prev_log_idx: prev_log_idx,
            prev_log_term: prev_log_term,
            // entries: entries,
            leader_commit_idx: leader_commit_idx,
        }
    }
}

impl RequestVote {
    pub fn new(
        term: i64,
        candidate_id: i32,
        last_log_idx: i64,
        last_log_term: i64) -> RequestVote {
        RequestVote {
            rtype: RPCType::RequestVote,
            term: term,
            candidate_id: candidate_id,
            last_log_idx: last_log_idx,
            last_log_term: last_log_term,
        }
    }
}

impl Response {
    pub fn new(
        term: i64,
        result: bool) -> Response {
        Response {
            rtype: RPCType::Response,
            term: term,
            result: result,
        }
    }
}
