extern crate log;
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::cmp::{min, max};
use rand::{thread_rng, Rng, random};
use log::*;
use crate::rpc::*;
use crate::raftlog::*;

// all in "ticks"
// TODO find a better unit of measurement for these...
static BASE_ELECT_TIMEOUT: i32 = 500000;
static PING_RATE: i32 = 5000;
static CRASH_FREQ: u32 = 10000000;
static CRASH_LENGTH: i32 = 10000000;

///
/// ServerState
/// enum for server state machine
///
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServerState {
    Follower,
    Candidate,
    Leader,
}

///
/// Server
/// Structure for maintaining server-local state
///
#[derive(Debug)]
pub struct Server {
    // PERSISTENT STATE FOR ALL SERVERS
    // the ID of this server
    pub id: i32,

    // the current term number
    curr_term: i64,

    // who this server voted for in the current term
    vote: i32,

    // the local log
    log: RaftLog,

    // whether the simulation is still running
    running: Arc<AtomicBool>,

    // VOLATILE STATE ON ALL SERVERS
    // the current state of the server
    state: ServerState,

    // the index of the highest committed entry in the local log
    commit_idx: usize,

    // the index of the highest applied entry in the local log
    applied_idx: usize,

    // VOLATILE STATE ON ALL CANDIDATES
    // the number of votes needed to win the election (majority of servers)
    votes_needed: i32,
    
    // votes received so far
    votes_recved: Vec<i32>,

    // VOLATILE STATE ON LEADERS
    // for each other server, the next log index to send
    next_idx: Vec<usize>,

    // for each other server, the highest log index that matches
    match_idx: Vec<usize>,

    // map from log index -> client index and client response when that index is applied
    client_res: HashMap<usize, (i32, RPC)>,

    // election timeout, in "ticks"
    elect_timeout: i32,

    // election timer, in "ticks"
    elect_timer: i32,

    // if the server is currently crashed
    crashed: bool,

    // crash timer, in "ticks"
    crash_timer: i32,

    // set of unique request IDs that have been committed
    committed_ops: HashSet<i32>,

    // COMMUNICATION CHANNELS
    // senders for each other server
    s_txs: HashMap<i32, Sender<RPC>>,

    // senders for each client
    c_txs: HashMap<i32, Sender<RPC>>,

    // generic receiver
    rx: Receiver<RPC>,
}

impl Server {
    pub fn new(
        id: i32, 
        running: &Arc<AtomicBool>,
        lpath: String,
        n_servers: i32,
        s_txs: HashMap<i32, Sender<RPC>>,
        c_txs: HashMap<i32, Sender<RPC>>,
        rx: Receiver<RPC>,
    ) -> Server {
        Server {
            id: id,
            curr_term: 0,
            vote: -1,
            log: RaftLog::new(lpath),
            running: running.clone(),
            state: ServerState::Follower,
            commit_idx: 0,
            applied_idx: 0,
            votes_needed: n_servers / 2 + 1,
            votes_recved: vec![],
            next_idx: vec![1; n_servers as usize],
            match_idx: vec![0; n_servers as usize],
            client_res: HashMap::new(),
            elect_timeout: thread_rng().gen_range(BASE_ELECT_TIMEOUT..2 * BASE_ELECT_TIMEOUT),
            elect_timer: 0,
            crashed: false,
            crash_timer: 0,
            committed_ops: HashSet::new(),
            s_txs: s_txs,
            c_txs: c_txs,
            rx: rx,
        }
    }

