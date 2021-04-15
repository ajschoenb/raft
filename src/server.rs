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
    s_txs: HashMap<i32, Sender<RPC>>,      // senders for each other server
    s_rxs: HashMap<i32, Receiver<RPC>>,    // receivers for each other server
    c_txs: HashMap<i32, Sender<RPC>>,      // senders for each client
    c_rxs: HashMap<i32, Receiver<RPC>>,    // receivers for each client
}

impl Server {
    pub fn new(
        id: i32, 
        running: &Arc<AtomicBool>,
        s_txs: HashMap<i32, Sender<RPC>>,
        s_rxs: HashMap<i32, Receiver<RPC>>,
        c_txs: HashMap<i32, Sender<RPC>>,
        c_rxs: HashMap<i32, Receiver<RPC>>,
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
            s_rxs: s_rxs,
            c_txs: c_txs,
            c_rxs: c_rxs,
        }
    }

    pub fn run(&mut self) {
        for (_, tx) in &self.s_txs {
            tx.send(make_client_request(self.id)).unwrap();
        }
        for (_, tx) in &self.c_txs {
            tx.send(make_client_request(self.id)).unwrap();
        }
        for (&i, rx) in &self.s_rxs {
            match rx.recv_timeout(Duration::from_secs(1)) {
                Ok(RPC::ClientRequest { id }) => {
                    assert_eq!(id, i);
                },
                Ok(_) => {
                    panic!("server {} got a bad RPC from server {}", self.id, i);
                },
                Err(_) => {
                    panic!("server {} failed to get an RPC from server {}", self.id, i);
                },
            }
        }
        for (&i, rx) in &self.c_rxs {
            match rx.recv_timeout(Duration::from_secs(1)) {
                Ok(RPC::ClientRequest { id }) => {
                    assert_eq!(id, i);
                },
                Ok(_) => {
                    panic!("server {} got a bad RPC from client {}", self.id, i);
                },
                Err(_) => {
                    panic!("server {} failed to get an RPC from client {}", self.id, i);
                },
            }
        }
        println!("server {} passed all tests", self.id);
    }
}
