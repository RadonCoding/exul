<div align="center">
    <img src="https://github.com/RadonCoding/exul/blob/main/example.gif?raw=true" width="50%" />
</div>
<br/>

Scripting language that compiles to x86 assembly.

## How it works

### 1. Analysis
- **Lexing & Parsing**: Source code is scanned into tokens and structured into an Abstract Syntax Tree (AST).
- **Lowering**: AST is converted into an Intermediate Representation (IR).
- **Standard Library**: Written in the language itself and compiled on-demand when referenced.

### 2. Optimization
- **Peephole**: IR is scanned in multiple passes to simplify instruction sequences and collapse redundant operations.

### 3. Emission
- **Registers**: Symbols are assigned registers using liveness analysis, spilling to the stack only when necessary.
- **Binary**: Optimized IR is converted into x86 assembly and packaged into an executable binary.

### 4. Runtime
- **Bootstrap**: External imports are resolved at startup before the entry point, ensuring standalone execution.

## Usage
`cargo run --bin compiler -- <filename> [options]`
```text
--output <path>   path to the output executable
--ip <address>    base instruction pointer
--tokens          print lexical tokens
--ast             print abstract syntax tree
--ir              print intermediate representation
--asm             print x86 assembly
--function <name> filter output to a specific function
```

## Contributing
1. Fork it
2. Create your branch (`git checkout -b my-change`)
3. Commit your changes (`git commit -m "changed something"`)
4. Push to the branch (`git push origin my-change`)
5. Create new pull request
