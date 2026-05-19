use emitter::assembly::Assembly;
use iced_x86::{Decoder, DecoderOptions, Formatter, MasmFormatter};
use intermediate::Module;

use crate::symbols::{Symbols, build_symbols};

pub fn asm(assembly: &Assembly, module: &Module, ip: u64, filter: Option<&str>) -> bool {
    let symbols = build_symbols(assembly, module, ip);

    let imports_start = assembly
        .sections
        .last()
        .map(|b| b.offset + b.len)
        .unwrap_or(0);
    let imports_end = imports_start + module.imports.len() * 8;

    let mut formatter = MasmFormatter::with_options(Some(Box::new(Symbols(symbols.clone()))), None);
    formatter
        .options_mut()
        .set_space_after_operand_separator(true);

    let mut entries = Vec::new();

    let mut max_bytes = 0;
    let mut max_mnemonic = 0;

    let mut offset = 0;

    while offset < assembly.bytes.len() {
        if let Some(blob) = assembly.sections.iter().find(|b| b.offset == offset) {
            let hex = assembly.bytes[offset..offset + blob.len]
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<String>();

            let display = match str::from_utf8(&blob.content[..blob.content.len() - 1]) {
                Ok(s) => format!("\"{}\"", s),
                Err(_) => format!("<{} bytes>", blob.len),
            };

            max_bytes = max_bytes.max(hex.len());
            entries.push((ip + offset as u64, hex, display));
            offset += blob.len;
            continue;
        }

        if assembly
            .sections
            .iter()
            .any(|b| offset >= b.offset && offset < b.offset + b.len)
        {
            offset += 1;
            continue;
        }

        if offset >= imports_start && offset < imports_end {
            let hex = assembly.bytes[offset..offset + 8]
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<String>();

            max_bytes = max_bytes.max(hex.len());
            entries.push((ip + offset as u64, hex, String::new()));
            offset += 8;
            continue;
        }

        let mut decoder = Decoder::with_ip(
            64,
            &assembly.bytes[offset..],
            ip + offset as u64,
            DecoderOptions::NONE,
        );

        if !decoder.can_decode() {
            break;
        }

        let instruction = decoder.decode();
        let len = instruction.len();

        let hex = assembly.bytes[offset..offset + len]
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>();

        let mut output = String::new();
        formatter.format(&instruction, &mut output);

        let parts = output.splitn(2, ' ').collect::<Vec<_>>();
        max_bytes = max_bytes.max(hex.len());
        max_mnemonic = max_mnemonic.max(parts[0].len());

        entries.push((ip + offset as u64, hex, output));
        offset += len;
    }

    let mut printed = false;
    let mut printing = filter.is_none();
    let mut was_label = true;

    for (address, hex, output) in &entries {
        if let Some(name) = symbols.get(address) {
            if let Some(f) = filter {
                printing = name == f;
            } else {
                printing = true;
            }

            if printing {
                if !was_label {
                    println!();
                }
                println!("{}:", name);
                printed = true;
            }

            was_label = printing;
        } else {
            if !printing {
                continue;
            }
            was_label = false;
        }

        if !printing {
            continue;
        }

        printed = true;
        print!(
            "  0x{:04X}: {:<width_b$}  ",
            address,
            hex,
            width_b = max_bytes
        );

        let parts = output.splitn(2, ' ').collect::<Vec<_>>();

        if parts.len() == 2 {
            println!(
                "{:<width_m$} {}",
                parts[0],
                parts[1],
                width_m = max_mnemonic
            );
        } else {
            println!("{}", output);
        }
    }

    printed
}
