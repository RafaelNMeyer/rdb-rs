const READ_FD: usize = 0;
const WRITE_FD: usize = 1;

const BUF_SIZE: usize = 1024;

use core::str;

use crate::bindings::{O_CLOEXEC, c_char, close, pipe2, read, write};
use crate::error::errno_string;

pub struct Pipe {
    fds: [i32; 2],
}

impl Pipe {
    pub fn new(close_on_exec: bool) -> Pipe {
        let pipe = Pipe { fds: [0, 0] };
        unsafe {
            let flags = match close_on_exec {
                true => O_CLOEXEC,
                false => 0,
            };
            if pipe2(pipe.fds.as_ptr(), flags) < 0 {
                println!("error creating pipe: {}", errno_string())
            };
        }
        return pipe;
    }

    pub fn read(&self) -> Vec<u8> {
        let mut buf: [c_char; BUF_SIZE] = [0 as c_char; 1024];
        unsafe {
            // TODO: send errno
            let bytes_read = read(self.fds[READ_FD], buf.as_mut_ptr(), BUF_SIZE);
            if bytes_read < 0 {
                println!("error reading: {}", errno_string())
            }
            return Vec::from(&buf[..bytes_read as usize]);
        }
    }

    pub fn write(&self, msg: &str) {
        unsafe {
            // TODO: send errno
            if write(self.fds[WRITE_FD], msg.as_ptr(), msg.len()) < 0 {
                println!("error writing: {}", errno_string())
            };
        }
    }

    pub fn close_read(&mut self) {
        if self.fds[READ_FD] != -1 {
            unsafe {
                if close(self.fds[READ_FD]) < 0 {
                    println!("error closing read: {}", errno_string())
                };
            }
            self.fds[READ_FD] = -1;
        }
    }

    pub fn close_write(&mut self) {
        if self.fds[WRITE_FD] != -1 {
            unsafe {
                if close(self.fds[WRITE_FD]) < 0 {
                    println!("error closing write: {}", errno_string())
                };
            }
            self.fds[WRITE_FD] = -1;
        }
    }
}

impl Drop for Pipe {
    fn drop(&mut self) {
        self.close_read();
        self.close_write();
    }
}

// int rdb::pipe::release_read() { return std::exchange(fds_[read_fd], -1); }
//
// int rdb::pipe::release_write() { return std::exchange(fds_[write_fd], -1); }
