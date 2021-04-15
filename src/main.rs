extern crate ctrlc;
use std::collections::HashMap;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;
use std::thread::JoinHandle;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

pub mod server;
pub mod client;
pub mod rpc;

use rpc::*;
use server::Server;
use client::Client;

///
/// init_channels
/// Initializes communication channels
/// returns: (ss_senders, sc_senders, cs_senders, s_recvers, c_recvers)
///
fn init_channels(n_servers: i32, n_clients: i32) -> (Vec<HashMap<i32, Sender<RPC>>>, Vec<HashMap<i32, Sender<RPC>>>, Vec<HashMap<i32, Sender<RPC>>>, Vec<Receiver<RPC>>, Vec<Receiver<RPC>>) {
    // for each server, build a channel to every other server and every client
    let mut s_channels = vec![];
    let mut c_channels = vec![];
    let mut ss_senders = vec![];
    let mut sc_senders = vec![];
    let mut cs_senders = vec![];

    for _ in 0..n_servers {
        ss_senders.push(HashMap::new());
        sc_senders.push(HashMap::new());
        s_channels.push(channel());
    }
    for _ in 0..n_clients {
        cs_senders.push(HashMap::new());
        c_channels.push(channel());
    }

    for i in 0..n_servers {
        for j in 0..n_servers {
            if i != j {
                // build communication between server i and server j
                let txi = s_channels[i as usize].0.clone();
                let txj = s_channels[j as usize].0.clone();
                ss_senders[i as usize].insert(j, txj);
                ss_senders[j as usize].insert(i, txi);
            }
        }
        for j in 0..n_clients {
            // build communication between server i and client j
            let txi = s_channels[i as usize].0.clone();
            let txj = c_channels[j as usize].0.clone();
            sc_senders[i as usize].insert(j, txj);
            cs_senders[j as usize].insert(i, txi);
        }
    }

    let s_recvers = s_channels.into_iter().map(|(_, rx)| rx).collect();
    let c_recvers = c_channels.into_iter().map(|(_, rx)| rx).collect();
    (ss_senders, sc_senders, cs_senders, s_recvers, c_recvers)
}

///
/// init_servers
/// Initializes servers
/// returns: (servers)
///
fn init_servers(n: i32, running: &Arc<AtomicBool>, mut ss_senders: Vec<HashMap<i32, Sender<RPC>>>, mut sc_senders: Vec<HashMap<i32, Sender<RPC>>>, mut s_recvers: Vec<Receiver<RPC>>) -> Vec<Server> {
    let mut servers = vec![];

    for i in 0..n {
        let s_txs = ss_senders.remove(0);
        let c_txs = sc_senders.remove(0);
        let rx = s_recvers.remove(0);
        let s = Server::new(i,
                            running,
                            s_txs,
                            c_txs,
                            rx);
        servers.push(s);
    }

    servers
}

///
/// init_clients
/// Initializes clients
/// returns: (clients)
///
fn init_clients(n: i32, n_reqs: i64, running: &Arc<AtomicBool>, mut cs_senders: Vec<HashMap<i32, Sender<RPC>>>, mut c_recvers: Vec<Receiver<RPC>>) -> Vec<Client> {
    let mut clients = vec![];

    for i in 0..n {
        let txs = cs_senders.remove(0);
        let rx = c_recvers.remove(0);
        let c = Client::new(i,
                            n_reqs,
                            running,
                            txs,
                            rx);
        clients.push(c);
    }

    clients
}

///
/// launch
/// Launches clients and servers
/// returns: (handles)
///
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
    let r = running.clone();
    ctrlc::set_handler(move || {
        println!("CTRL-C");
        r.store(false, Ordering::SeqCst);
    }).expect("error setting signal handler");

    let (ss_senders, sc_senders, cs_senders, s_recvers, c_recvers) = init_channels(n_servers, n_clients);
    let servers = init_servers(n_servers, &running.clone(), ss_senders, sc_senders, s_recvers);
    let clients = init_clients(n_clients, n_reqs, &running.clone(), cs_senders, c_recvers);
    let handles = launch(servers, clients);
    for h in handles {
        h.join().unwrap();
    }
}
