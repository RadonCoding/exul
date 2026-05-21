use object::{
    pe,
    write::pe::{NtHeaders, Writer as PeWriter},
};

pub struct LinkerOptions {
    pub image_base: u64,
    pub stack_reserve: u64,
    pub stack_commit: u64,
}

impl Default for LinkerOptions {
    fn default() -> Self {
        Self {
            image_base: 0x0000000140000000,
            stack_reserve: 0x0000000000100000,
            stack_commit: 0x0000000000001000,
        }
    }
}

pub enum Format {
    Pe64,
}

pub fn build(assembly: &[u8], entry: usize, options: &LinkerOptions, format: Format) -> Vec<u8> {
    match format {
        Format::Pe64 => build_pe(assembly, entry, options),
    }
}

fn build_pe(assembly: &[u8], entry: usize, options: &LinkerOptions) -> Vec<u8> {
    let mut buffer = Vec::new();
    let mut w = PeWriter::new(true, 0x1000, 0x200, &mut buffer);

    w.reserve_dos_header_and_stub();
    w.reserve_nt_headers(pe::IMAGE_NUMBEROF_DIRECTORY_ENTRIES as usize);
    w.reserve_section_headers(1);

    let text = w.reserve_section(
        *b".text\0\0\0",
        pe::IMAGE_SCN_CNT_CODE
            | pe::IMAGE_SCN_MEM_EXECUTE
            | pe::IMAGE_SCN_MEM_READ
            | pe::IMAGE_SCN_MEM_WRITE,
        assembly.len() as u32,
        assembly.len() as u32,
    );

    w.write_dos_header_and_stub().unwrap();
    w.write_nt_headers(NtHeaders {
        machine: pe::IMAGE_FILE_MACHINE_AMD64,
        time_date_stamp: 0,
        characteristics: pe::IMAGE_FILE_EXECUTABLE_IMAGE | pe::IMAGE_FILE_LARGE_ADDRESS_AWARE,
        major_linker_version: 0,
        minor_linker_version: 0,
        address_of_entry_point: text.virtual_address + entry as u32,
        image_base: options.image_base,
        major_operating_system_version: 6,
        minor_operating_system_version: 0,
        major_image_version: 0,
        minor_image_version: 0,
        major_subsystem_version: 6,
        minor_subsystem_version: 0,
        subsystem: pe::IMAGE_SUBSYSTEM_WINDOWS_CUI,
        dll_characteristics: 0,
        size_of_stack_reserve: options.stack_reserve,
        size_of_stack_commit: options.stack_commit,
        size_of_heap_reserve: 0,
        size_of_heap_commit: 0,
    });
    w.write_section_headers();
    w.write_section(text.file_offset, assembly);
    w.write_file_align();

    buffer
}