    pub fn test_comms(&self) {
        for (_, tx) in &self.s_txs {
            tx.send(make_client_request(self.id, 0)).unwrap();
        }
        for (_, tx) in &self.c_txs {
            tx.send(make_client_request(self.id, 0)).unwrap();
        }
        for _ in 0..(self.s_txs.len() + self.c_txs.len()) {
            match self.rx.recv_timeout(Duration::from_secs(1)) {
                Ok(RPC::ClientRequest { id: _, opid: _ }) => {},
                Ok(_) => panic!("server {} got a bad RPC", self.id),
                Err(_) => panic!("server {} failed to get enough RPCs", self.id),
            }
        }
        println!("server {} passed all tests", self.id);
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub fn send_server(&self, id: i32, rpc: RPC) {
        self.s_txs[&id].send(rpc).unwrap();
    }

    pub fn send_client(&self, id: i32, rpc: RPC) {
        self.c_txs[&id].send(rpc).unwrap();
    }

    pub fn become_follower(&mut self) {
        if self.state != ServerState::Follower {
            info!("server {} becoming follower", self.id);
        }
        self.vote = -1;
        self.state = ServerState::Follower;
    }

    pub fn become_candidate(&mut self) {
        if self.state != ServerState::Candidate {
            info!("server {} becoming candidate", self.id);
        }
        self.curr_term += 1;
        self.vote = self.id;
        self.votes_recved = vec![];
        self.elect_timer = 0;
        self.state = ServerState::Candidate;
    }

    pub fn become_leader(&mut self) {
        if self.state != ServerState::Leader {
            info!("server {} becoming leader", self.id);
        }
        // insert blank no-op entry into log
        self.log.append(RaftLogEntry::new(0, self.curr_term));

        let last_log_idx = self.log.len() - 1;
        self.elect_timer = -1;
        self.next_idx.fill(last_log_idx + 1);
        self.match_idx.fill(0);
        self.state = ServerState::Leader;

    }

    pub fn handle_recv_append_entries(&mut self, id: i32, term: i64, prev_log_idx: usize, prev_log_term: i64, entries: Vec<RaftLogEntry>, leader_commit_idx: usize) {
        // reset election timer
        self.elect_timer = 0;

        // 1. reply false if term < curr_term
        // 2. reply false if no entry at prev_log_idx whose term matches prev_log_term
        let success = term >= self.curr_term && match self.log.get(prev_log_idx) {
            Some(entry) => entry.term == prev_log_term,
            None => false,
        };
        if success {
            // 3. if an existing entry conflicts with a new one (same idx, diff terms) delete the
            // existing entry and all that follow it
            // 4. append any new entries not already in the log
            for (i, entry) in entries.into_iter().enumerate() {
                // prev_log_idx is the index of the entry immediately preceding the new ones
                let idx = i + prev_log_idx + 1;

                match self.log.get(idx) {
                    Some(existing_entry) => {
                        // conflict -> delete existing entries
                        // no conflict -> entry already exists in the log
                        if existing_entry.term != entry.term {
                            debug!("server {} deleting log starting at {}", self.id, idx);
                            self.log.delete_from(idx);
                            self.log.append(entry);
                        } else {
                            debug!("server {} existing entry at index {}", self.id, idx);
                            assert_eq!(existing_entry.opid, entry.opid);
                        }
                    },
                    None => {
                        // nothing there -> append entry
                        assert!(self.log.len() == idx, "log length inconsistent, {} {}", self.log.len(), idx);
                        debug!("server {} appending opid {} to log at index {}", self.id, entry.opid, idx);
                        self.log.append(entry);
                    },
                }
            }

            // 5. if leader_commit_idx > commit_idx, set commit_idx =
            //    min(leader_commit_idx, idx of last new entry)
            if leader_commit_idx > self.commit_idx {
                self.commit_idx = min(leader_commit_idx, self.log.len() - 1);
            }
        }

        // actually send a response
        let response = make_append_entries_response(self.id, self.curr_term, self.log.len() - 1, success);
        self.send_server(id, response);
    }

    pub fn handle_recv_request_vote(&mut self, id: i32, term: i64, last_log_idx: usize, last_log_term: i64) {
        // 1. reply false if term < curr_term
        // 2. if vote is null or id, and candidate's log is at least as up-to-date as self,
        //    reply true, else reply false
        let self_last_log_term = self.log.get_last().term;
        let self_last_log_idx = self.log.len() - 1;
        let up_to_date = (self_last_log_term < last_log_term) || (self_last_log_term == last_log_term && self_last_log_idx <= last_log_idx);
        let success = (term >= self.curr_term) && ((self.vote == -1 || self.vote == id)) && up_to_date;
        if success {
            info!("server {} state {:?} voted for {}", self.id, self.state, id);
            self.vote = id;
        }

        // actually send a response
        let response = make_request_vote_response(self.id, self.curr_term, success);
        self.send_server(id, response);
    }

    pub fn handle_recv_client_request(&mut self, id: i32, opid: i32) {
        if self.state == ServerState::Leader {
            let response = make_client_response(self.id, opid, true);

            match self.log.contains(opid) {
                Some(true) => {
                    info!("server {} short-circuiting applied request {}", self.id, opid);
                    self.send_client(id, response);
                },
                Some(false) => {
                    info!("server {} short-circuiting non-applied request {}", self.id, opid);
                    self.client_res.insert(self.log.len() - 1, (id, response));
                },
                None => {
                    // process the request and remember to send back the result
                    let entry = RaftLogEntry::new(opid, self.curr_term);
                    self.log.append(entry);
                    self.client_res.insert(self.log.len() - 1, (id, response));
                },
            }
        } else {
            // tell client that this isn't the leader so they can try someone else
            let response = make_client_response(self.id, opid, false);
            self.send_client(id, response);
        }
    }

    pub fn handle_recv_request_vote_response(&mut self, id: i32, term: i64, vote: bool) {
        if term > self.curr_term {
            self.curr_term = term;
            self.become_follower();
        } else if self.state == ServerState::Candidate {
            if vote && !self.votes_recved.iter().any(|&i| i == id) {
                self.votes_recved.push(id);
                info!("server {} recved vote from {}, now have {} votes", self.id, id, self.votes_recved.len());
            }
            if self.votes_recved.len() >= self.votes_needed as usize {
                self.become_leader();
            }
        } else {
            info!("server {} recved vote, but not candidate", self.id);
        }
    }

    pub fn handle_recv_append_entries_response(&mut self, id: i32, term: i64, idx: usize, result: bool) {
        if term > self.curr_term {
            self.curr_term = term;
            self.become_follower();
        } else if result {
            // update next_idx and match_idx
            self.match_idx[id as usize] = max(self.match_idx[id as usize], idx);
            self.next_idx[id as usize] = max(self.next_idx[id as usize], idx + 1);
        } else {
            // decrement next_idx
            self.next_idx[id as usize] -= 1;
        }
    }

    pub fn recv_rpc(&mut self) {
        loop {
            match self.rx.try_recv() {
                Ok(RPC::AppendEntries {
                    id,
                    term,
                    prev_log_idx,
                    prev_log_term,
                    entries,
                    leader_commit_idx,
                }) => {
                    if term > self.curr_term {
                        self.curr_term = term;
                    }

                    // the only place an AppendEntries can come from is the leader
                    self.become_follower();
                    self.handle_recv_append_entries(id, term, prev_log_idx, prev_log_term, entries, leader_commit_idx);
                },
                Ok(RPC::RequestVote {
                    id,
                    term,
                    last_log_idx,
                    last_log_term,
                }) => {
                    if term > self.curr_term {
                        self.curr_term = term;
                        self.become_follower();
                    }
                    self.handle_recv_request_vote(id, term, last_log_idx, last_log_term);
                },
                Ok(RPC::ClientRequest {
                    id,
                    opid,
                }) => {
                    self.handle_recv_client_request(id, opid);
                },
                Ok(RPC::RequestVoteResponse {
                    id,
                    term,
                    vote,
                }) => {
                    self.handle_recv_request_vote_response(id, term, vote);
                },
                Ok(RPC::AppendEntriesResponse {
                    id,
                    term,
                    idx,
                    result,
                }) => {
                    self.handle_recv_append_entries_response(id, term, idx, result);
                },
                Ok(rpc) => {
                    panic!("server {} got a bad RPC {:?}", self.id, rpc);
                },
                Err(_) => {
                    break;
                },
            }
        }
    }

    pub fn tick_follower(&mut self) {
    }

    pub fn tick_candidate(&mut self) {
        // send RequestVote RPCs to all other servers
        if self.elect_timer % PING_RATE == 0 {
            let last_log_idx = self.log.len() - 1;
            let last_log_term = self.log.get(last_log_idx).unwrap().term;
            let request = make_request_vote(self.id, self.curr_term, last_log_idx, last_log_term);
            for (&i, _) in self.s_txs.iter().filter(|(i, _)| !self.votes_recved.contains(i)) {
                self.send_server(i, request.clone());
            }
        }
    }

    pub fn tick_leader(&mut self) {
        // send AppendEntries to all servers
        if self.elect_timer % PING_RATE == 0 {
            let heartbeat = make_append_entries(self.id, self.curr_term, self.log.len() - 1, self.log.get(self.log.len() - 1).unwrap().term, vec![], self.commit_idx);
            for (&i, _) in self.s_txs.iter() {
                let rpc: RPC;
                if self.log.len() - 1 >= self.next_idx[i as usize] {
                    let prev_idx = self.next_idx[i as usize] - 1;
                    let prev_term = self.log.get(prev_idx).unwrap().term;
                    let entries = (&self.log.get_vec()[prev_idx + 1..]).to_vec();
                    debug!("server {} sending {} {} entries to commit starting at {}", self.id, i, entries.len(), prev_idx + 1);
                    rpc = make_append_entries(self.id, self.curr_term, prev_idx, prev_term, entries, self.commit_idx);
                } else {
                    rpc = heartbeat.clone();
                }
                self.send_server(i, rpc);
            }
            // leader can't time out, so just reset election timer
            self.elect_timer = 0;
        }

        // if there exists an N such that N > commit_idx, a majority of match_idx[i] >= N, and
        // log[N].term == curr_term, set commit_idx = N
        let mut sorted_match_idx = self.match_idx.clone();
        sorted_match_idx.sort();
        let majority_match_idx = sorted_match_idx[sorted_match_idx.len() / 2 + 1];
        if majority_match_idx > self.commit_idx && self.log.get(majority_match_idx).unwrap().term == self.curr_term {
            debug!("leader update commit_idx to {}", majority_match_idx);
            self.commit_idx = majority_match_idx;
        }
    }

    pub fn should_crash(&self) -> bool {
        let x = random::<u32>() % CRASH_FREQ;
        x == 0
    }

    pub fn crash(&mut self) {
        self.crashed = true;
        self.crash_timer = 0;
    }

    pub fn tick_crashed(&mut self) {
        // ignore messages sent while crashed
        loop {
            match self.rx.try_recv() {
                Ok(_) => {},
                Err(_) => break,
            }
        }

        // spin until uncrashed
        self.crash_timer += 1;
        if self.crash_timer == CRASH_LENGTH {
            info!("server {} uncrashing", self.id);
            self.become_follower();
            self.crashed = false;
        }
    }

    pub fn run(&mut self) {
        // self.test_comms();

        // MAIN LOOP
        // while running {
        //      increment the "timer" and check for election timeout
        //      try_recv a message
        //      process the message
        //      send messages to other servers as needed
        // }
        while self.is_running() {
            if self.crashed {
                self.tick_crashed();
            } else {
                // check for random crash
                if self.should_crash() {
                    info!("server {} crashing", self.id);
                    self.crash();
                }

                // if commit_idx > applied_idx, increment applied_idx and apply log[applied_idx]
                while self.commit_idx > self.applied_idx {
                    self.applied_idx += 1;
                    self.log.apply(self.applied_idx);

                    // if we're the leader, check to notify a client their request was applied
                    if self.state == ServerState::Leader {
                        match self.client_res.remove(&self.applied_idx) {
                            Some((i, res)) => self.send_client(i, res),
                            None => {},
                        }
                    }
                }

                // check for election timeout
                self.elect_timer += 1;
                if self.elect_timer >= self.elect_timeout {
                    self.become_candidate();
                }

                // receive and respond to incoming RPCs
                self.recv_rpc();

                // tick based on state
                match self.state {
                    ServerState::Follower => self.tick_follower(),
                    ServerState::Candidate => self.tick_candidate(),
                    ServerState::Leader => self.tick_leader(),
                }
            }
        }
    }
}
