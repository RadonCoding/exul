pub struct PeOptions {
    pub image_base: u64,
    pub stack_reserve: u64,
    pub stack_commit: u64,
}

impl Default for PeOptions {
    fn default() -> Self {
        Self {
            image_base: 0x140000000,
            stack_reserve: 0x100000,
            stack_commit: 0x1000,
        }
    }
}

pub fn build(assembly: &[u8], entry: usize, options: &PeOptions) -> Vec<u8> {
    const FILE_ALIGN: usize = 0x200;
    const SECTION_ALIGN: usize = 0x1000;

    // File layout
    let pe_sig_offset: usize = 0x40; // "PE\0\0"
    let file_header_offset = pe_sig_offset + 4; // IMAGE_FILE_HEADER (20 bytes)
    let opt_header_offset = file_header_offset + 20; // IMAGE_OPTIONAL_HEADER64 (240 bytes)
    let section_table_offset = opt_header_offset + 240; // IMAGE_SECTION_HEADER (40 bytes)
    let headers_size = align(section_table_offset + 40, FILE_ALIGN);

    let code_rva: usize = SECTION_ALIGN;
    let code_virtual_size: usize = assembly.len();
    let code_raw_size: usize = align(assembly.len(), FILE_ALIGN);
    let image_size: usize = align(
        code_rva + align(code_virtual_size, SECTION_ALIGN),
        SECTION_ALIGN,
    );

    let mut pe = vec![0u8; headers_size + code_raw_size];

    // DOS header
    pe[0..2].copy_from_slice(b"MZ");
    write_u32(&mut pe, 0x3C, pe_sig_offset as u32);

    // PE signature
    pe[pe_sig_offset..pe_sig_offset + 4].copy_from_slice(b"PE\0\0");

    // IMAGE_FILE_HEADER
    let fh = file_header_offset;
    write_u16(&mut pe, fh + 0, 0x8664); // Machine: AMD64
    write_u16(&mut pe, fh + 2, 1); // NumberOfSections
    write_u16(&mut pe, fh + 16, 240); // SizeOfOptionalHeader
    write_u16(&mut pe, fh + 18, 0x0022); // Characteristics: executable, large address aware

    // IMAGE_OPTIONAL_HEADER64
    let oh = opt_header_offset;
    write_u16(&mut pe, oh + 0, 0x020B); // Magic: PE32+
    write_u32(&mut pe, oh + 4, code_raw_size as u32); // SizeOfCode
    write_u32(&mut pe, oh + 16, (code_rva + entry) as u32); // AddressOfEntryPoint
    write_u32(&mut pe, oh + 20, code_rva as u32); // BaseOfCode
    write_u64(&mut pe, oh + 24, options.image_base); // ImageBase
    write_u32(&mut pe, oh + 32, SECTION_ALIGN as u32); // SectionAlignment
    write_u32(&mut pe, oh + 36, FILE_ALIGN as u32); // FileAlignment
    write_u16(&mut pe, oh + 40, 6); // MajorOperatingSystemVersion
    write_u16(&mut pe, oh + 48, 6); // MajorSubsystemVersion
    write_u32(&mut pe, oh + 56, image_size as u32); // SizeOfImage
    write_u32(&mut pe, oh + 60, headers_size as u32); // SizeOfHeaders
    write_u16(&mut pe, oh + 68, 3); // Subsystem: console
    write_u64(&mut pe, oh + 72, options.stack_reserve); // SizeOfStackReserve
    write_u64(&mut pe, oh + 80, options.stack_commit); // SizeOfStackCommit
    write_u32(&mut pe, oh + 108, 16); // NumberOfRvaAndSizes

    const IMAGE_SCN_CNT_CODE: u32 = 0x00000020;
    const IMAGE_SCN_MEM_EXECUTE: u32 = 0x20000000;
    const IMAGE_SCN_MEM_READ: u32 = 0x40000000;
    const IMAGE_SCN_MEM_WRITE: u32 = 0x80000000;

    // IMAGE_SECTION_HEADER .text
    let sh = section_table_offset;
    pe[sh..sh + 8].copy_from_slice(b".text\0\0\0");
    write_u32(&mut pe, sh + 8, code_virtual_size as u32); // VirtualSize
    write_u32(&mut pe, sh + 12, code_rva as u32); // VirtualAddress
    write_u32(&mut pe, sh + 16, code_raw_size as u32); // SizeOfRawData
    write_u32(&mut pe, sh + 20, headers_size as u32); // PointerToRawData
    write_u32(
        &mut pe,
        sh + 36,
        IMAGE_SCN_CNT_CODE | IMAGE_SCN_MEM_EXECUTE | IMAGE_SCN_MEM_READ | IMAGE_SCN_MEM_WRITE,
    ); // Characteristics

    // Code
    pe[headers_size..headers_size + assembly.len()].copy_from_slice(assembly);

    pe
}

fn align(value: usize, alignment: usize) -> usize {
    (value + alignment - 1) & !(alignment - 1)
}

fn write_u16(buf: &mut [u8], offset: usize, value: u16) {
    buf[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn write_u32(buf: &mut [u8], offset: usize, value: u32) {
    buf[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_u64(buf: &mut [u8], offset: usize, value: u64) {
    buf[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}
