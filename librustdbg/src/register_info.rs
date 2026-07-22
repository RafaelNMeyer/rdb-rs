use crate::user::*;
use core::mem::{offset_of, size_of_val, zeroed};

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

pub struct RegisterInfo<'a> {
    id: RegisterId,
    name: &'a str,
    dwarf_id: i32,
    size: usize,
    offset: usize, // offset to find it in <sys/user.h> c structs
    r_type: RegisterType,
    format: RegisterFormat,
}

macro_rules! register
{
    ($(($name:ident, $dwarf_id:expr, $size:expr, $offset:expr, $r_type:expr, $format:expr)),+) => {
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

macro_rules! fpr_offset {
    ($reg:ident) => {
        offset_of!(user, i387) + offset_of!(user_fpregs_struct, $reg)
    };
}

macro_rules! dr_offset {
    ($number:expr) => {
        offset_of!(user, u_debugreg) + $number*8
    }
}

macro_rules! fpr_size {
    ($reg:ident) => {
        size_of_val(&unsafe { zeroed::<user_fpregs_struct>() }.$reg)
    };
}

macro_rules! define_registers {
    ( $($any:tt)* ) => {
        define_registers_helper!(@collect [] $($any)*);
    };
}

macro_rules! define_registers_helper {
    (@collect [$($acc:tt)*] r_gpr_64 ( $( ($name:ident, $dwarf_id:expr) ),*) $($any:tt)* ) => {
        define_registers_helper!(
            @collect
            [
                $((
                    $name,
                    $dwarf_id,
                    8,
                    gpr_offset!($name),
                    RegisterType::GPR,
                    RegisterFormat::UINT
                )),*]
            $($any)*
            );
    };

    (@collect [$($acc:tt),*] r_gpr_32 ( $( ($name:ident, $super:ident) ),*) $($any:tt)* ) => {
        define_registers_helper!(
            @collect
            [
                $($acc,)*
                $((
                    $name,
                    -1,
                    4,
                    gpr_offset!($super),
                    RegisterType::SUBGPR,
                    RegisterFormat::UINT
                )),*]
            $($any)*
            );
    };

    (@collect [$($acc:tt),*] r_gpr_16 ( $( ($name:ident, $super:ident) ),*) $($any:tt)* ) => {
        define_registers_helper!(
            @collect
            [
                $($acc,)*
                $((
                    $name,
                    -1,
                    2,
                    gpr_offset!($super),
                    RegisterType::SUBGPR,
                    RegisterFormat::UINT
                )),*]
            $($any)*
            );
    };

    (@collect [$($acc:tt),*] r_gpr_8h ( $( ($name:ident, $super:ident) ),*) $($any:tt)* ) => {
        define_registers_helper!(
            @collect
            [
                $($acc,)*
                $((
                    $name,
                    -1,
                    1,
                    gpr_offset!($super) + 1,
                    RegisterType::SUBGPR,
                    RegisterFormat::UINT
                )),*]
            $($any)*
            );
    };

    (@collect [$($acc:tt),*] r_gpr_8l ($( ($name:ident, $super:ident) ),*) $($any:tt)* ) => {
        define_registers_helper!(
            @collect
            [
                $($acc,)*
                $((
                    $name,
                    -1,
                    1,
                    gpr_offset!($super),
                    RegisterType::SUBGPR,
                    RegisterFormat::UINT
                )),*]
            $($any)*
            );
    };

    (@collect [$($acc:tt),*] r_fpr ($( ($name:ident, $dwarf_id:expr, $user_name:ident) ),*) $($any:tt)*) => {
        define_registers_helper!(
            @collect
            [
                $($acc,)*
                $((
                    $name,
                    $dwarf_id,
                    fpr_size!($user_name),
                    fpr_offset!($user_name),
                    RegisterType::FPR,
                    RegisterFormat::UINT
                )),*]
            $($any)*
            );
    };

    (@collect [$($acc:tt),*] r_fp_st ($( ($number:expr, $i:ident) ),*) $($any:tt)*) => {
        define_registers_helper!(
            @collect
            [
                $($acc,)*
                $((
                    $i,
                    33 + $number,
                    16,
                    fpr_offset!(st_space) + $number*16,
                    RegisterType::FPR,
                    RegisterFormat::VECTOR
                )),*]
            $($any)*
            );
    };

    (@collect [$($acc:tt),*] r_fp_mm ($( ($number:expr, $i:ident) ),*) $($any:tt)*) => {
        define_registers_helper!(
            @collect
            [
                $($acc,)*
                $((
                    $i,
                    41 + $number,
                    8,
                    fpr_offset!(st_space) + $number*16,
                    RegisterType::FPR,
                    RegisterFormat::VECTOR
                )),*]
            $($any)*
            );
    };

    (@collect [$($acc:tt),*] r_fp_xmm ($( ($number:expr, $i:ident) ),*) $($any:tt)*) => {
        define_registers_helper!(
            @collect
            [
                $($acc,)*
                $((
                    $i,
                    17 + $number,
                    16,
                    fpr_offset!(xmm_space) + $number*16,
                    RegisterType::FPR,
                    RegisterFormat::VECTOR
                )),*]
            $($any)*
            );
    };

    (@collect [$($acc:tt),*] r_fp_xmm ($( ($number:expr, $i:ident) ),*) $($any:tt)*) => {
        define_registers_helper!(
            @collect
            [
                $($acc,)*
                $((
                    $i,
                    17 + $number,
                    16,
                    fpr_offset!(xmm_space) + $number*16,
                    RegisterType::FPR,
                    RegisterFormat::VECTOR
                )),*]
            $($any)*
            );
    };

    (@collect [$($acc:tt),*] r_dr ($( ($number:expr, $i:ident) ),*) $($any:tt)*) => {
        define_registers_helper!(
            @collect
            [
                $($acc,)*
                $((
                    $i,
                    -1,
                    8,
                    dr_offset!($number),
                    RegisterType::DR,
                    RegisterFormat::UINT
                )),*]
            $($any)*
            );
    };
     
    ( @collect [$($acc:tt)*] ) => {
        register!($($acc)*);
    }
}

// define_registers!(r_gpr_64((rax, 0), (rdx, 1)));
define_registers!(
    r_gpr_64(
        (rax, 0),     (rdx, 1),
        (rcx, 2),     (rbx, 3),
        (rsi, 4),     (rsp, 7),
        (r8, 8),      (r9, 9),
        (r10, 10),    (r11, 11),
        (r12, 12),    (r13, 13),
        (r14, 14),    (rip, 16),
        (eflags, 49), (cs, 51),
        (fs, 54),     (gs, 55),
        (ss, 52),     (ds, 53),
        (es, 50),     (orig_rax, -1)
    )
    r_gpr_32(
        (eax, rax),  (edx, rdx),
        (ecx, rcx),  (ebx, rbx),
        (esi, rsi),  (edi, rdi),
        (ebp, rbp),  (esp, rsp),
        (r8d, r8),   (r9d, r9),
        (r10d, r10), (r11d, r11),
        (r12d, r12), (r13d, r13),
        (r14d, r14), (r15d, r15)
    )
    r_gpr_16(
        (ax, rax),   (dx, rdx),
        (cx, rcx),   (bx, rbx),
        (si, rsi),   (di, rdi),
        (bp, rbp),   (sp, rsp),
        (r8w, r8),   (r9w, r9),
        (r10w, r10), (r11w, r11),
        (r12w, r12), (r13w, r13),
        (r14w, r14), (r15w, r15)
    )
    r_gpr_8h(
        (ah, rax), (dh, rdx),
        (ch, rcx), (bh, rbx)
    )
    r_gpr_8l(
        (al, rax),   (dl, rdx),
        (cl, rcx),   (bl, rbx),
        (sil, rsi),  (dil, rdi),
        (bpl, rbp),  (spl, rsp),
        (r8b, r8),   (r9b, r9),
        (r10b, r10), (r11b, r11),
        (r12b, r12), (r13b, r13),
        (r14b, r14), (r15b, r15)
    )
    r_fpr(
        (fcw, 65, cwd),
        (fsw, 66, swd),
        (ftw, -1, ftw),
        (fop, -1, fop),
        (frip, -1, rip),
        (frdp, -1, rdp),
        (mxcsr, 64, mxcsr),
        (mxcsrmask, -1, mxcr_mask)
    )
    r_fp_st(
        (0,st0), (1,st1),
        (2,st2), (3,st3),
        (4,st4), (5,st5),
        (6,st6), (7,st7)
    )
    r_fp_mm(
        (0,mm0),(1,mm1),
        (2,mm2),(3,mm3),
        (4,mm4),(5,mm5),
        (6,mm6),(7,mm7)
    )
    r_fp_xmm(
        (0,xmm0),(1,xmm1),
        (2,xmm2),(3,xmm3),
        (4,xmm4),(5,xmm5),
        (6,xmm6),(7,xmm7),
        (8,xmm8),(9,xmm9),
        (10,xmm10),(11,xmm11),
        (12,xmm12),(13,xmm13),
        (14,xmm14),(15,xmm15)
    )
    r_dr(
        (0,dr0), (1,dr1),
        (2,dr2), (3,dr3),
        (4,dr4), (5,dr5),
        (6,dr6), (7,dr7)
    )
);
