use std::cell::RefCell;
use std::mem;
use std::ptr::copy;
use std::rc::Weak;

use crate::bit::from_bytes;
use crate::register_info::{
    RegisterFormat, RegisterId, RegisterInfo, RegisterType, register_info_by_id,
};
use crate::{Error, types::*};
use crate::{Process, user::user};

pub struct Registers {
    pub data: user,
    proc: Weak<RefCell<Process>>,
}

#[derive(Debug)]
pub enum Variant {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    Byte64(Byte64),
    Byte128(Byte128),
}

impl Registers {
    pub fn new(proc: Weak<RefCell<Process>>) -> Registers {
        let regs: Registers;
        unsafe {
            regs = Registers {
                data: mem::zeroed(),
                proc: proc,
            };
        }
        regs
    }
    pub fn read_by_id(self, id: RegisterId) -> Result<Variant, Error> {
        Ok(self.read(register_info_by_id(id)?)?)
    }

    fn read(self, info: &RegisterInfo) -> Result<Variant, Error> {
        let bytes: *const u8 = (&self.data as *const user).cast::<u8>();

        unsafe {
            if matches!(info.format, RegisterFormat::UINT) {
                match info.size {
                    1 => return Ok(Variant::U8(from_bytes::<u8>(bytes.add(info.offset)))),
                    2 => {
                        return Ok(Variant::U16(from_bytes::<u16>(bytes.add(info.offset))));
                    }
                    4 => {
                        return Ok(Variant::U32(from_bytes::<u32>(bytes.add(info.offset))));
                    }
                    8 => {
                        return Ok(Variant::U64(from_bytes::<u64>(bytes.add(info.offset))));
                    }
                    _ => return Err(Error::send("Unexpected register size")),
                }
            } else if matches!(info.format, RegisterFormat::DOUBLEFLOAT) {
                return Ok(Variant::F64(from_bytes::<f64>(bytes.add(info.offset))));
            } else if matches!(info.format, RegisterFormat::LONGDOUBLE) {
                return Ok(Variant::Byte128(from_bytes::<Byte128>(
                    bytes.add(info.offset),
                )));
            } else if matches!(info.format, RegisterFormat::VECTOR) && info.size == 8 {
                return Ok(Variant::Byte64(from_bytes::<Byte64>(
                    bytes.add(info.offset),
                )));
            } else {
                return Ok(Variant::Byte128(from_bytes::<Byte128>(
                    bytes.add(info.offset),
                )));
            }
        }
    }

    pub fn write_by_id(&mut self, id: RegisterId, value: Variant) -> Result<(), Error> {
        self.write(register_info_by_id(id)?, value)
    }

