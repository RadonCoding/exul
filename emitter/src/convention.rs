use iced_x86::Register;

pub trait Convention: Default {
    fn return_reg(&self) -> Register;
    fn volatile_regs(&self) -> Vec<Register>;
    fn argument_regs(&self) -> Vec<Register>;
    fn shadow_space(&self) -> usize;
}

#[derive(Default)]
pub struct MicrosoftX64;

impl Convention for MicrosoftX64 {
    fn return_reg(&self) -> Register {
        Register::RAX
    }
    fn volatile_regs(&self) -> Vec<Register> {
        vec![
            Register::RAX,
            Register::RCX,
            Register::RDX,
            Register::R8,
            Register::R9,
            Register::R10,
            Register::R11,
        ]
    }
    fn argument_regs(&self) -> Vec<Register> {
        vec![Register::RCX, Register::RDX, Register::R8, Register::R9]
    }
    fn shadow_space(&self) -> usize {
        32
    }
}
