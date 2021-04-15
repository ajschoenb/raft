use std::collections::HashMap;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::rpc::*;

///
/// Client
/// Structure for simulating requests to the cluster
///
#[derive(Debug)]
pub struct Client {
    pub id: i32,
    n_reqs: i64,
    running: Arc<AtomicBool>,
    txs: HashMap<i32, Sender<RPC>>,
    rxs: HashMap<i32, Receiver<RPC>>,
}

impl Client {
    pub fn new(
        id: i32,
        n_reqs: i64,
        running: &Arc<AtomicBool>,
        txs: HashMap<i32, Sender<RPC>>,
        rxs: HashMap<i32, Receiver<RPC>>,
    ) -> Client {
        Client {
            id: id,
            n_reqs: n_reqs,
            running: running.clone(),
            txs: txs,
            rxs: rxs,
        }
    }

    pub fn run(&mut self) {
        println!("client {} running with {} requests", self.id, self.n_reqs);
    }
}
