use std::{
    collections::{BTreeMap, HashMap},
    ops::Range,
};

use iced_x86::code_asm::CodeLabel;
use intermediate::{Instruction, LabelId, SymbolId};

use crate::registers::Registers;

pub(crate) struct FunctionContext<'a> {
    pub(crate) name: String,
    pub(crate) slots: BTreeMap<SymbolId, i32>,
    pub(crate) labels: BTreeMap<LabelId, usize>,
    pub(crate) pending: Vec<LabelId>,
    pub(crate) epilogue: CodeLabel,
    pub(crate) cursor: usize,
    pub(crate) instructions: &'a [Instruction],
    pub(crate) registers: Registers,
    pub(crate) liveness: HashMap<SymbolId, Range<usize>>,
}

impl<'a> FunctionContext<'a> {
    /// Whether the symbol is live at the current cursor position.
    pub(crate) fn is_live(&self, sym: SymbolId) -> bool {
        self.liveness.get(&sym).map_or(false, |range| {
            range.start <= self.cursor && self.cursor < range.end
        })
    }

    /// Whether the symbol will be live after the current instruction.
    pub(crate) fn will_be_live(&self, sym: SymbolId) -> bool {
        self.liveness
            .get(&sym)
            .map_or(false, |range| self.cursor + 1 < range.end)
    }
}
