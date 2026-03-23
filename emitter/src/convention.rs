use iced_x86::Register;

pub trait Convention: Default {
    fn return_register(&self) -> Register;
    fn volatile_registers(&self) -> Vec<Register>;
    fn non_volatile_registers(&self) -> Vec<Register>;
    fn argument_registers(&self) -> Vec<Register>;
    fn shadow_space(&self) -> usize;

    fn argument_reg(&self, index: usize) -> Option<Register> {
        self.argument_registers().get(index).copied()
    }
}

#[derive(Default)]
pub struct MicrosoftX64;

impl Convention for MicrosoftX64 {
    fn return_register(&self) -> Register {
        Register::RAX
    }
    fn volatile_registers(&self) -> Vec<Register> {
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
    fn non_volatile_registers(&self) -> Vec<Register> {
        vec![
            Register::RBX,
            Register::RDI,
            Register::RSI,
            Register::R12,
            Register::R13,
            Register::R14,
            Register::R15,
        ]
    }
    fn argument_registers(&self) -> Vec<Register> {
        vec![Register::RCX, Register::RDX, Register::R8, Register::R9]
    }
    fn shadow_space(&self) -> usize {
        32
    }
}
