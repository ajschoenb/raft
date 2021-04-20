use std::collections::HashMap;
use std::time::Duration;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use crate::rpc::*;

static REQNO: AtomicI32 = AtomicI32::new(1);

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

    pub fn test_comms(&self) {
        for (_, tx) in &self.txs {
            tx.send(make_client_request(self.id, 0)).unwrap();
        }
        for _ in 0..self.txs.len() {
            match self.rx.recv_timeout(Duration::from_secs(1)) {
                Ok(RPC::ClientRequest { id: _, opid: _ }) => {
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

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub fn run(&mut self) {
        // self.test_comms();
        
        let mut leader_idx = 0;
        let mut n = 0;

        // MAIN LOOP
        // while running {
        //      send a request to the leader (might need to discover leader first)
        //      wait for response - if failure or timeout then retry request
        // }
        while self.is_running() && n < self.n_reqs {
            let req_opid = REQNO.fetch_add(1, Ordering::SeqCst);
            let req = make_client_request(self.id, req_opid);
            loop {
                self.txs[&leader_idx].send(req.clone()).unwrap();
                match self.rx.recv_timeout(Duration::from_millis(100)) {
                    Ok(RPC::ClientResponse {
                        id: _,
                        opid: _,
                        success,
                    }) => {
                        if success {
                            n += 1;
                            break;
                        } else {
                            // failure -> try another server
                            leader_idx = (leader_idx + 1) % (self.txs.len() as i32);
                        }
                    },
                    Ok(rpc) => {
                        panic!("client {} recved a bad RPC {:?}", self.id, rpc);
                    },
                    Err(_) => {
                        continue;
                    },
                }
            }
        }

        println!("client {} done with requests", self.id);
        while self.is_running() {}
    }
}
