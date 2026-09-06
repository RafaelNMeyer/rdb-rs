use core::mem::zeroed;
use std::ptr::copy;

pub fn from_bytes<To>(bytes: *const u8) -> To {
    unsafe {
        let mut ret: To = zeroed();
        copy(bytes, (&mut ret as *mut To).cast::<u8>(), size_of::<To>());
        ret
    }
}
