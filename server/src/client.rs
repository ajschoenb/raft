use std::time::Duration;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::thread;
use rand::random; 
use log::*;

use crate::rpc::*;
use crate::comms::RaftComms;

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
}

impl<C> Client<C> where C: RaftComms {
    pub fn new(
        id: String,
        n_reqs: i64,
        g_reqs: &Arc<AtomicI64>,
        running: &Arc<AtomicBool>,
        s_ids: Vec<String>,
        comms: C,
    ) -> Client<C> {
        Client {
            id: id,
            n_reqs: n_reqs,
            g_reqs: g_reqs.clone(),
            running: running.clone(),
            s_ids: s_ids,
            comms: comms,
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub fn run(&mut self) {
        let mut leader_idx = 0;
        let mut n = 0;

        // MAIN LOOP
        // while running {
        //      send a request to the leader (might need to discover leader first)
        //      wait for response - if failure or timeout then retry request
        // }
        while self.is_running() && n < self.n_reqs {
            let req_opid = random::<i64>();
            let req = make_client_request(req_opid);
            while self.is_running() {
                let leader_id = self.s_ids[leader_idx].clone();
                self.comms.send(leader_id, req.clone());
                match self.comms.recv_timeout(Duration::from_millis(100)) {
                    Some((id, RPC::ClientResponse {
                        opid,
                        success,
                    })) => {
                        if success && opid == req_opid {
                            debug!("client {} recved response from {} opid {}", self.id, id, opid);
                            n += 1;
                            break;
                        } else {
                            // failure -> try another server
                            leader_idx = (leader_idx + 1) % self.s_ids.len();
                        }
                    },
                    Some((_, rpc)) => panic!("client {} recved a bad RPC {:?}", self.id, rpc),
                    None => {},
                }
            }
            info!("client {} recved response for req {}, opid {}", self.id, n, req_opid);
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
