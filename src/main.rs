mod editline;

use std::env;
use std::io::{Write, stderr};
use std::process::exit;

use crate::editline::*;

use librustdbg::{self as rdb, Process};

fn print_stop_reason(proc: &Process, reason: rdb::StopReason) {
    let msg: String;
    match reason.reason {
        rdb::ProcessState::Exited => msg = format!("exited with status {}", reason.info as i32),
        rdb::ProcessState::Terminated => {
            msg = format!("terminated with signal {}", reason.info as i32)
        }
        rdb::ProcessState::Stopped => {
            msg = format!("stopped with signal {}", reason.info_string());
        }
        librustdbg::ProcessState::Running => todo!(),
    }
    println!("Process {} {}", proc.pid(), msg);
}

fn handle_command(proc: &mut Process, command: &str) {
    if "continue".starts_with(command) {
        // TODO: remove unwrap and deal with error
        proc.resume().unwrap();
        let reason = proc.wait_on_signal().unwrap();
        print_stop_reason(&proc, reason);
    }
}

fn main_loop(mut proc: Box<rdb::Process>) {
    loop {
        let line = readline("(rdb) ").unwrap();
        let mut line_str = String::new();

        if line == "" {
            if let Some(last_line) = history_list().last() {
                line_str = last_line.clone();
            }
        } else {
            line_str = line;
            add_history(&line_str);
        }

        if !line_str.is_empty() {
            // TODO: handle error
            handle_command(proc.as_mut(), &line_str);
        }
    }
}

fn attach(args: Vec<String>) -> Result<Box<rdb::Process>, rdb::Error> {
    if args.len() == 3 && args[1] == "-p" {
        // TODO: handle error
        match args[2].parse::<i32>() {
            Ok(pid) => return rdb::Process::attach(pid),
            Err(_) => return Err(rdb::Error::send("Pid must be a number")),
        };
    }
    rdb::Process::launch(String::from(&args[1]), true, None)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() == 1 {
        stderr()
            .write_all("No arguments given\n".as_bytes())
            .unwrap();
        exit(-1);
    }

    match attach(args) {
        Ok(p) => main_loop(p),
        // TODO: impl Display for Error
        Err(e) => println!("{:?}", e),
    };
}
