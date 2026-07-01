#[allow(non_camel_case_types)]
pub type c_char = u8;
#[allow(non_camel_case_types)]
pub type pid_t = i32;

pub const O_CLOEXEC: i32 = 0x80000;
pub const STDOUT_FILENO: i32 = 1;

#[cfg(test)]
pub const ESRCH: i32 = 3;

#[allow(non_camel_case_types)]
#[repr(C)]
pub enum PTRACE_REQUEST {
    PTRACE_TRACEME,
    PTRACE_CONT = 7,
    PTRACE_ATTACH = 16,
    PTRACE_DETACH = 17,
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub enum SIGNALS {
    #[cfg(test)]
    NOSIGNAL = 0,

    SIGINT = 2,
    SIGCONT = 18,
    SIGSTOP = 19,
}

unsafe extern "C" {
    pub fn fork() -> pid_t;
    pub fn waitpid(pid: pid_t, status: *mut i32, options: i32) -> pid_t;
    pub fn execlp(file: *const c_char, arg: *const c_char, ...) -> i32;
    pub fn ptrace(op: PTRACE_REQUEST, pid: pid_t, addr: *const u32, data: *const u32) -> i32;
    pub fn kill(pid: pid_t, sig: SIGNALS) -> i32;

    pub fn dup2(fd: i32, fd2: i32) -> i32;
    pub fn pipe2(pipefd: *const i32, flags: i32) -> i32;
    pub fn read(fd: i32, buf: *mut c_char, count: usize) -> i32;
    pub fn write(fd: i32, buf: *const c_char, count: usize) -> i32;
    pub fn close(fd: i32) -> i32;

    pub fn __errno_location() -> *const i32;
    pub fn strerror(errno: i32) -> *const c_char;

    pub fn sigabbrev_np(sig: i32) -> *const c_char;
}

pub fn char_ptr_to_string(c_ptr: *const c_char) -> String {
    let mut size: usize = 0;
    unsafe {
        while *(c_ptr.add(size)) != b'\0' {
            size += 1;
        }
        let slice = std::slice::from_raw_parts(c_ptr, size);
        String::from_utf8_lossy(slice).into_owned()
    }
}

#[macro_export]
macro_rules! WIFEXITED {
    ($wait_status:expr) => {
        ((($wait_status) & 0x7f) == 0)
    };
}

#[macro_export]
macro_rules! WEXITSTATUS {
    ($wait_status:expr) => {
        ((($wait_status) & 0xff00) >> 8) as u8
    };
}

#[macro_export]
macro_rules! WIFSIGNALED {
    ($wait_status:expr) => {
        ((((($wait_status) & 0x7f) + 1) >> 1) > 0)
    };
}

#[macro_export]
macro_rules! WTERMSIG {
    ($wait_status:expr) => {
        (($wait_status) & 0x7f) as u8
    };
}

#[macro_export]
macro_rules! WIFSTOPPED {
    ($wait_status:expr) => {
        ((($wait_status) & 0xff) == 0x7f)
    };
}

#[macro_export]
macro_rules! WSTOPSIG {
    ($wait_status:expr) => {
        ((($wait_status) & 0xff00) >> 8) as u8
    };
}
