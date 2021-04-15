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
    rx: Receiver<RPC>,
}

impl Client {
    pub fn new(
        id: i32,
        n_reqs: i64,
        running: &Arc<AtomicBool>,
        txs: HashMap<i32, Sender<RPC>>,
        rx: Receiver<RPC>,
    ) -> Client {
        Client {
            id: id,
            n_reqs: n_reqs,
            running: running.clone(),
            txs: txs,
            rx: rx,
        }
    }

    pub fn run(&mut self) {
        for (_, tx) in &self.txs {
            tx.send(make_client_request(self.id)).unwrap();
        }
        for _ in 0..self.txs.len() {
            match self.rx.recv_timeout(Duration::from_secs(1)) {
                Ok(RPC::ClientRequest { id: _ }) => {
                },
                Ok(_) => {
                    panic!("client {} got a bad RPC", self.id);
                },
                Err(_) => {
                    panic!("client {} failed to get enough RPCs", self.id);
                }
            }
        }
        println!("client {} passed all tests", self.id);
    }
}
