use std::error::Error;

use intermediate::Module;

use crate::{assembly::Assembly, convention::Convention, emitter::Emitter};

mod allocator;
pub mod assembly;
mod context;
pub mod convention;
mod emitter;
mod emitters;
mod macros;
pub mod peephole;
mod registers;

pub fn emit<C: Convention>(ip: u64, module: &mut Module) -> Result<Assembly, Box<dyn Error>> {
    let mut emitter = Emitter::new(C::default())?;

    emitter.emit(ip, module)
}
