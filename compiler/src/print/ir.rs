use intermediate::Module;

pub fn ir(module: &Module, filter: Option<&String>) -> bool {
    let mut first = true;
    let mut printed = false;

    for f in &module.functions {
        if let Some(filter) = filter {
            if &f.name != filter {
                continue;
            }
        }

        if !first {
            println!();
        }

        print!("{}(", f.name);

        for (i, param) in f.params.iter().enumerate() {
            if i > 0 {
                print!(", ");
            }
            print!("{:?}", param);
        }

        println!(") {{");

        for instruction in &f.instructions {
            println!("  {:?}", instruction.kind);
        }

        println!("}}");

        first = false;
        printed = true;
    }

    printed
}
