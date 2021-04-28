use std::collections::HashMap;
use std::sync::mpsc::{Sender, Receiver};
use std::net::UdpSocket;
use std::str;

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
    socket: UdpSocket,
}

impl RaftSocketComms {
    pub fn new(
        socket: UdpSocket,
    ) -> RaftSocketComms {
        socket.set_nonblocking(true).unwrap();
        RaftSocketComms {
            socket: socket,
        }
    }

}

impl RaftComms for RaftSocketComms {
    fn send(&self, addr: String, rpc: RPC) {
        let s_rpc = serde_json::to_string(&rpc).unwrap();
        let buf = s_rpc.as_bytes();
        assert!(buf.len() < 576);
        self.socket.send_to(buf, addr).unwrap();
    }

    fn try_recv(&self) -> Option<(String, RPC)> {
        let mut buf = [0; 1000];
        match self.socket.recv_from(&mut buf) {
            Ok((n, addr)) => {
                Some((addr.to_string(), serde_json::from_str(str::from_utf8(&buf[..n]).unwrap()).unwrap()))
            },
            Err(_) => None,
        }
    }
}
