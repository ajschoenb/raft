extern crate ctrlc;
extern crate stderrlog;
use std::collections::HashMap;
use std::sync::mpsc::{channel};
use std::thread;
use std::thread::JoinHandle;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};

pub mod server;
pub mod client;
pub mod rpc;
pub mod raftlog;
pub mod opts;
pub mod checker;
pub mod comms;

use server::Server;
use client::Client;
use opts::Opts;
use comms::RaftChannelComms;

fn s_id(id: i32) -> String {
    format!("server{}", id)
}

fn c_id(id: i32) -> String {
    format!("client{}", id)
}

fn init_comms(n_servers: i32, n_clients: i32) -> HashMap<String, RaftChannelComms> {
    // for each server, build a channel to every other server and every client
    let mut channels = HashMap::new();
    let mut senders = HashMap::new();

    for i in 0..n_servers {
        let id = s_id(i);
        senders.insert(id.clone(), HashMap::new());
        channels.insert(id, channel());
    }
    for i in 0..n_clients {
        let id = c_id(i);
        senders.insert(id.clone(), HashMap::new());
        channels.insert(id, channel());
    }

    for i in 0..n_servers {
        let idi = s_id(i);
        for j in 0..n_servers {
            if i != j {
                // build communication between server i and server j
                let idj = s_id(j);
                let txi = channels[&idi].0.clone();
                let txj = channels[&idj].0.clone();
                senders.get_mut(&idi).unwrap().insert(idj.clone(), txj);
                senders.get_mut(&idj).unwrap().insert(idi.clone(), txi);
            }
        }
        for j in 0..n_clients {
            // build communication between server i and client j
            let idj = c_id(j);
            let txi = channels[&idi].0.clone();
            let txj = channels[&idj].0.clone();
            senders.get_mut(&idi).unwrap().insert(idj.clone(), txj);
            senders.get_mut(&idj).unwrap().insert(idi.clone(), txi);
        }
    }

    let mut comms = HashMap::new();
    for i in 0..n_servers {
        let id = s_id(i);
        comms.insert(id.clone(), RaftChannelComms::new(id.clone(), senders.remove(&id).unwrap(), channels.remove(&id).unwrap().1));
    }
    for i in 0..n_clients {
        let id = c_id(i);
        comms.insert(id.clone(), RaftChannelComms::new(id.clone(), senders.remove(&id).unwrap(), channels.remove(&id).unwrap().1));
    }

    comms
}

fn init_servers(n: i32, running: &Arc<AtomicBool>, s_ids: Vec<String>, comms: &mut HashMap<String, RaftChannelComms>) -> Vec<Server<RaftChannelComms>> {
    let mut servers = vec![];

    for i in 0..n {
        let id = s_id(i);
        let logpathbase = shellexpand::tilde("~/raft");
        let lpath = format!("{}/{}.log", logpathbase, id.clone());
        let s = Server::new(id.clone(),
                            running,
                            lpath,
                            s_ids.clone(),
                            comms.remove(&id).unwrap());
        servers.push(s);
    }

    servers
}

fn init_clients(n: i32, n_reqs: i64, running: &Arc<AtomicBool>, s_ids: Vec<String>, comms: &mut HashMap<String, RaftChannelComms>) -> Vec<Client<RaftChannelComms>> {
    let mut clients = vec![];

    let g_reqs = Arc::new(AtomicI64::new((n as i64) * n_reqs));

    for i in 0..n {
        let id = c_id(i);
        let c = Client::new(id.clone(),
                            n_reqs,
                            &g_reqs,
                            running,
                            s_ids.clone(),
                            comms.remove(&id).unwrap());
        clients.push(c);
    }

    clients
}

fn launch(servers: Vec<Server<RaftChannelComms>>, clients: Vec<Client<RaftChannelComms>>) -> Vec<JoinHandle<()>> {
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

fn run(opts: Opts) {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        println!("CTRL-C");
        r.store(false, Ordering::SeqCst);
    }).expect("error setting signal handler");

    let mut comms = init_comms(opts.n_servers, opts.n_clients);
    let mut s_ids = vec![];
    for i in 0..opts.n_servers { s_ids.push(s_id(i)) }
    let servers = init_servers(opts.n_servers, &running.clone(), s_ids.clone(), &mut comms);
    let clients = init_clients(opts.n_clients, opts.n_request, &running.clone(), s_ids, &mut comms);
    let handles = launch(servers, clients);
    for h in handles {
        h.join().unwrap();
    }
}

fn main() {
    let opts = Opts::new();
    stderrlog::new()
            .module(module_path!())
            .quiet(false)
            .timestamp(stderrlog::Timestamp::Millisecond)
            .verbosity(opts.verbosity)
            .init()
            .unwrap();

    match opts.mode.as_ref() {
        "run" => run(opts),
        "check" => checker::check(opts, shellexpand::tilde("~/raft").to_string()),
        _ => panic!("unknown mode"),
    }
}
