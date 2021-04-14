use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;
use std::time::Duration;

pub mod server;
pub mod rpc;

use rpc::*;

fn main() {
    let (tx, rx): (Sender<RPC>, Receiver<RPC>) = channel();
    let mut handles = vec![];
    handles.push(thread::spawn(move || {
        tx.send(make_append_entries(0, 1, 2, 3, 4)).unwrap();
        thread::sleep(Duration::from_secs(1));
    }));

    handles.push(thread::spawn(move || {
        println!("{:?}", rx.recv().unwrap());
        thread::sleep(Duration::from_secs(1));
    }));

    for h in handles { h.join().unwrap(); }
}
