macro_rules! r8 {
    ($r:expr) => {
        ::iced_x86::code_asm::get_gpr8(match $r.full_register() {
            ::iced_x86::Register::RAX => ::iced_x86::Register::AL,
            ::iced_x86::Register::RCX => ::iced_x86::Register::CL,
            ::iced_x86::Register::RDX => ::iced_x86::Register::DL,
            ::iced_x86::Register::RBX => ::iced_x86::Register::BL,
            ::iced_x86::Register::RSP => ::iced_x86::Register::SPL,
            ::iced_x86::Register::RBP => ::iced_x86::Register::BPL,
            ::iced_x86::Register::RSI => ::iced_x86::Register::SIL,
            ::iced_x86::Register::RDI => ::iced_x86::Register::DIL,
            ::iced_x86::Register::R8 => ::iced_x86::Register::R8L,
            ::iced_x86::Register::R9 => ::iced_x86::Register::R9L,
            ::iced_x86::Register::R10 => ::iced_x86::Register::R10L,
            ::iced_x86::Register::R11 => ::iced_x86::Register::R11L,
            ::iced_x86::Register::R12 => ::iced_x86::Register::R12L,
            ::iced_x86::Register::R13 => ::iced_x86::Register::R13L,
            ::iced_x86::Register::R14 => ::iced_x86::Register::R14L,
            ::iced_x86::Register::R15 => ::iced_x86::Register::R15L,
            _ => unreachable!(),
        })
        .unwrap()
    };
}

macro_rules! r16 {
    ($r:expr) => {
        ::iced_x86::code_asm::get_gpr16(match $r.full_register() {
            ::iced_x86::Register::RAX => ::iced_x86::Register::AX,
            ::iced_x86::Register::RCX => ::iced_x86::Register::CX,
            ::iced_x86::Register::RDX => ::iced_x86::Register::DX,
            ::iced_x86::Register::RBX => ::iced_x86::Register::BX,
            ::iced_x86::Register::RSP => ::iced_x86::Register::SP,
            ::iced_x86::Register::RBP => ::iced_x86::Register::BP,
            ::iced_x86::Register::RSI => ::iced_x86::Register::SI,
            ::iced_x86::Register::RDI => ::iced_x86::Register::DI,
            ::iced_x86::Register::R8 => ::iced_x86::Register::R8W,
            ::iced_x86::Register::R9 => ::iced_x86::Register::R9W,
            ::iced_x86::Register::R10 => ::iced_x86::Register::R10W,
            ::iced_x86::Register::R11 => ::iced_x86::Register::R11W,
            ::iced_x86::Register::R12 => ::iced_x86::Register::R12W,
            ::iced_x86::Register::R13 => ::iced_x86::Register::R13W,
            ::iced_x86::Register::R14 => ::iced_x86::Register::R14W,
            ::iced_x86::Register::R15 => ::iced_x86::Register::R15W,
            _ => unreachable!(),
        })
        .unwrap()
    };
}

macro_rules! r32 {
    ($r:expr) => {
        ::iced_x86::code_asm::get_gpr32($r.full_register32()).unwrap()
    };
}

macro_rules! r64 {
    ($r:expr) => {
        ::iced_x86::code_asm::get_gpr64($r.full_register()).unwrap()
    };
}

pub(crate) use r8;
pub(crate) use r16;
pub(crate) use r32;
pub(crate) use r64;
