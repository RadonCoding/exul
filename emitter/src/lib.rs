use std::error::Error;

use ir::Module;

use crate::{convention::Convention, emitter::Emitter};

mod allocator;
mod arithmetic;
mod branches;
pub mod convention;
mod emitter;
mod peephole;

pub fn emit<C: Convention>(ip: u64, module: Module) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut emitter = Emitter::new(C::default())?;

    let optimized = Module {
        functions: module
            .functions
            .into_iter()
            .map(peephole::optimize)
            .collect(),
        entry: module.entry,
    };

    emitter.emit(ip, optimized)
}
