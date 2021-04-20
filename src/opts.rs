extern crate clap;
use clap::{Arg, App};

#[derive(Clone, Debug)]
pub struct Opts {
    pub n_servers: i32,
    pub n_clients: i32,
    pub n_request: i64,
    pub verbosity: usize,
    pub mode: String,
}

impl Opts {
    pub fn new() -> Opts {
        let _n_servers = "5";
        let _n_clients = "10";
        let _n_request = "100";
        let _verbosity = "0";
        let _mode = "run";

        let matches = App::new("raft")
            .version("0.1.0")
            .author("Adam Schoenberg <ajschoenb@utexas.edu>")
            .about("Basic implementation of the Raft protocol")
            .arg(Arg::with_name("n_servers")
                .short("s")
                .required(false)
                .takes_value(true)
                .help("number of servers"))
            .arg(Arg::with_name("n_clients")
                .short("c")
                .required(false)
                .takes_value(true)
                .help("number of clients"))
            .arg(Arg::with_name("n_request")
                .short("r")
                .required(false)
                .takes_value(true)
                .help("number of requests per client"))
            .arg(Arg::with_name("verbosity")
                .short("v")
                .required(false)
                .takes_value(true)
                .help("output verbosity"))
            .arg(Arg::with_name("mode")
                .short("m")
                .required(false)
                .takes_value(true)
                .help("mode--\"run\" runs Raft, \"check\" checks logs produced by previous run"))
            .get_matches();

        let n_servers = matches.value_of("n_servers").unwrap_or(_n_servers).parse::<i32>().unwrap();
        let n_clients = matches.value_of("n_clients").unwrap_or(_n_clients).parse::<i32>().unwrap();
        let n_request = matches.value_of("n_request").unwrap_or(_n_request).parse::<i64>().unwrap();
        let verbosity = matches.value_of("verbosity").unwrap_or(_verbosity).parse::<usize>().unwrap();
        let mode = matches.value_of("mode").unwrap_or(_mode);

        match mode.as_ref() {
            "run" => {},
            "check" => {},
            _ => panic!("unknown execution mode requested"),
        }

        Opts {
            n_servers: n_servers,
            n_clients: n_clients,
            n_request: n_request,
            verbosity: verbosity,
            mode: mode.to_string(),
        }
    }
}
