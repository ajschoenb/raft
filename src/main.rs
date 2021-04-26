extern crate ctrlc;
extern crate stderrlog;
use std::collections::HashMap;
use std::sync::mpsc::{channel};
use std::thread;
use std::thread::JoinHandle;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use std::net::{UdpSocket, SocketAddr};

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
use comms::{RaftChannelComms, RaftSocketComms};

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

fn init_servers(n: i32, running: &Arc<AtomicBool>, s_ids: Vec<String>, comms: &mut HashMap<String, RaftChannelComms>, logpathbase: String) -> Vec<Server<RaftChannelComms>> {
    let mut servers = vec![];

    for i in 0..n {
        let id = s_id(i);
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

fn run_local(opts: Opts, logpathbase: String) {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        println!("CTRL-C");
        r.store(false, Ordering::SeqCst);
    }).expect("error setting signal handler");

    let mut comms = init_comms(opts.n_servers, opts.n_clients);
    let mut s_ids = vec![];
    for i in 0..opts.n_servers { s_ids.push(s_id(i)) }
    let servers = init_servers(opts.n_servers, &running.clone(), s_ids.clone(), &mut comms, logpathbase);
    let clients = init_clients(opts.n_clients, opts.n_request, &running.clone(), s_ids, &mut comms);
    let handles = launch(servers, clients);
    for h in handles {
        h.join().unwrap();
    }
}

fn run_dist(opts: Opts, logpathbase: String) {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        println!("CTRL-C");
        r.store(false, Ordering::SeqCst);
    }).expect("error setting signal handler");

    // load candidate socket addrs from hosts file
    let hostsfile = File::open(format!("{}/hosts.txt", logpathbase)).unwrap();
    let mut hosts = vec![];
    let mut reader = BufReader::new(&hostsfile);
    let mut line = String::new();
    let mut len = reader.read_line(&mut line).unwrap();
    while len > 0 {
        if !line.starts_with("#") {
            hosts.push(line.replace("\n", ""));
        }
        line = String::new();
        len = reader.read_line(&mut line).unwrap();
    }

    if opts.isclient {
        let g_reqs = Arc::new(AtomicI64::new(opts.n_request));
        let socket = UdpSocket::bind("0.0.0.0:5800").unwrap();
        let mut client = Client::new(
            socket.local_addr().unwrap().to_string(),
            opts.n_request,
            &g_reqs,
            &running,
            hosts,
            RaftSocketComms::new(socket),
        );
        client.run();
    } else {
        // create socket from one of the candidate hosts
        let addrs: Vec<SocketAddr> = hosts.iter().map(|h| h.parse().unwrap()).collect();
        let socket = UdpSocket::bind(&addrs[..]).unwrap();
        let addr = socket.local_addr().unwrap().to_string();
        let mut server = Server::new(
            addr.clone(),
            &running,
            format!("{}/{}.log", logpathbase, addr),
            hosts,
            RaftSocketComms::new(socket),
        );
        server.run();
    }
}

fn clean(path: String) {
    for p in fs::read_dir(path).unwrap() {
        let p = p.unwrap().path();
        if !p.to_string_lossy().into_owned().ends_with("hosts.txt") {
            fs::remove_file(p).unwrap();
        }
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

    let logpathbase = shellexpand::tilde("~/raft").to_string();

    match opts.mode.as_ref() {
        "local" => run_local(opts, logpathbase),
        "dist" => run_dist(opts, logpathbase),
        "check" => checker::check(opts, logpathbase),
        "clean" => clean(logpathbase),
        _ => panic!("unknown mode"),
    }
}
