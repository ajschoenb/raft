use std::collections::HashMap;
use std::sync::mpsc::{Sender, Receiver};

use crate::rpc::RPC;

pub trait RaftComms {
    fn send(&self, addr: String, rpc: RPC);
    fn try_recv(&self) -> Option<(String, RPC)>;
}

pub struct RaftChannelComms {
    id: String,
    txs: HashMap<String, Sender<(String, RPC)>>,
    rx: Receiver<(String, RPC)>,
}

impl RaftChannelComms {
    pub fn new(
        id: String,
        txs: HashMap<String, Sender<(String, RPC)>>,
        rx: Receiver<(String, RPC)>,
    ) -> RaftChannelComms {
        RaftChannelComms {
            id: id,
            txs: txs,
            rx: rx,
        }
    }
}

impl RaftComms for RaftChannelComms {
    fn send(&self, addr: String, rpc: RPC) {
        assert!(addr != self.id);
        // if addr != self.id {
            self.txs.get(&addr).unwrap().send((self.id.clone(), rpc)).unwrap();
        // }
    }

    fn try_recv(&self) -> Option<(String, RPC)> {
        match self.rx.try_recv() {
            Ok(m) => Some(m),
            Err(_) => None,
        }
    }
}

pub struct RaftSocketComms {
}
