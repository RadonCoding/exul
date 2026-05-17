use emitter::assembly::Assembly;
use iced_x86::{Instruction, SymbolResolver, SymbolResult};
use intermediate::Module;
use std::collections::HashMap;

pub struct Symbols(pub HashMap<u64, String>);

impl SymbolResolver for Symbols {
    fn symbol(
        &mut self,
        _instruction: &Instruction,
        _operand: u32,
        _instruction_operand: Option<u32>,
        address: u64,
        _address_size: u32,
    ) -> Option<SymbolResult<'_>> {
        self.0
            .get(&address)
            .map(|name| SymbolResult::with_str(address, name.as_str()))
    }
}

pub fn build_symbols(assembly: &Assembly, module: &Module, ip: u64) -> HashMap<u64, String> {
    let mut map = HashMap::new();

    for (id, offset) in &assembly.functions {
        let name = if let Some(import) = module.imports.iter().find(|i| i.id == *id) {
            format!("{}!{}", import.module, import.function)
        } else {
            module
                .functions
                .iter()
                .find(|f| f.id == *id)
                .map(|f| f.name.clone())
                .unwrap_or_else(|| id.0.to_string())
        };
        map.insert(ip + *offset as u64, name);
    }

    let import_base = assembly.blobs.last().map(|b| b.offset + b.len).unwrap_or(0);
    
    for (i, import) in module.imports.iter().enumerate() {
        map.insert(
            ip + (import_base + i * 8) as u64,
            format!("{}!{}", import.module, import.function),
        );
    }

    map
}
