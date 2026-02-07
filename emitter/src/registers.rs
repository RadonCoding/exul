use crate::convention::Convention;
use iced_x86::{
    Register,
    code_asm::{AsmRegister64, CodeAssembler, get_gpr32, get_gpr64, qword_ptr, rbp},
};
use intermediate::{SymbolId, Value};
use std::{collections::HashMap, error::Error};

pub struct Registers {
    tracked: HashMap<Register, Value>,
}

impl Registers {
    pub fn new() -> Self {
        Self {
            tracked: HashMap::new(),
        }
    }

    pub fn find_value(&self, val: Value) -> Option<Register> {
        self.tracked
            .iter()
            .find(|(_, v)| **v == val)
            .map(|(r, _)| *r)
    }

    pub fn track(&mut self, reg: Register, val: Value) {
        self.tracked.insert(reg, val);
    }

    pub fn invalidate_symbol(&mut self, symbol: SymbolId) {
        self.tracked.retain(|_, v| {
            if let Value::Symbol(s) = v {
                *s != symbol
            } else {
                true
            }
        });
    }

    pub fn invalidate_volatile<C: Convention>(&mut self, convention: &C) {
        let volatiles = convention.volatile_regs();
        self.tracked.retain(|reg, _| !volatiles.contains(reg));
    }

    pub fn clear(&mut self) {
        self.tracked.clear();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueLocation {
    Register(Register),
    Stack(i32),
    Immediate(i64),
}

pub struct ValueContext<'a> {
    pub allocs: &'a HashMap<SymbolId, Register>,
    pub slots: &'a HashMap<SymbolId, i32>,
    pub registers: &'a Registers,
}

impl<'a> ValueContext<'a> {
    pub fn locate(&self, val: Value) -> ValueLocation {
        match val {
            Value::Constant(c) => ValueLocation::Immediate(c),
            Value::Symbol(s) => {
                if let Some(reg) = self.registers.find_value(val) {
                    return ValueLocation::Register(reg);
                }

                if let Some(&reg) = self.allocs.get(&s) {
                    return ValueLocation::Register(reg);
                }

                if let Some(&offset) = self.slots.get(&s) {
                    return ValueLocation::Stack(offset);
                }

                unreachable!()
            }
        }
    }
}

#[derive(Debug)]
pub enum LoadAction {
    None,
    Move(AsmRegister64),
    LoadStack(i32),
    LoadImmediate(i64),
}

pub fn plan_load(val: Value, target: Register, ctx: &ValueContext) -> LoadAction {
    let location = ctx.locate(val);

    match location {
        ValueLocation::Register(r) if r == target => LoadAction::None,
        ValueLocation::Register(r) => LoadAction::Move(get_gpr64(r).unwrap()),
        ValueLocation::Stack(offset) => LoadAction::LoadStack(offset),
        ValueLocation::Immediate(imm) => LoadAction::LoadImmediate(imm),
    }
}

pub fn execute_load(
    asm: &mut CodeAssembler,
    action: LoadAction,
    target: AsmRegister64,
) -> Result<(), Box<dyn Error>> {
    match action {
        LoadAction::None => Ok(()),
        LoadAction::Move(src) => {
            asm.mov(target, src)?;
            Ok(())
        }
        LoadAction::LoadStack(offset) => {
            asm.mov(target, qword_ptr(rbp - offset))?;
            Ok(())
        }
        LoadAction::LoadImmediate(imm) => {
            if imm == 0 {
                let reg = Into::<Register>::into(target);
                let r32 = get_gpr32(reg.full_register32()).unwrap();
                asm.xor(r32, r32)?;
            } else if imm >= 0 && imm <= u32::MAX as i64 {
                let reg = Into::<Register>::into(target);
                let r32 = get_gpr32(reg.full_register32()).unwrap();
                asm.mov(r32, imm as u32)?;
            } else {
                asm.mov(target, imm as u64)?;
            }
            Ok(())
        }
    }
}
