extern crate serde;
extern crate serde_json;

use crate::raftlog::RaftLogEntry;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub enum RPC {
    AppendEntries {
        term: i64,
        prev_log_idx: usize,
        prev_log_term: i64,
        entries: Vec<RaftLogEntry>,
        leader_commit_idx: usize,
    },
    RequestVote {
        term: i64,
        last_log_idx: usize,
        last_log_term: i64,
    },
    AppendEntriesResponse {
        term: i64,
        idx: usize,
        result: bool,
    },
    RequestVoteResponse {
        term: i64,
        vote: bool,
    },
    ClientRequest {
        opid: i32,
    },
    ClientResponse {
        opid: i32,
        success: bool,
    },
}

pub fn make_append_entries(
    term: i64,
    prev_log_idx: usize,
    prev_log_term: i64,
    entries: Vec<RaftLogEntry>,
    leader_commit_idx: usize,
) -> RPC {
    RPC::AppendEntries {
        term: term,
        prev_log_idx: prev_log_idx,
        prev_log_term: prev_log_term,
        entries: entries,
        leader_commit_idx: leader_commit_idx,
    }
}

pub fn make_request_vote(
    term: i64,
    last_log_idx: usize,
    last_log_term: i64,
) -> RPC {
    RPC::RequestVote {
        term: term,
        last_log_idx: last_log_idx,
        last_log_term: last_log_term,
    }
}

pub fn make_append_entries_response(
    term: i64,
    idx: usize,
    result: bool,
) -> RPC {
    RPC::AppendEntriesResponse {
        term: term,
        idx: idx,
        result: result,
    }
}

pub fn make_request_vote_response(
    term: i64,
    vote: bool,
) -> RPC {
    RPC::RequestVoteResponse {
        term: term,
        vote: vote,
    }
}

pub fn make_client_request(
    opid: i32,
) -> RPC {
    RPC::ClientRequest {
        opid: opid,
    }
}

pub fn make_client_response(
    opid: i32,
    success: bool,
) -> RPC {
    RPC::ClientResponse {
        opid: opid,
        success: success,
    }
}
