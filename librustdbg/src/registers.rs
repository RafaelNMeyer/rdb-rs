use crate::register_info::RegisterInfo;
use crate::user::*;
use core::mem::offset_of;

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

macro_rules! register_gpr_64 {
    ( $( ($name:ident, $dwarf_id:expr) ),+ ) => {
        register!(
            $((
                $name,
                $dwarf_id,
                8,
                gpr_offset!($name),
                RegisterType::GPR,
                RegisterFormat::UINT
            )),*
        );
    }
}

macro_rules! register_gpr_32 {
    ( $( ($name:ident, $super:ident) ),+ ) => {
        register!(
            $((
                $name,
                -1,
                4,
                gpr_offset!($super),
                RegisterType::SUB_GPR,
                RegisterFormat::UINT
            )),*
        );
    }
}

register_gpr_64!((rax, 0), (rdx, 1));
register_gpr_32!((eax, rax), (edx, rdx));