    fn write(&mut self, info: &RegisterInfo, value: Variant) -> Result<(), Error> {
        let bytes: *mut u8 = (&mut self.data as *mut user).cast::<u8>();

        let size: usize;
        let val_widen: Byte128;
        match value {
            Variant::U8(x) => {
                size = size_of::<u8>();
                val_widen = to_byte128::<u8>(&x);
            }
            Variant::U16(x) => {
                size = size_of::<u16>();
                val_widen = to_byte128::<u16>(&x);
            }
            Variant::U32(x) => {
                size = size_of::<u32>();
                val_widen = to_byte128::<u32>(&x);
            }
            Variant::U64(x) => {
                size = size_of::<u64>();
                val_widen = to_byte128::<u64>(&x);
            }
            Variant::I8(x) => {
                size = size_of::<i8>();
                val_widen = to_byte128::<i8>(&x);
            }
            Variant::I16(x) => {
                size = size_of::<i16>();
                val_widen = to_byte128::<i16>(&x);
            }
            Variant::I32(x) => {
                size = size_of::<i32>();
                val_widen = to_byte128::<i32>(&x);
            }
            Variant::I64(x) => {
                size = size_of::<i64>();
                val_widen = to_byte128::<i64>(&x);
            }
            Variant::F32(x) => {
                size = size_of::<f32>();
                val_widen = to_byte128::<f32>(&x);
            }
            Variant::F64(x) => {
                size = size_of::<f64>();
                val_widen = to_byte128::<f64>(&x);
            }
            Variant::Byte64(x) => {
                size = size_of::<Byte64>();
                val_widen = to_byte128::<Byte64>(&x);
            }
            Variant::Byte128(x) => {
                size = size_of::<Byte128>();
                val_widen = to_byte128::<Byte128>(&x);
            }
        }

        if size > info.size {
            return Err(Error::send(
                "registers::write called with mismatched register and value sizes",
            ));
        }
        unsafe {
            copy(val_widen.as_ptr(), bytes.add(info.offset), info.size);
        }

        // // needed to align 8 bytes for registers ah,bh,ch,dh
        let aligned_offset = info.offset & !0b111; // same as ~0b111?
        //
        // // this write to user area!
        // // and can throw an error since we cannot write to i837 registers
        // // they are smaller than 64 bits and we can write more than once
        // // so we do a condition here
        unsafe {
            if matches!(info.r_type, RegisterType::FPR) {
                if let Some(proc) = self.proc.upgrade() {
                    proc.borrow_mut().write_fprs(&self.data.i387)?;
                }
            } else {
                if let Some(proc) = self.proc.upgrade() {
                    proc.borrow_mut().write_user_area(
                        aligned_offset,
                        from_bytes::<u64>(bytes.add(aligned_offset)),
                    )?;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::mem::offset_of;

    use crate::{
        Pipe, Process,
        bindings::double_to_bytes,
        bit::from_bytes,
        register_info::RegisterId,
        types::{Byte128, x87_from_f64},
        user::{self, user_fpregs_struct},
    };

    #[test]
    fn write_register_works() {
        let close_on_exec = false;
        let mut channel = Pipe::new(close_on_exec);

        let proc = Process::launch(
            "target/reg_write".to_string(),
            true,
            Some(channel.get_write()),
        )
        .unwrap();

        channel.close_write();

        proc.borrow_mut().resume().unwrap();
        proc.borrow_mut().wait_on_signal().unwrap();

        let mut regs = proc.borrow_mut().registers.take().unwrap();

        regs.write_by_id(RegisterId::rsi, super::Variant::U64(0xcafecafe))
            .unwrap();
        proc.borrow_mut().registers = Some(regs);
        proc.borrow_mut().resume().unwrap();
        proc.borrow_mut().wait_on_signal().unwrap();
        assert!(String::from_utf8_lossy(&channel.read()[..]) == "0xcafecafe");

        regs = proc.borrow_mut().registers.take().unwrap();
        regs.write_by_id(RegisterId::mm0, super::Variant::U64(0x12345678))
            .unwrap();
        proc.borrow_mut().registers = Some(regs);
        proc.borrow_mut().resume().unwrap();
        proc.borrow_mut().wait_on_signal().unwrap();
        assert!(String::from_utf8_lossy(&channel.read()[..]) == "0x12345678");

        regs = proc.borrow_mut().registers.take().unwrap();
        regs.write_by_id(RegisterId::xmm0, super::Variant::F64(42.42))
            .unwrap();
        proc.borrow_mut().registers = Some(regs);
        proc.borrow_mut().resume().unwrap();
        proc.borrow_mut().wait_on_signal().unwrap();
        assert!(String::from_utf8_lossy(&channel.read()[..]) == "42.42");

        regs = proc.borrow_mut().registers.take().unwrap();
        unsafe {
            let bytes = super::Variant::Byte128(double_to_bytes(42.24));
            regs.write_by_id(RegisterId::st0, bytes)
        }
        .unwrap();
        // assume that float stats word is full setting 111 to 11-13 bits
        regs.write_by_id(RegisterId::fsw, super::Variant::U16(0b0011100000000000))
            .unwrap();
        // tag registers, (0b11=empty, 0b00 valid)
        // since it's st0, we set 0b00 to firsts bits
        regs.write_by_id(RegisterId::ftw, super::Variant::U16(0b0011111111111111))
            .unwrap();
        proc.borrow_mut().registers = Some(regs);
        proc.borrow_mut().resume().unwrap();
        proc.borrow_mut().wait_on_signal().unwrap();
        let out_bytes = &channel.read()[..];
        let output = String::from_utf8_lossy(out_bytes);
        assert!(output == "42.24");
    }
}
