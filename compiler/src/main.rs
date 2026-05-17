mod ast;
mod lex;
mod linker;
mod lower;
mod print;
mod symbols;

use clap::Parser;
use emitter::{convention::MicrosoftX64, peephole};
use std::{error::Error, fs, path::PathBuf, time::Instant};

use linker::{Format, LinkerOptions};

#[derive(Parser)]
#[command(author, version)]
pub struct Args {
    pub input: PathBuf,
    #[arg(long)]
    pub output: Option<PathBuf>,
    #[arg(
        long,
        default_value = "0",
        value_parser = |s: &str| u64::from_str_radix(s.trim_start_matches("0x"), 16)
    )]
    pub ip: u64,
    #[arg(long)]
    pub tokens: bool,
    #[arg(long)]
    pub ast: bool,
    #[arg(long)]
    pub ir: bool,
    #[arg(long)]
    pub asm: bool,
    #[arg(long)]
    pub function: Option<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let input = fs::read(&args.input)?;
    let mut elapsed = std::time::Duration::ZERO;
    let mut space = false;

    let start = Instant::now();
    let tokens = lex::tokenize(&input)?;
    elapsed += start.elapsed();

    if args.tokens {
        println!("{:#?}", tokens);
        space = true;
    }

    let start = Instant::now();
    let tree = ast::parse(tokens)?;
    elapsed += start.elapsed();

    if args.ast {
        if space {
            println!();
        }
        println!("{:#?}", tree);
        space = true;
    }

    let start = Instant::now();
    let mut module = lower::generate(tree)?;
    elapsed += start.elapsed();

    let start = Instant::now();
    peephole::optimize(&mut module);
    elapsed += start.elapsed();

    if args.ir {
        let printed = print::ir(&module, args.function.as_ref());

        if printed {
            println!();
            space = false;
        }
    }

    let start = Instant::now();
    let assembly = emitter::emit::<MicrosoftX64>(args.ip, &mut module)?;
    elapsed += start.elapsed();

    if args.asm {
        let printed = print::asm(&assembly, &module, args.ip, args.function.as_deref());

        if printed {
            space = true;
        }
    }

    if space {
        println!();
    }

    println!(
        "Compilation took {}.{:03} seconds.",
        elapsed.as_secs(),
        elapsed.subsec_millis()
    );

    if let Some(output) = &args.output {
        let entry = module
            .entry
            .and_then(|e| assembly.functions.get(&module.functions[e].id))
            .copied()
            .unwrap_or(0);

        let image = linker::build(
            &assembly.bytes,
            entry,
            &LinkerOptions::default(),
            Format::Pe64,
        );
        fs::write(output, image)?;
    }

    Ok(())
}
