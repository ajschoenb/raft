use std::collections::HashMap;
use std::time::Duration;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::rpc::*;

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
    pub id: i32,                // the index of this server
    curr_term: i64,             // the current term
    vote: i32,                  // who received vote in current term
    // log: Log,                // local log
    running: Arc<AtomicBool>,   // whether the simulation is running

    // VOLATILE STATE ON ALL SERVERS
    state: ServerState,         // current state of the server
    commit_idx: i64,            // index of highest committed log entry
    applied_idx: i64,           // index of highest applied log entry

    // VOLATILE STATE ON LEADERS
    next_idx: Vec<i64>,         // for each server, next log index to send
    match_idx: Vec<i64>,        // for each server, highest log index that matches

    // COMMUNICATION CHANNELS
    s_txs: HashMap<i32, Sender<RPC>>,       // senders for each other server
    c_txs: HashMap<i32, Sender<RPC>>,       // senders for each client
    rx: Receiver<RPC>,                      // receiver for this server
}

impl Server {
    pub fn new(
        id: i32, 
        running: &Arc<AtomicBool>,
        s_txs: HashMap<i32, Sender<RPC>>,
        c_txs: HashMap<i32, Sender<RPC>>,
        rx: Receiver<RPC>,
    ) -> Server {
        Server {
            id: id,
            curr_term: 0,
            vote: -1,
            // log: ?,
            running: running.clone(),
            state: ServerState::Follower,
            commit_idx: 0,
            applied_idx: 0,
            next_idx: vec![],
            match_idx: vec![],
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

    pub fn run(&mut self) {
        self.test_comms();

        // MAIN LOOP
        // while running {
        //      increment the "timer" and check for election timeout
        //      try_recv a message
        //      process the message
        //      BLOCKING IS BAD, DON'T DO IT
        //      YOU NEED TO KEEP LOOPING BECAUSE OF HEARTBEATS
        // }
    }
}
