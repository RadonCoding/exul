use std::collections::HashMap;

use intermediate::FunctionId;

pub struct Assembly {
    pub bytes: Vec<u8>,
    pub blobs: Vec<Blob>,
    pub functions: HashMap<FunctionId, usize>,
}

pub struct Blob {
    pub offset: usize,
    pub len: usize,
    pub content: Vec<u8>,
}
