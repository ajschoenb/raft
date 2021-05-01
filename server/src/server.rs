extern crate log;
use std::collections::HashMap;
use std::time::Duration;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::cmp::{min, max};
use std::thread;
use rand::{thread_rng, Rng, random};
use log::*;

use crate::rpc::*;
use crate::raftlog::*;
use crate::comms::RaftComms;

// all measured in ms
static BASE_ELECT_TIMEOUT: i32 = 500;
static PING_RATE: i32 = 50;
static CRASH_FREQ: u32 = 60000;
static CRASH_LENGTH: i32 = 5000;

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
pub struct Server<C: RaftComms> {
    // PERSISTENT STATE FOR ALL SERVERS
    // the ID of this server
    pub id: String,

    // the current term number
    curr_term: i64,

    // who this server voted for in the current term
    vote: String,

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
    votes_recved: Vec<String>,

    // VOLATILE STATE ON LEADERS
    // for each other server, the next log index to send
    next_idx: HashMap<String, usize>,

    // for each other server, the highest log index that matches
    match_idx: HashMap<String, usize>,

    // map from log index -> client index and client response when that index is applied
    client_res: HashMap<usize, (String, RPC)>,

    // election timeout, in ms
    elect_timeout: i32,

    // election timer, in ms
    elect_timer: i32,

    // if the server is currently crashed
    crashed: bool,

    // crash timer, in ms
    crash_timer: i32,

    // COMMUNICATION CHANNELS
    // names of other servers
    server_ids: Vec<String>,

    // communication struct
    comms: C,
}

