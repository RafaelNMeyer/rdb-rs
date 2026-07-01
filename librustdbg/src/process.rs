use std::process::exit;
use std::ptr::{self, null};

use crate::error::{Error, errno_string};
use crate::{Pipe, WEXITSTATUS, WIFEXITED, WIFSIGNALED, WIFSTOPPED, WSTOPSIG, WTERMSIG};

use crate::bindings::{
    PTRACE_REQUEST::*, SIGNALS::*, STDOUT_FILENO, c_char, char_ptr_to_string, dup2, execlp, fork,
    kill, pid_t, ptrace, sigabbrev_np, waitpid,
};

pub struct Process {
    pid: pid_t,
    terminate_on_end: bool,
    is_attached: bool,
    state: ProcessState,
}

pub enum ProcessState {
    Stopped,
    Running,
    Exited,
    Terminated,
}

pub struct StopReason {
    pub reason: ProcessState,
    pub info: u8,
}

impl StopReason {
    pub fn info_string(&self) -> String {
        unsafe { char_ptr_to_string(sigabbrev_np(self.info as i32)) }
    }
}

impl Process {
    fn new(pid: pid_t, terminate_on_end: bool, is_attached: bool) -> Process {
        Process {
            pid,
            terminate_on_end,
            is_attached,
            state: ProcessState::Stopped,
        }
    }

    pub fn attach(pid: pid_t) -> Result<Box<Process>, Error> {
        if pid <= 0 {
            return Err(Error::send("Invalid pid"));
        }

        unsafe {
            if ptrace(
                PTRACE_ATTACH,
                pid,
                /*addr=*/ null(),
                /*data=*/ null(),
            ) < 0
            {
                return Err(Error::send_errno("Couldn't attach to process"));
            }
        }
        let mut proc = Box::new(Self::new(
            pid, /*terminate_on_end*/ false, /*is_attached*/ true,
        ));
        proc.wait_on_signal()?;

        Ok(proc)
    }

    pub fn launch(
        mut path: String,
        debug: bool,
        stdout_replacement: Option<i32>,
    ) -> Result<Box<Process>, Error> {
        let mut channel = Pipe::new(true);
        unsafe {
            let pid = fork();
            if pid < 0 {
                return Err(Error::send_errno("Fork failed"));
            }
            if pid == 0 {
                channel.close_read();

                if let Some(stdout) = stdout_replacement {
                    if dup2(stdout, STDOUT_FILENO) < 0 {
                        exit_with_perror(&channel, "Stdout replacement failed");
                    }
                }

                if debug && ptrace(PTRACE_TRACEME, pid, ptr::null(), ptr::null()) < 0 {
                    exit_with_perror(&channel, "Traceme failed");
                }

                if execlp(path.to_cstring(), path.to_cstring(), null::<c_char>()) < 0 {
                    exit_with_perror(&channel, "Execlp failed");
                }
            }
            channel.close_write();
            let data = channel.read();
            channel.close_read();

            if data.len() > 0 {
                if waitpid(pid, ptr::null_mut(), 0) < 0 {
                    return Err(Error::send_errno("Waitpid failed"));
                };
                let chars =
                    str::from_utf8(&data[..]).unwrap_or_else(|_| "Error building chars from pipe");
                return Err(Error::send(chars));
            }

            let mut proc = Box::new(Self::new(
                pid, /*terminate_on_end*/ true, /*is_attached*/ debug,
            ));

            if debug {
                proc.wait_on_signal()?;
            }

            Ok(proc)
        }
    }

    pub fn resume(&mut self) -> Result<(), Error> {
        unsafe {
            if ptrace(PTRACE_CONT, self.pid, ptr::null(), ptr::null()) < 0 {
                return Err(Error::send_errno("Could not resume"));
            }
        }
        self.state = ProcessState::Running;
        Ok(())
    }

    pub fn wait_on_signal(&mut self) -> Result<StopReason, Error> {
        let mut wait_status = 0;
        let options = 0;
        unsafe {
            if waitpid(self.pid, &mut wait_status as *mut i32, options) < 0 {
                return Err(Error::send_errno("Waitpid failed"));
            }
        }
        let reason = StopReason::new(wait_status);
        self.state = reason.reason;

        if self.is_attached && matches!(self.state, ProcessState::Stopped) {
            // read_all_registers();
        }

        Ok(reason)
    }

