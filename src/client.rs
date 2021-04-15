use std::collections::HashMap;
use std::time::Duration;
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
        for (_, tx) in &self.txs {
            tx.send(make_client_request(self.id)).unwrap();
        }
        for (&i, rx) in &self.rxs {
            match rx.recv_timeout(Duration::from_secs(1)) {
                Ok(RPC::ClientRequest { id }) => {
                    assert_eq!(id, i);
                },
                Ok(_) => {
                    panic!("client {} got a bad RPC from server {}", self.id, i);
                },
                Err(_) => {
                    panic!("client {} failed to get an RPC from server {}", self.id, i);
                },
            }
        }
        println!("client {} passed all tests", self.id);
    }
}
