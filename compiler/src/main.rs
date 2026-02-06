mod ast;
mod lower;
mod lex;

use clap::Parser;
use emitter::convention::MicrosoftX64;
use iced_x86::{Decoder, DecoderOptions, Formatter, NasmFormatter};
use std::{error::Error, fs, path::PathBuf};

#[derive(Parser)]
#[command(author, version)]
struct Args {
    input: PathBuf,
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

fn dump(bytes: &[u8], ip: u64) {
    let mut decoder = Decoder::with_ip(64, bytes, ip, DecoderOptions::NONE);
    let mut formatter = NasmFormatter::new();
    formatter
        .options_mut()
        .set_space_after_operand_separator(true);

    let mut instructions = Vec::new();

    let mut max_bytes = 0;
    let mut max_mnemonic = 0;

    for instruction in &mut decoder {
        let mut output = String::new();
        formatter.format(&instruction, &mut output);

        let start = (instruction.ip() - ip) as usize;
        let end = start + instruction.len();
        let hex = bytes[start..end]
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>();

        max_bytes = max_bytes.max(hex.len());

        let parts = output.splitn(2, ' ').collect::<Vec<&str>>();
        max_mnemonic = max_mnemonic.max(parts[0].len());

        instructions.push((instruction.ip(), hex, output));
    }

    for (address, hex, output) in instructions {
        print!("0x{:04X}: {:<width_b$} ", address, hex, width_b = max_bytes);

        let parts = output.splitn(2, ' ').collect::<Vec<&str>>();

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
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let input = fs::read(&args.input)?;

    let tokens = lex::tokenize(&input)?;

    if args.tokens {
        println!("{:#?}", tokens);
    }

    let tree = ast::parse(tokens)?;

    if args.ast {
        println!("{:#?}", tree);
    }

    let module = lower::generate(tree)?;

    if args.ir {
        println!("{:#?}", module);
    }

    let bytes = emitter::emit::<MicrosoftX64>(args.ip, module)?;

    dump(&bytes, args.ip);

    Ok(())
}
