use std::fmt::Debug;

use crate::bindings::{__errno_location, char_ptr_to_string, strerror};

pub struct Error {
    err: String,
}

impl Error {
    fn new(err: String) -> Error {
        Error { err }
    }

    pub fn send(what: &str) -> Error {
        return Self::new(String::from(what));
    }

    pub fn send_errno(prefix: &str) -> Error {
        return Self::new(format!("{}: {}", prefix, errno_string()));
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.err))
    }
}

pub fn errno_string() -> String {
    unsafe {
        let reason = strerror(*__errno_location());
        char_ptr_to_string(reason)
    }
}
