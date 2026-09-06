use std::cell::RefCell;
use std::process::exit;
use std::ptr::{self, null};
use std::rc::Rc;

use crate::error::{Error, errno_string};
use crate::register_info::{RegisterId, register_info_by_id};
use crate::registers::Registers;
use crate::user::{user_fpregs_struct, user_regs_struct};
use crate::{Pipe, WEXITSTATUS, WIFEXITED, WIFSIGNALED, WIFSTOPPED, WSTOPSIG, WTERMSIG};

use crate::bindings::{
    __errno_location, PTRACE_REQUEST::*, SIGNALS::*, STDOUT_FILENO, c_char, char_ptr_to_string,
    dup2, execlp, fork, kill, pid_t, ptrace, sigabbrev_np, waitpid,
};

pub struct Process {
    pid: pid_t,
    terminate_on_end: bool,
    is_attached: bool,
    state: ProcessState,
    pub registers: Option<Registers>,
}

#[derive(Debug)]
pub enum ProcessState {
    Stopped,
    Running,
    Exited,
    Terminated,
}

#[derive(Debug)]
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
    pub fn new(pid: pid_t, terminate_on_end: bool, is_attached: bool) -> Rc<RefCell<Process>> {
        let proc = Process {
            pid,
            terminate_on_end,
            is_attached,
            state: ProcessState::Stopped,
            registers: None,
        };
        let proc_ref = Rc::new(RefCell::new(proc));
        let regs = Registers::new(Rc::downgrade(&proc_ref));
        proc_ref.borrow_mut().registers = Some(regs);
        proc_ref
    }

    pub fn attach(pid: pid_t) -> Result<Rc<RefCell<Process>>, Error> {
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
        // let mut proc = Box::new(Self::new(
        //     pid, /*terminate_on_end*/ false, /*is_attached*/ true,
        // ));
        let proc = Self::new(
            pid, /*terminate_on_end*/ false, /*is_attached*/ true,
        );
        proc.borrow_mut().wait_on_signal()?;

        Ok(proc)
    }

    pub fn launch(
        mut path: String,
        debug: bool,
        stdout_replacement: Option<i32>,
    ) -> Result<Rc<RefCell<Process>>, Error> {
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

            let proc = Self::new(
                pid, /*terminate_on_end*/ true, /*is_attached*/ debug,
            );

            if debug {
                proc.borrow_mut().wait_on_signal()?;
            }

            Ok(proc)
        }
    }

    pub fn resume(&mut self) -> Result<(), Error> {
        unsafe {
            if ptrace(PTRACE_CONT, self.pid, null(), null()) < 0 {
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
            self.read_all_registers()?;
        }

        Ok(reason)
    }

    fn read_all_registers(&mut self) -> Result<(), Error> {
        let mut regs = self.registers.take().unwrap();
        unsafe {
            if ptrace(
                PTRACE_GETREGS,
                self.pid,
                null(),
                (&regs.data.regs as *const user_regs_struct).cast::<u64>(),
            ) < 0
            {
                return Err(Error::send_errno("Could not read GPR registers"));
            }
            if ptrace(
                PTRACE_GETFPREGS,
                self.pid,
                null(),
                (&regs.data.i387 as *const user_fpregs_struct).cast::<u64>(),
            ) < 0
            {
                return Err(Error::send_errno("Could not read FPR registers"));
            }
            let mut id: RegisterId;
            let mut i = 0;
            while i < 8 {
                match i {
                    0 => id = RegisterId::dr0,
                    1 => id = RegisterId::dr1,
                    2 => id = RegisterId::dr2,
                    3 => id = RegisterId::dr3,
                    4 => id = RegisterId::dr4,
                    5 => id = RegisterId::dr5,
                    6 => id = RegisterId::dr6,
                    7 => id = RegisterId::dr7,
                    _ => return Err(Error::send("debugger reg does not exists")),
                }
                let info = register_info_by_id(id)?;
                let data: i64 = ptrace(
                    PTRACE_PEEKUSER,
                    self.pid,
                    (info.offset as *const usize).cast::<u64>(),
                    null(),
                );
                if *__errno_location() != 0 {
                    return Err(Error::send_errno("Could not read from debug registers"));
                }
                regs.data.u_debugreg[i] = data as u64;
                i += 1;
            }
            self.registers = Some(regs);
            Ok(())
        }
    }

    pub fn pid(&self) -> pid_t {
        self.pid
    }

    pub fn write_fprs(&self, fprs: &user_fpregs_struct) -> Result<(), Error> {
        unsafe {
            if ptrace(
                PTRACE_SETFPREGS,
                self.pid,
                null(),
                (fprs as *const user_fpregs_struct).cast::<u64>(),
            ) < 0
            {
                return Err(Error::send_errno("Could not write to user area"));
            }
        }
        Ok(())
    }

    pub fn write_user_area(&self, offset: usize, data: u64) -> Result<(), Error> {
        let off: u64 = match offset.try_into() {
            Ok(v) => v,
            Err(_) => return Err(Error::send("could not transform offset usize to u64")),
        };
        unsafe {
            if ptrace(
                PTRACE_POKEUSER,
                self.pid,
                (off as *const usize).cast::<u64>(),
                data as *const u64,
            ) < 0
            {
                return Err(Error::send_errno("Could not write to user area"));
            }
        }
        Ok(())
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
        build_targets();
        let proc = Process::launch("target/run_endlessly".to_string(), false, None).unwrap();
        assert!(process_exists(proc.borrow_mut().pid));
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
        let _attached = Process::attach(target.borrow().pid).unwrap();
        assert_eq!(get_process_status(target.borrow().pid), 't');
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
            let proc = Process::launch("target/run_endlessly".to_string(), true, None).unwrap();
            proc.borrow_mut().resume().unwrap();
            let status = get_process_status(proc.borrow().pid);
            let success = status == 'R' || status == 'S';
            assert!(success);
        }
        {
            let target = Process::launch("target/run_endlessly".to_string(), false, None).unwrap();
            let proc = Process::attach(target.borrow().pid).unwrap();
            proc.borrow_mut().resume().unwrap();
            let status = get_process_status(proc.borrow().pid);
            let success = status == 'R' || status == 'S';
            assert!(success);
        }
    }

    #[test]
    fn process_resume_already_terminated() {
        build_targets();
        let proc = Process::launch("target/end_immediately".to_string(), true, None).unwrap();

        proc.borrow_mut().resume().unwrap();
        proc.borrow_mut().wait_on_signal().unwrap();
        // Resume should return Err so we explicit unwrap to an Err!
        proc.borrow_mut().resume().unwrap_err();
    }
}
