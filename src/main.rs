#![allow(unknown_lints)]
extern crate errno;
extern crate glob;
extern crate libc;
extern crate linefeed;
extern crate nix;
extern crate regex;
extern crate sqlite;
extern crate time;
extern crate yaml_rust;
extern crate exec;

#[macro_use]
extern crate nom;

use std::env;
use std::rc::Rc;

// use std::thread;
// use std::time::Duration;

// use ansi_term::Colour::{Red, Green};

use linefeed::Command;
use linefeed::{Reader, ReadResult};

use std::io::{self, Read};

#[macro_use]
mod tools;
mod binds;
mod builtins;
mod completers;
mod execute;
mod libs;
mod parsers;
mod history;
mod rcfile;
mod shell;


fn main() {
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    if let Ok(env_path) = env::var("PATH") {
        let env_path_new = format!("/usr/local/bin:{}", env_path);
        env::set_var("PATH", &env_path_new);
    } else {
        env::set_var("PATH", "/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin");
    }
    let mut sh = shell::Shell::new();
    rcfile::load_rcfile(&mut sh);

    if env::args().len() > 1 {
        let line = tools::env_args_to_command_line();
        execute::run_procs(&mut sh, &line, false);
        return;
    }

    let isatty: bool = unsafe { libc::isatty(0) == 1 };
    if !isatty {
        // cases like open a new MacVim window on an existing one
        let mut buffer = String::new();
        let stdin = io::stdin();
        let mut handle = stdin.lock();
        match handle.read_to_string(&mut buffer) {
            Ok(_) => {
                execute::run_procs(&mut sh, &buffer, false);
            }
            Err(e) => {
                println!("cicada: io stdin read_to_string failed: {:?}", e);
            }
        }
        return;
    }

    let mut rl;
    match Reader::new("cicada") {
        Ok(x) => rl = x,
        Err(e) => {
            // non-tty will raise errors here
            println!("Reader Error: {:?}", e);
            return;
        }
    }
    history::init(&mut rl);
    rl.set_completer(Rc::new(completers::CicadaCompleter));

    rl.define_function("up-key-function", Rc::new(binds::UpKeyFunction));
    rl.bind_sequence(binds::SEQ_UP_KEY, Command::from_str("up-key-function"));
    rl.define_function("down-key-function", Rc::new(binds::DownKeyFunction));
    rl.bind_sequence(binds::SEQ_DOWN_KEY, Command::from_str("down-key-function"));

    let mut status = 0;
    loop {
        let prompt = libs::prompt::get_prompt(status);
        rl.set_prompt(&prompt);
        match rl.read_line() {
            Ok(ReadResult::Input(line)) => {
                let mut cmd;
                if line.trim() == "exit" {
                    break;
                } else if line.trim() == "" {
                    continue;
                } else if line.trim() == "version" {
                    println!("Cicada v{} by @mitnk", VERSION);
                    continue;
                } else if line.trim() == "bash" {
                    cmd = String::from("bash --rcfile ~/.bash_profile");
                } else {
                    cmd = line.clone();
                }

                let tsb_spec = time::get_time();
                let tsb = (tsb_spec.sec as f64) + tsb_spec.nsec as f64 / 1000000000.0;
                tools::pre_handle_cmd_line(&sh, &mut cmd);
                status = execute::run_procs(&mut sh, &cmd, true);
                let tse_spec = time::get_time();
                let tse = (tse_spec.sec as f64) + tse_spec.nsec as f64 / 1000000000.0;
                sh.previous_status = status;
                history::add(&mut sh, &mut rl, &line, status, tsb, tse);
            }
            Ok(ReadResult::Eof) => {
                println!("");
                continue;
            }
            Ok(ReadResult::Signal(s)) => {
                println!("readline signal: {:?}", s);
            }
            Err(e) => {
                println!("readline error: {:?}", e);
            }
        }
    }
}
