extern crate serde;
extern crate serde_json;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub enum RPC {
    AppendEntries {
        term: i64,
        leader_id: i32,
        prev_log_idx: i64,
        prev_log_term: i64,
        // entries: Vec<LogEntry>,
        leader_commit_idx: i64,
    },
    RequestVote {
        term: i64,
        candidate_id: i32,
        last_log_idx: i64,
        last_log_term: i64,
    },
    Response {
        term: i64,
        result: bool,
    },
    ClientRequest {
        id: i32,
    },
    ClientResponse {
        id: i32,
    },
}


pub fn make_append_entries(
    term: i64,
    leader_id: i32,
    prev_log_idx: i64,
    prev_log_term: i64,
    // entries: Vec<LogEntry>
    leader_commit_idx: i64,
) -> RPC {
    RPC::AppendEntries {
        term: term,
        leader_id: leader_id,
        prev_log_idx: prev_log_idx,
        prev_log_term: prev_log_term,
        // entries: entries,
        leader_commit_idx: leader_commit_idx,
    }
}

pub fn make_request_vote(
    term: i64,
    candidate_id: i32,
    last_log_idx: i64,
    last_log_term: i64,
) -> RPC {
    RPC::RequestVote {
        term: term,
        candidate_id: candidate_id,
        last_log_idx: last_log_idx,
        last_log_term: last_log_term,
    }
}

pub fn make_response(
    term: i64,
    result: bool,
) -> RPC {
    RPC::Response {
        term: term,
        result: result,
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
