use std::time::Duration;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicI64, Ordering};
use std::thread;

use crate::rpc::*;
use crate::comms::RaftComms;

static REQNO: AtomicI32 = AtomicI32::new(1);

///
/// Client
/// Structure for simulating requests to the cluster
///
#[derive(Debug)]
pub struct Client<C: RaftComms> {
    pub id: String,
    n_reqs: i64,
    g_reqs: Arc<AtomicI64>,
    running: Arc<AtomicBool>,
    s_ids: Vec<String>,
    comms: C,
    // txs: HashMap<i32, Sender<RPC>>,
    // rx: Receiver<RPC>,
}

impl<C> Client<C> where C: RaftComms {
    pub fn new(
        id: String,
        n_reqs: i64,
        g_reqs: &Arc<AtomicI64>,
        running: &Arc<AtomicBool>,
        s_ids: Vec<String>,
        comms: C,
        // txs: HashMap<i32, Sender<RPC>>,
        // rx: Receiver<RPC>,
    ) -> Client<C> {
        Client {
            id: id,
            n_reqs: n_reqs,
            g_reqs: g_reqs.clone(),
            running: running.clone(),
            s_ids: s_ids,
            comms: comms,
            // txs: txs,
            // rx: rx,
        }
    }

    /*
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
    */

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
            let req = make_client_request(req_opid);
            loop {
                let leader_id = self.s_ids[leader_idx].clone();
                // self.txs[&leader_idx].send(req.clone()).unwrap();
                self.comms.send(leader_id, req.clone());
                // match self.rx.recv_timeout(Duration::from_millis(100)) {
                match self.comms.try_recv() {
                    Some((_, RPC::ClientResponse {
                        opid: _,
                        success,
                    })) => {
                        if success {
                            n += 1;
                            break;
                        } else {
                            // failure -> try another server
                            leader_idx = (leader_idx + 1) % self.s_ids.len();
                        }
                    },
                    Some((_, rpc)) => panic!("client {} recved a bad RPC {:?}", self.id, rpc),
                    None => thread::sleep(Duration::from_millis(100)),
                }
            }
        }

        println!("client {} done with requests", self.id);
        self.g_reqs.fetch_sub(self.n_reqs, Ordering::SeqCst);
        if self.g_reqs.load(Ordering::SeqCst) == 0 {
            thread::sleep(Duration::from_secs(3));
            self.running.store(false, Ordering::SeqCst);
        }
        while self.is_running() {}
    }
}
