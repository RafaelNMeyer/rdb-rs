use std::ptr::copy;

pub type Byte64 = [u8; 8];
pub type Byte128 = [u8; 16];

pub fn to_byte128<From>(src: &From) -> Byte128 {
    // TODO: check if this initialization is legal
    let mut ret: Byte128 = [0; 16];
    unsafe {
        copy(
            (src as *const From).cast::<u8>(),
            (&mut ret as *mut Byte128).cast::<u8>(),
            size_of::<From>(),
        );
    }
    ret
}

pub fn to_byte64<From>(src: &From) -> Byte64 {
    // TODO: check if this initialization is legal
    let mut ret: Byte64 = [0; 8];
    unsafe {
        copy(
            (src as *const From).cast::<u8>(),
            (&mut ret as *mut Byte64).cast::<u8>(),
            size_of::<From>(),
        );
    }
    ret
}