    pub fn pid(&self) -> pid_t {
        self.pid
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        if self.pid != 0 {
            let mut status: i32 = 0;
            if self.is_attached {
                if matches!(self.state, ProcessState::Running) {
                    unsafe {
                        kill(self.pid, SIGSTOP);
                        waitpid(self.pid, &mut status as *mut i32, 0);
                    }
                }
                unsafe {
                    ptrace(PTRACE_DETACH, self.pid, null(), null());
                    kill(self.pid, SIGCONT);
                }
            }
            if self.terminate_on_end {
                unsafe {
                    kill(self.pid, SIGINT);
                    waitpid(self.pid, &mut status as *mut i32, 0);
                }
            }
        }
    }
}

impl StopReason {
    fn new(wait_status: i32) -> StopReason {
        let mut process_state = ProcessState::Stopped;
        let mut info: u8 = 0;
        if WIFEXITED!(wait_status) {
            process_state = ProcessState::Exited;
            info = WEXITSTATUS!(wait_status);
        } else if WIFSIGNALED!(wait_status) {
            process_state = ProcessState::Terminated;
            info = WTERMSIG!(wait_status);
        } else if WIFSTOPPED!(wait_status) {
            process_state = ProcessState::Stopped;
            info = WSTOPSIG!(wait_status);
        }
        StopReason {
            reason: process_state,
            info,
        }
    }
}

impl Copy for ProcessState {}

impl Clone for ProcessState {
    fn clone(&self) -> Self {
        *self
    }
}

fn exit_with_perror(channel: &Pipe, prefix: &str) {
    let message = format!("{}: {}", prefix, errno_string());
    channel.write(&message);
    exit(-1);
}

// TODO: Verify if this is the best way to do it
trait CSTRING {
    fn to_cstring<'a>(&mut self) -> *const c_char;
}

impl CSTRING for String {
    fn to_cstring<'a>(&mut self) -> *const c_char {
        self.push('\0');
        self.as_ptr()
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::read_to_string;
    use std::process::Command;
    use std::sync::Once;

    use crate::bindings::{__errno_location, ESRCH};

    use super::*;

    static BUILD: Once = Once::new();

    fn build_targets() {
        BUILD.call_once(|| {
            for (src, out) in [
                ("test/targets/run_endlessly.rs", "target/run_endlessly"),
                ("test/targets/end_immediately.rs", "target/end_immediately"),
            ] {
                let output = Command::new("rustc")
                    .args([src, "-o", out])
                    .output()
                    .expect("failed to run rustc");
                assert!(output.status.success());
            }
        });
    }

    fn process_exists(pid: pid_t) -> bool {
        unsafe {
            let ret = kill(pid, NOSIGNAL);
            return ret != -1 && *__errno_location() != ESRCH;
        }
    }

    fn get_process_status(pid: pid_t) -> char {
        let stat = File::open(format!("/proc/{}/stat", pid)).unwrap();
        let data = read_to_string(stat).unwrap();
        let index_of_last_parenthesis = data.rfind(')').unwrap();
        let index_of_status_indicator = index_of_last_parenthesis + 2;
        data.chars().nth(index_of_status_indicator).unwrap()
    }

    #[test]
    fn process_launch_success() {
        let proc = Process::launch("yes".to_string(), false, None).unwrap();
        assert!(process_exists(proc.pid));
    }

    #[test]
    #[should_panic]
    fn process_launch_no_such_program() {
        Process::launch("fade_to_black".to_string(), false, None).unwrap();
    }

    #[test]
    fn process_attach_success() {
        build_targets();
        let target = Process::launch("target/run_endlessly".to_string(), false, None).unwrap();
        // Need the _var to bind the value until the end of running.
        // Instead it'll drop the value immediately
        let _attached = Process::attach(target.pid).unwrap();
        assert_eq!(get_process_status(target.pid), 't');
    }

    #[test]
    #[should_panic]
    fn process_attach_invalid_pid() {
        Process::attach(0).unwrap();
    }

    #[test]
    fn process_resume_success() {
        build_targets();
        {
            let mut proc = Process::launch("target/run_endlessly".to_string(), true, None).unwrap();
            proc.resume().unwrap();
            let status = get_process_status(proc.pid);
            let success = status == 'R' || status == 'S';
            assert!(success);
        }
        {
            let target = Process::launch("target/run_endlessly".to_string(), false, None).unwrap();
            let mut proc = Process::attach(target.pid).unwrap();
            proc.resume().unwrap();
            let status = get_process_status(proc.pid);
            let success = status == 'R' || status == 'S';
            assert!(success);
        }
    }

    #[test]
    fn process_resume_already_terminated() {
        build_targets();
        let mut proc = Process::launch("target/end_immediately".to_string(), true, None).unwrap();

        proc.resume().unwrap();
        proc.wait_on_signal().unwrap();
        // Resume should return Err so we explicit unwrap to an Err!
        proc.resume().unwrap_err();
    }
}
