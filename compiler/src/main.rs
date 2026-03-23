mod ast;
mod lex;
mod lower;

use clap::Parser;
use emitter::{Assembly, convention::MicrosoftX64};
use iced_x86::{
    Decoder, DecoderOptions, Formatter, Instruction, MasmFormatter, SymbolResolver, SymbolResult,
};
use intermediate::Module;
use std::{collections::HashMap, error::Error, fs, path::PathBuf, time::Instant};

mod pe;

#[derive(Parser)]
#[command(author, version)]
struct Args {
    input: PathBuf,
    #[arg(long)]
    output: Option<PathBuf>,
    #[arg(
        long,
        default_value = "0",
        value_parser = |s: &str| u64::from_str_radix(s.trim_start_matches("0x"), 16)
    )]
    ip: u64,
    #[arg(long)]
    tokens: bool,
    #[arg(long)]
    ast: bool,
    #[arg(long)]
    ir: bool,
}

struct Symbols(HashMap<u64, String>);

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

fn build_symbols(assembly: &Assembly, module: &Module, ip: u64) -> HashMap<u64, String> {
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

fn dump(assembly: &Assembly, module: &Module, ip: u64) {
    let bytes = &assembly.bytes;
    let symbols = build_symbols(assembly, module, ip);
    let imports_start = assembly.blobs.last().map(|b| b.offset + b.len).unwrap_or(0);
    let imports_end = imports_start + module.imports.len() * 8;

    let mut formatter = MasmFormatter::with_options(Some(Box::new(Symbols(symbols.clone()))), None);
    formatter
        .options_mut()
        .set_space_after_operand_separator(true);

    let mut entries = Vec::new();

    let mut max_bytes = 0;
    let mut max_mnemonic = 0;
    let mut offset = 0;

    while offset < bytes.len() {
        if let Some(blob) = assembly.blobs.iter().find(|b| b.offset == offset) {
            let hex = bytes[offset..offset + blob.len]
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<String>();
            let display = match std::str::from_utf8(&blob.content[..blob.content.len() - 1]) {
                Ok(s) => format!("\"{}\"", s),
                Err(_) => format!("<{} bytes>", blob.len),
            };
            max_bytes = max_bytes.max(hex.len());
            entries.push((ip + offset as u64, hex, display));
            offset += blob.len;
            continue;
        }

        if assembly
            .blobs
            .iter()
            .any(|b| offset >= b.offset && offset < b.offset + b.len)
        {
            offset += 1;
            continue;
        }

        if offset >= imports_start && offset < imports_end {
            let hex = bytes[offset..offset + 8]
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
            &bytes[offset..],
            ip + offset as u64,
            DecoderOptions::NONE,
        );
        if !decoder.can_decode() {
            break;
        }

        let instruction = decoder.decode();
        let len = instruction.len();
        let hex = bytes[offset..offset + len]
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

    let mut was_label = false;

    for (address, hex, output) in &entries {
        if let Some(name) = symbols.get(address) {
            if !was_label {
                println!();
            }
            println!("{}:", name);
            was_label = true;
        } else {
            was_label = false;
        }

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
    println!();
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let input = fs::read(&args.input)?;
    let mut elapsed = std::time::Duration::ZERO;

    let start = Instant::now();
    let tokens = lex::tokenize(&input)?;
    elapsed += start.elapsed();

    if args.tokens {
        println!("{:#?}", tokens);
    }

    let start = Instant::now();
    let tree = ast::parse(tokens)?;
    elapsed += start.elapsed();

    if args.ast {
        println!("{:#?}", tree);
    }

    let start = Instant::now();
    let mut module = lower::generate(tree)?;
    elapsed += start.elapsed();

    if args.ir {
        println!("PRE-OPTIMIZATION:\n{:#?}", module);
    }

    let start = Instant::now();
    let assembly = emitter::emit::<MicrosoftX64>(args.ip, &mut module)?;
    elapsed += start.elapsed();

    if args.ir {
        println!("POST-OPTIMIZATION:\n{:#?}", module);
    }

    dump(&assembly, &module, args.ip);

    println!(
        "Compilation took {}.{:03} seconds",
        elapsed.as_secs(),
        elapsed.subsec_millis()
    );

    if let Some(output) = &args.output {
        let entry = module
            .entry
            .and_then(|e| assembly.functions.get(&module.functions[e].id))
            .copied()
            .unwrap_or(0);

        let pe = pe::build(&assembly.bytes, entry, &pe::PeOptions::default());
        fs::write(output, pe)?;
    }

    Ok(())
}