impl<C> Server<C> where C: RaftComms {
    pub fn new(
        id: String, 
        running: &Arc<AtomicBool>,
        lpath: String,
        server_ids: Vec<String>,
        comms: C,
    ) -> Server<C> {
        Server {
            id: id,
            curr_term: 0,
            vote: String::new(),
            log: RaftLog::from_file(lpath),
            running: running.clone(),
            state: ServerState::Follower,
            commit_idx: 0,
            applied_idx: 0,
            votes_needed: (server_ids.len() as i32 + 1) / 2 + 1,
            votes_recved: vec![],
            next_idx: HashMap::new(),
            match_idx: HashMap::new(),
            client_res: HashMap::new(),
            elect_timeout: thread_rng().gen_range(BASE_ELECT_TIMEOUT..2 * BASE_ELECT_TIMEOUT),
            elect_timer: 0,
            crashed: false,
            crash_timer: 0,
            server_ids: server_ids,
            comms: comms,
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub fn become_follower(&mut self) {
        if self.state != ServerState::Follower {
            info!("server {} becoming follower", self.id);
        }
        self.vote = String::new();
        self.state = ServerState::Follower;
    }

    pub fn become_candidate(&mut self) {
        if self.state != ServerState::Candidate {
            info!("server {} becoming candidate", self.id);
        }
        self.curr_term += 1;
        self.vote = self.id.clone();
        self.votes_recved = vec![];
        self.votes_recved.push(self.id.clone());
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
        let me = self.id.clone();
        for id in self.server_ids.iter().filter(|&i| i.clone() != me) {
            self.next_idx.insert(id.clone(), last_log_idx + 1);
            self.match_idx.insert(id.clone(), 0);
        }
        self.state = ServerState::Leader;

    }

    pub fn handle_recv_append_entries(&mut self, id: String, term: i64, prev_log_idx: usize, prev_log_term: i64, entries: Vec<RaftLogEntry>, leader_commit_idx: usize) {
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
        let response = make_append_entries_response(self.curr_term, self.log.len() - 1, success);
        self.comms.send(id, response);
    }

    pub fn handle_recv_request_vote(&mut self, id: String, term: i64, last_log_idx: usize, last_log_term: i64) {
        // 1. reply false if term < curr_term
        // 2. if vote is null or id, and candidate's log is at least as up-to-date as self,
        //    reply true, else reply false
        let self_last_log_term = self.log.get_last().term;
        let self_last_log_idx = self.log.len() - 1;
        let up_to_date = (self_last_log_term < last_log_term) || (self_last_log_term == last_log_term && self_last_log_idx <= last_log_idx);
        let success = (term >= self.curr_term) && ((self.vote == "" || self.vote == id)) && up_to_date;
        if success {
            info!("server {} state {:?} voted for {}", self.id, self.state, id.clone());
            self.vote = id.clone();
        }

        // actually send a response
        let response = make_request_vote_response(self.curr_term, success);
        self.comms.send(id, response);
    }

    pub fn handle_recv_client_request(&mut self, id: String, opid: i64) {
        if self.state == ServerState::Leader {
            let response = make_client_response(opid, true);

            match self.log.contains(opid) {
                Some(true) => {
                    debug!("server {} short-circuiting applied request {}", self.id, opid);
                    // self.send_client(id, response);
                    self.comms.send(id, response);
                },
                Some(false) => {
                    debug!("server {} short-circuiting non-applied request {}", self.id, opid);
                    self.client_res.insert(self.log.idxof(opid).unwrap(), (id, response));
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
            let response = make_client_response(opid, false);
            self.comms.send(id, response);
        }
    }

    pub fn handle_recv_request_vote_response(&mut self, id: String, term: i64, vote: bool) {
        if term > self.curr_term {
            self.curr_term = term;
            self.become_follower();
        } else if self.state == ServerState::Candidate {
            if vote && !self.votes_recved.iter().any(|i| i.clone() == id) {
                self.votes_recved.push(id.clone());
                info!("server {} recved vote from {}, now have {} votes", self.id, id, self.votes_recved.len());
            }
            if self.votes_recved.len() >= self.votes_needed as usize {
                self.become_leader();
            }
        } else {
            info!("server {} recved vote, but not candidate", self.id);
        }
    }

    pub fn handle_recv_append_entries_response(&mut self, id: String, term: i64, idx: usize, result: bool) {
        if term > self.curr_term {
            self.curr_term = term;
            self.become_follower();
        } else if result {
            // update next_idx and match_idx
            self.match_idx.insert(id.clone(), max(self.match_idx[&id], idx));
            self.next_idx.insert(id.clone(), max(self.next_idx[&id], idx + 1));
        } else {
            // decrement next_idx
            self.next_idx.insert(id.clone(), self.next_idx[&id] - 1);
        }
    }

    pub fn handle_recv_request_log(&self, id: String, start_idx: usize) {
        // basically just send back as many entries as we can starting at start_idx
        let start_idx = min(start_idx, self.log.len());
        let end_idx = min(start_idx + 9, self.log.len());
        let entries = (&self.log.get_vec()[start_idx..end_idx]).to_vec();
        let response = make_request_log_response(end_idx, self.log.len(), self.applied_idx, entries);
        self.comms.send(id, response);
    }

    pub fn recv_rpc(&mut self) {
        loop {
            match self.comms.try_recv() {
                Some((id, RPC::AppendEntries {
                    term,
                    prev_log_idx,
                    prev_log_term,
                    entries,
                    leader_commit_idx,
                })) => {
                    if term > self.curr_term {
                        self.curr_term = term;
                    }

                    // the only place an AppendEntries can come from is the leader
                    self.become_follower();
                    self.handle_recv_append_entries(id, term, prev_log_idx, prev_log_term, entries, leader_commit_idx);
                },
                Some((id, RPC::RequestVote {
                    term,
                    last_log_idx,
                    last_log_term,
                })) => {
                    if term > self.curr_term {
                        self.curr_term = term;
                        self.become_follower();
                    }
                    self.handle_recv_request_vote(id, term, last_log_idx, last_log_term);
                },
                Some((id, RPC::ClientRequest {
                    opid,
                })) => {
                    self.handle_recv_client_request(id, opid);
                },
                Some((id, RPC::RequestVoteResponse {
                    term,
                    vote,
                })) => {
                    self.handle_recv_request_vote_response(id, term, vote);
                },
                Some((id, RPC::AppendEntriesResponse {
                    term,
                    idx,
                    result,
                })) => {
                    self.handle_recv_append_entries_response(id, term, idx, result);
                },
                Some((id, RPC::RequestLog {
                    start_idx
                })) => {
                    self.handle_recv_request_log(id, start_idx);
                },
                Some((_, rpc)) => {
                    panic!("server {} got a bad RPC {:?}", self.id, rpc);
                },
                None => {
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
            let request = make_request_vote(self.curr_term, last_log_idx, last_log_term);
            for id in self.server_ids.iter().filter(|i| !self.votes_recved.contains(i)) {
                self.comms.send(id.clone(), request.clone());
            }
        }
    }

    pub fn tick_leader(&mut self) {
        // send AppendEntries to all servers
        if self.elect_timer % PING_RATE == 0 {
            let heartbeat = make_append_entries(self.curr_term, self.log.len() - 1, self.log.get(self.log.len() - 1).unwrap().term, vec![], self.commit_idx);
            for id in self.server_ids.iter().filter(|&i| i.clone() != self.id) {
                let rpc: RPC;
                if self.log.len() - 1 >= self.next_idx[id] {
                    let prev_idx = self.next_idx[id] - 1;
                    let prev_term = self.log.get(prev_idx).unwrap().term;
                    let last_idx = min(self.log.len(), prev_idx + 9);
                    let entries = (&self.log.get_vec()[prev_idx + 1..last_idx]).to_vec();
                    debug!("server {} sending {} {} entries to commit starting at {}", self.id, id, entries.len(), prev_idx + 1);
                    rpc = make_append_entries(self.curr_term, prev_idx, prev_term, entries, self.commit_idx);
                } else {
                    rpc = heartbeat.clone();
                }
                self.comms.send(id.clone(), rpc);
            }
            // leader can't time out, so just reset election timer
            self.elect_timer = 0;
        }

        // if there exists an N such that N > commit_idx, a majority of match_idx[i] >= N, and
        // log[N].term == curr_term, set commit_idx = N
        let mut sorted_match_idx = self.match_idx.values().cloned().collect::<Vec<usize>>();
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
            match self.comms.try_recv() {
                Some(_) => {},
                None => break,
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
                            Some((i, res)) => {
                                debug!("server {} sending response for idx {}", self.id, self.applied_idx);
                                self.comms.send(i, res)
                            },
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

            // sleep for 1 ms to make timing easy
            thread::sleep(Duration::from_millis(1));
        }
    }
}
