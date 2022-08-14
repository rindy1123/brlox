use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::{env, process::exit};

use vm::{InterpretError, VM};

mod chunk;
mod compiler;
mod disassembler;
mod scan;
mod token;
mod vm;

fn repl() {
    let stdin = io::stdin();
    let mut vm = VM::new();
    loop {
        print!("> ");
        let mut buffer = String::new();
        io::stdout().flush().unwrap();
        let bytes = stdin.read_line(&mut buffer).unwrap();
        if bytes == 0 {
            exit(0);
        }
        if buffer == "exit\n" {
            exit(0)
        }
        let line = buffer.trim().to_string();
        vm.interpret(&line);
    }
}

fn run_file(path: &Path) {
    let source = fs::read_to_string(path).unwrap();
    let mut vm = VM::new();
    if let Err(e) = vm.interpret(&source) {
        match e {
            InterpretError::CompileError => exit(65),
            InterpretError::RuntimeError => exit(70),
        }
    };
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let args_length = args.len();
    if args_length == 1 {
        repl();
    } else if args_length == 2 {
        let path = Path::new(&args[1]);
        run_file(path);
    } else {
        eprintln!("Usage: brlox [path]");
        exit(64)
    }
}
