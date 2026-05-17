use clap::Parser;
use std::path::PathBuf;

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
