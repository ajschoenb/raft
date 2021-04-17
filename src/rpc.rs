extern crate serde;
extern crate serde_json;
use crate::log::LogEntry;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub enum RPC {
    AppendEntries {
        id: i32,
        term: i64,
        prev_log_idx: usize,
        prev_log_term: i64,
        entries: Vec<LogEntry>,
        leader_commit_idx: usize,
    },
    RequestVote {
        id: i32,
        term: i64,
        last_log_idx: usize,
        last_log_term: i64,
    },
    AppendEntriesResponse {
        id: i32,
        term: i64,
        result: bool,
    },
    RequestVoteResponse {
        id: i32,
        term: i64,
        vote: bool,
    },
    ClientRequest {
        id: i32,
    },
    ClientResponse {
        id: i32,
    },
}

pub fn make_append_entries(
    id: i32,
    term: i64,
    prev_log_idx: usize,
    prev_log_term: i64,
    entries: Vec<LogEntry>,
    leader_commit_idx: usize,
) -> RPC {
    RPC::AppendEntries {
        id: id,
        term: term,
        prev_log_idx: prev_log_idx,
        prev_log_term: prev_log_term,
        entries: entries,
        leader_commit_idx: leader_commit_idx,
    }
}

pub fn make_request_vote(
    id: i32,
    term: i64,
    last_log_idx: usize,
    last_log_term: i64,
) -> RPC {
    RPC::RequestVote {
        id: id,
        term: term,
        last_log_idx: last_log_idx,
        last_log_term: last_log_term,
    }
}

pub fn make_append_entries_response(
    id: i32,
    term: i64,
    result: bool,
) -> RPC {
    RPC::AppendEntriesResponse {
        id: id,
        term: term,
        result: result,
    }
}

pub fn make_request_vote_response(
    id: i32,
    term: i64,
    vote: bool,
) -> RPC {
    RPC::RequestVoteResponse {
        id: id,
        term: term,
        vote: vote,
    }
}

pub fn make_client_request(
    id: i32,
) -> RPC {
    RPC::ClientRequest {
        id: id,
    }
}

pub fn make_client_response(
    id: i32,
) -> RPC {
    RPC::ClientResponse {
        id: id,
    }
}
