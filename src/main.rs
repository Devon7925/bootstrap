use std::env;
use std::fs;
use std::path::Path;
use std::process;

fn print_usage(program: &str) {
    eprintln!("Usage: {program} <input.bp> [-o <output.wat>]");
}

fn main() {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        print_usage(&env::args().next().unwrap_or_else(|| "bootstrapc".into()));
        process::exit(1);
    }

    let input_path = args.remove(0);
    let mut output_path: Option<String> = None;

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "-o" {
            match iter.next() {
                Some(path) => output_path = Some(path.clone()),
                None => {
                    eprintln!("error: expected path after -o");
                    process::exit(1);
                }
            }
        } else {
            eprintln!("error: unexpected argument '{arg}'");
            print_usage(&env::args().next().unwrap_or_else(|| "bootstrapc".into()));
            process::exit(1);
        }
    }

    let source = match fs::read_to_string(&input_path) {
        Ok(src) => src,
        Err(err) => {
            eprintln!("error: failed to read '{input_path}': {err}");
            process::exit(1);
        }
    };

    match bootstrap::compile_to_wat(&source) {
        Ok(wat) => {
            if let Some(path) = output_path {
                if let Err(err) = fs::write(Path::new(&path), wat) {
                    eprintln!("error: failed to write '{path}': {err}");
                    process::exit(1);
                }
            } else {
                println!("{wat}");
            }
        }
        Err(err) => {
            eprintln!("{err}");
            process::exit(1);
        }
    }
}
