use std::{
    collections::{BTreeMap, HashMap},
    ops::Range,
};

use iced_x86::code_asm::CodeLabel;
use intermediate::{Instruction, LabelId, SymbolId};

use crate::registers::Registers;

pub(crate) struct FunctionContext<'a> {
    pub(crate) slots: BTreeMap<SymbolId, i32>,
    pub(crate) labels: BTreeMap<LabelId, usize>,
    pub(crate) pending: Vec<LabelId>,
    pub(crate) epilogue: CodeLabel,
    pub(crate) cursor: usize,
    pub(crate) instructions: &'a [Instruction],
    pub(crate) registers: Registers,
    pub(crate) liveness: HashMap<SymbolId, Range<usize>>,
}
