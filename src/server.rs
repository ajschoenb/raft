use std::collections::HashMap;
use std::time::Duration;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::cmp::min;
use rand::{thread_rng, Rng};
use crate::rpc::*;
use crate::log::*;

static BASE_ELECT_TIMEOUT: i32 = 100000;
static PING_RATE: i32 = 5000;

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
    log: Log,

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

    // election timeout, in "ticks"
    elect_timeout: i32,

    // election timer, in "ticks"
    elect_timer: i32,

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
            log: Log::new(lpath),
            running: running.clone(),
            state: ServerState::Follower,
            commit_idx: 0,
            applied_idx: 0,
            votes_needed: n_servers / 2 + 1,
            votes_recved: vec![],
            next_idx: vec![0, n_servers as usize - 1],
            match_idx: vec![0, n_servers as usize - 1],
            elect_timeout: thread_rng().gen_range(BASE_ELECT_TIMEOUT..2 * BASE_ELECT_TIMEOUT),
            elect_timer: 0,
            s_txs: s_txs,
            c_txs: c_txs,
            rx: rx,
        }
    }

    pub fn test_comms(&self) {
        for (_, tx) in &self.s_txs {
            tx.send(make_client_request(self.id)).unwrap();
        }
        for (_, tx) in &self.c_txs {
            tx.send(make_client_request(self.id)).unwrap();
        }
        for _ in 0..(self.s_txs.len() + self.c_txs.len()) {
            match self.rx.recv_timeout(Duration::from_secs(1)) {
                Ok(RPC::ClientRequest { id: _ }) => {
                },
                Ok(_) => {
                    panic!("server {} got a bad RPC", self.id);
                },
                Err(_) => {
                    panic!("server {} failed to get enough RPCs", self.id);
                }
            }
        }
        println!("server {} passed all tests", self.id);
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub fn send_server(&mut self, id: i32, rpc: RPC) {
        self.s_txs[&id].send(rpc).unwrap();
    }

    pub fn handle_recv_append_entries(&mut self, id: i32, term: i64, prev_log_idx: usize, prev_log_term: i64, entries: Vec<LogEntry>, leader_commit_idx: usize) {
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
            let mut last_new_entry_idx = 0;
            for (i, entry) in entries.into_iter().enumerate() {
                // prev_log_idx is the index of the entry immediately preceding the new ones
                let idx = i + prev_log_idx + 1;

                match self.log.get(idx) {
                    Some(existing_entry) => {
                        // conflict -> delete existing entries
                        // no conflict -> entry already exists in the log
                        if existing_entry.term != entry.term {
                            self.log.delete_from(idx);
                        } else {
                            assert_eq!(existing_entry.opid, entry.opid);
                        }
                    },
                    None => {
                        // nothing there -> append entry
                        self.log.append(entry);
                        last_new_entry_idx += 1;
                    },
                }
            }

            // 5. if leader_commit_idx > commit_idx, set commit_idx =
            //    min(leader_commit_idx, idx of last new entry)
            if leader_commit_idx > self.commit_idx {
                self.commit_idx = min(leader_commit_idx, last_new_entry_idx);
            }
        }

        // actually send a response
        let response = make_append_entries_response(self.id, self.curr_term, success);
        self.send_server(id, response);
    }

    pub fn handle_recv_request_vote(&mut self, id: i32, term: i64, last_log_idx: usize, last_log_term: i64) {
        // reset election timer
        self.elect_timer = 0;

        // 1. reply false if term < curr_term
        // 2. if vote is null or id, and candidate's log is at least as up-to-date as self,
        //    reply true, else reply false
        let self_last_log_term = self.log.get_last().term;
        let self_last_log_idx = self.log.len() - 1;
        let up_to_date = (self_last_log_term < last_log_term) || (self_last_log_term == last_log_term && self_last_log_idx <= last_log_idx);
        let success = (term > self.curr_term) && ((self.vote == -1 || self.vote == id)) && up_to_date;
        if success {
            println!("server {} state {:?} voted for {}", self.id, self.state, id);
            self.vote = id;
        }

        // actually send a response
        let response = make_request_vote_response(self.id, self.curr_term, success);
        self.send_server(id, response);
    }

    pub fn become_follower(&mut self) {
        println!("server {} becoming follower", self.id);
        self.vote = -1;
        self.state = ServerState::Follower;
    }

    pub fn become_candidate(&mut self) {
        println!("server {} becoming candidate", self.id);
        self.curr_term += 1;
        self.vote = self.id;
        self.votes_recved = vec![];
        self.elect_timer = 0;
        self.state = ServerState::Candidate;
    }

    pub fn become_leader(&mut self) {
        println!("server {} becoming leader", self.id);
        let last_log_idx = self.log.len() - 1;
        self.elect_timer = 0;
        self.next_idx = vec![last_log_idx, self.s_txs.len()];
        self.match_idx = vec![0, self.s_txs.len()];
        self.state = ServerState::Leader;
    }

    pub fn tick_follower(&mut self) {
        match self.rx.try_recv() {
            Ok(RPC::AppendEntries {
                id,
                term,
                prev_log_idx,
                prev_log_term,
                entries,
                leader_commit_idx,
            }) => {
                self.handle_recv_append_entries(id, term, prev_log_idx, prev_log_term, entries, leader_commit_idx);
            },
            Ok(RPC::RequestVote {
                id,
                term,
                last_log_idx,
                last_log_term,
            }) => {
                self.handle_recv_request_vote(id, term, last_log_idx, last_log_term);
            },
            Ok(RPC::ClientRequest {
                id: _,
            }) => {
                println!("not sure what to do when client requests from non-leader");
            },
            Ok(rpc) => {
                // println!("server {} got a bad RPC {:?}", self.id, rpc);
            },
            Err(_) => {
                // no need to panic, we just didn't have a message queued up
            },
        }
    }

    pub fn tick_candidate(&mut self) {
        // send RequestVote RPCs to all other servers
        if self.elect_timer % PING_RATE == 0 {
            let last_log_idx = self.log.len() - 1;
            let last_log_term = self.log.get(last_log_idx).unwrap().term;
            let request = make_request_vote(self.id, self.curr_term, last_log_idx, last_log_term);
            for (_, tx) in self.s_txs.iter().filter(|(i, _)| !self.votes_recved.contains(i)) {
                tx.send(request.clone()).unwrap();
            }
        }

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

                // if a candidate receives an AppendEntries RPC, they must have lost the election
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
                id: _,
            }) => {
                println!("not sure what to do when client requests from non-leader");
            },
            Ok(RPC::RequestVoteResponse {
                id,
                term,
                vote,
            }) => {
                if term > self.curr_term {
                    self.curr_term = term;
                    self.become_follower();
                } else {
                    if vote && !self.votes_recved.iter().any(|&i| i == id) {
                        self.votes_recved.push(id);
                        println!("server {} recved vote from {}, now have {} votes", self.id, id, self.votes_recved.len());
                    }
                    if self.votes_recved.len() >= self.votes_needed as usize {
                        self.become_leader();
                    }
                }
            },
            Ok(rpc) => {
                println!("server {} got a bad RPC {:?}", self.id, rpc);
            },
            Err(_) => {
                // no need to panic, we just didn't have a message queued up
            },
        }
    }

    pub fn tick_leader(&mut self) {
        // send heartbeat to all servers
        if self.elect_timer % PING_RATE == 0 {
            let heartbeat = make_append_entries(self.id, self.curr_term, self.log.len() - 1, self.log.get_last().term, vec![], self.commit_idx);
            for (_, tx) in self.s_txs.iter() {
                tx.send(heartbeat.clone()).unwrap();
            }
            // leader can't time out, so just reset election timer
            self.elect_timer = 0;
        }
    }

    pub fn run(&mut self) {
        // self.test_comms();
        // println!("server {} election timeout {}", self.id, self.elect_timeout);

        println!("TODO need to stop sending extra messages to all servers");
        println!("sending RequestVote when they've already voted for you is... not helpful, it just clogs the channel");

        // MAIN LOOP
        // while running {
        //      increment the "timer" and check for election timeout
        //      try_recv a message
        //      process the message
        //      BLOCKING IS BAD, DON'T DO IT
        //      YOU NEED TO KEEP LOOPING BECAUSE OF HEARTBEATS
        // }
        while self.is_running() {
            // if commit_idx > applied_idx, increment applied_idx and apply log[applied_idx]
            if self.commit_idx > self.applied_idx {
                self.applied_idx += 1;
                self.log.apply(self.applied_idx);
            }

            self.elect_timer += 1;
            if self.elect_timer >= self.elect_timeout {
                // election timeout, become a candidate
                self.become_candidate();
                // println!("server {} election timeout, becoming a candidate", self.id);
            }
            match self.state {
                ServerState::Follower => {
                    self.tick_follower();
                },
                ServerState::Candidate => {
                    // println!("tick candidate");
                    self.tick_candidate();
                },
                ServerState::Leader => {
                    // println!("tick leader");
                    // self.running.store(false, Ordering::SeqCst);
                    self.tick_leader();
                },
            }
        }
    }
}
