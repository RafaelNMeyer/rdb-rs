use crate::user::*;
use core::mem::offset_of;

enum RegisterType {
    GPR,
    SUBGPR,
    FPR,
    DR,
}

enum RegisterFormat {
    UINT,
    DOUBLEFLOAT,
    LONGDOUBLE,
    VECTOR,
}

struct RegisterInfo<'a> {
    id: RegisterId,
    name: &'a str,
    dwarf_id: i32,
    size: usize,
    offset: usize, // offset to find it in <sys/user.h> c structs
    r_type: RegisterType,
    format: RegisterFormat,
}

macro_rules! register {
    ( $( ($name:ident, $dwarf_id:expr, $size:expr, $offset:expr, $r_type:expr, $format:expr) ),+ ) => {

        #[allow(non_camel_case_types)]
        pub enum RegisterId {
            $(
                $name,
            )*
        }

pub static G_REGISTER_INFOS: &[RegisterInfo] = &[
        $(
        RegisterInfo {
            id: RegisterId::$name,
            name: stringify!($name),
            dwarf_id: $dwarf_id,
            size: $size,
            offset: $offset,
            r_type: $r_type,
            format: $format,
        }
        ),*
];
    };
}

macro_rules! gpr_offset {
    ($reg:ident) => {
        offset_of!(user, regs) + offset_of!(user_regs_struct, $reg)
    };
}

macro_rules! define_registers {
    ( @collect [$($acc:tt)*] ) => {
        register!($($acc)*);
    };

    (@collect [$($acc:tt)*] r_gpr_64 ( $( ($name:ident, $dwarf_id:expr) ),*) $($any:tt)* ) => {
        define_registers!(
            @collect
            [
                $($acc,)*
                $((
                    $name,
                    $dwarf_id,
                    8,
                    gpr_offset!($name),
                    RegisterType::GPR,
                    RegisterFormat::UINT
                )),*]
            $($any)*);
    };

    // It needs to be the last statement,
    // otherwise, it falls into an infinite loop
    ( $($any:tt)* ) => {
        define_registers!(@collect [] $($any)*);
    };
}

define_registers!(
    r_gpr_64(
        (rax, 0), (rdx, 1)
    )
    // r_gpr_32(
    //     (eax, rax), (edx, rdx)
    // )
);
