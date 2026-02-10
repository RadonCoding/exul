use std::error::Error;

use intermediate::Module;

use crate::{convention::Convention, emitter::Emitter};

mod allocator;
mod arithmetic;
mod branches;
pub mod convention;
mod emitter;
mod peephole;
mod registers;

pub fn emit<C: Convention>(ip: u64, module: &mut Module) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut emitter = Emitter::new(C::default())?;

    for func in &mut module.functions {
        peephole::optimize(func);
    }

    emitter.emit(ip, module)
}
