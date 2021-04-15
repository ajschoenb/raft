use std::collections::HashMap;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

pub mod server;
pub mod client;
pub mod rpc;

use rpc::*;
use server::Server;
use client::Client;

fn init_channels(n_servers: i32, n_clients: i32) -> (Vec<HashMap<i32, (Sender<RPC>, Receiver<RPC>)>>, Vec<HashMap<i32, (Sender<RPC>, Receiver<RPC>)>>, Vec<HashMap<i32, (Sender<RPC>, Receiver<RPC>)>>) {
    // for each server, build a channel to every other server and every client
    let mut ss_channels = vec![];
    let mut sc_channels = vec![];
    let mut cs_channels = vec![];

    for _ in 0..n_servers {
        ss_channels.push(HashMap::new());
        sc_channels.push(HashMap::new());
    }
    for _ in 0..n_clients {
        cs_channels.push(HashMap::new());
    }

    for i in 0..n_servers {
        for j in 0..n_servers {
            if i != j {
                // build communication between server i and server j
                let (txi, rxj) = channel();
                let (txj, rxi) = channel();
                ss_channels[i as usize].insert(j, (txi, rxi));
                ss_channels[j as usize].insert(i, (txj, rxj));
            }
        }
        for j in 0..n_clients {
            // build communication between server i and client j
            let (txi, rxj) = channel();
            let (txj, rxi) = channel();
            sc_channels[i as usize].insert(j, (txi, rxi));
            cs_channels[j as usize].insert(i, (txj, rxj));
        }
    }
    (ss_channels, sc_channels, cs_channels)
}

fn init_servers(n: i32, running: &Arc<AtomicBool>, mut ss_channels: Vec<HashMap<i32, (Sender<RPC>, Receiver<RPC>)>>, mut sc_channels: Vec<HashMap<i32, (Sender<RPC>, Receiver<RPC>)>>) -> Vec<Server> {
    let mut servers = vec![];

    for i in 0..n {
        let mut ss_map = ss_channels.remove(0);
        let mut sc_map = sc_channels.remove(0);
        let mut s_txs = HashMap::new();
        let mut s_rxs = HashMap::new();
        let mut c_txs = HashMap::new();
        let mut c_rxs = HashMap::new();
        for (j, ch) in ss_map.drain() {
            s_txs.insert(j, ch.0);
            s_rxs.insert(j, ch.1);
        }
        for (j, ch) in sc_map.drain() {
            c_txs.insert(j, ch.0);
            c_rxs.insert(j, ch.1);
        }
        let s = Server::new(i,
                            running,
                            s_txs,
                            s_rxs,
                            c_txs,
                            c_rxs);
        servers.push(s);
    }

    servers
}

fn init_clients(n: i32, n_reqs: i64, running: &Arc<AtomicBool>, mut cs_channels: Vec<HashMap<i32, (Sender<RPC>, Receiver<RPC>)>>) -> Vec<Client> {
    let mut clients = vec![];

    for i in 0..n {
        let mut cs_map = cs_channels.remove(0);
        let mut txs = HashMap::new();
        let mut rxs = HashMap::new();
        for (j, ch) in cs_map.drain() {
            txs.insert(j, ch.0);
            rxs.insert(j, ch.1);
        }
        let c = Client::new(i,
                            n_reqs,
                            running,
                            txs,
                            rxs);
        clients.push(c);
    }

    clients
}

fn launch(servers: Vec<Server>, clients: Vec<Client>) -> Vec<JoinHandle<()>> {
    let mut handles = vec![];

    for mut s in servers {
        handles.push(thread::spawn(move || {
            s.run();
        }));
    }

    for mut c in clients {
        handles.push(thread::spawn(move || {
            c.run();
        }));
    }

    handles
}

fn main() {
    let n_servers = 5;
    let n_clients = 10;
    let n_reqs = 10;
    let running = Arc::new(AtomicBool::new(true));
    let (ss_channels, sc_channels, cs_channels) = init_channels(n_servers, n_clients);
    let servers = init_servers(n_servers, &running.clone(), ss_channels, sc_channels);
    let clients = init_clients(n_clients, n_reqs, &running.clone(), cs_channels);
    let handles = launch(servers, clients);
    for h in handles {
        h.join().unwrap();
    }
}
