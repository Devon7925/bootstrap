use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::{self, Command, Stdio};

use bootstrap::compile;

fn print_usage(program: &str) {
    eprintln!("Usage: {program} <input.bp> [options]");
    eprintln!("Options:");
    eprintln!("    -o <path>           Write output to file (.wat or .wasm)");
    eprintln!("    --emit <wat|wasm>   Choose output format for stdout (default: wat)");
    eprintln!("    --run               Execute the compiled module with Node.js");
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EmitKind {
    Wat,
    Wasm,
}

fn run_with_node(wasm: &[u8]) -> Result<(), String> {
    const SCRIPT: &str = r#"const fs = require('fs');
const bytes = fs.readFileSync(0);
(async () => {
  const { instance } = await WebAssembly.instantiate(bytes, {});
  const main = instance.exports.main;
  if (typeof main !== 'function') {
    console.error("error: wasm module does not export 'main'");
    process.exit(1);
  }
  const result = main();
  if (result !== undefined) {
    if (typeof result === 'bigint') {
      console.log(result.toString());
    } else {
      console.log(result);
    }
  }
})().catch(err => {
  console.error(err);
  process.exit(1);
});"#;

    let mut child = Command::new("node")
        .arg("--eval")
        .arg(SCRIPT)
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|err| format!("failed to start node: {err}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(wasm)
            .map_err(|err| format!("failed to send wasm bytes to node: {err}"))?;
    }

    let status = child
        .wait()
        .map_err(|err| format!("failed to wait for node: {err}"))?;

    if !status.success() {
        let code = status
            .code()
            .map(|code| code.to_string())
            .unwrap_or_else(|| "signal".into());
        return Err(format!("node exited with status {code}"));
    }

    Ok(())
}

fn main() {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        print_usage(&env::args().next().unwrap_or_else(|| "bootstrapc".into()));
        process::exit(1);
    }

    let input_path = args.remove(0);
    let mut output_path: Option<String> = None;
    let mut emit_flag: Option<EmitKind> = None;
    let mut run = false;

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
        } else if arg == "--emit" {
            match iter.next() {
                Some(value) => {
                    let kind = match value.as_str() {
                        "wat" => EmitKind::Wat,
                        "wasm" => EmitKind::Wasm,
                        other => {
                            eprintln!("error: unsupported emit target '{other}'");
                            process::exit(1);
                        }
                    };
                    emit_flag = Some(kind);
                }
                None => {
                    eprintln!("error: expected format after --emit");
                    process::exit(1);
                }
            }
        } else if arg == "--run" {
            run = true;
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

    let compilation = match compile(&source) {
        Ok(compilation) => compilation,
        Err(err) => {
            eprintln!("{err}");
            process::exit(1);
        }
    };

    let emit_from_ext = output_path.as_ref().and_then(|path| {
        Path::new(path)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase())
            .and_then(|ext| match ext.as_str() {
                "wat" => Some(EmitKind::Wat),
                "wasm" => Some(EmitKind::Wasm),
                _ => None,
            })
    });

    if let (Some(cli_emit), Some(ext_emit)) = (emit_flag, emit_from_ext) {
        if cli_emit != ext_emit {
            eprintln!("error: emit format flag does not match output extension");
            process::exit(1);
        }
    }

    let emit_kind = emit_flag.or(emit_from_ext).unwrap_or(EmitKind::Wat);

    let wasm_bytes = if run || matches!(emit_kind, EmitKind::Wasm) {
        match compilation.to_wasm() {
            Ok(bytes) => Some(bytes),
            Err(err) => {
                eprintln!("{err}");
                process::exit(1);
            }
        }
    } else {
        None
    };

    if let Some(path) = output_path.as_ref() {
        let result = match emit_kind {
            EmitKind::Wat => fs::write(Path::new(path), compilation.wat()),
            EmitKind::Wasm => wasm_bytes
                .as_ref()
                .map(|bytes| fs::write(Path::new(path), bytes))
                .unwrap_or_else(|| Err(io::Error::new(io::ErrorKind::Other, "missing wasm bytes"))),
        };

        if let Err(err) = result {
            eprintln!("error: failed to write '{path}': {err}");
            process::exit(1);
        }
    } else {
        match emit_kind {
            EmitKind::Wat => println!("{}", compilation.wat()),
            EmitKind::Wasm => {
                if let Some(bytes) = wasm_bytes.as_ref() {
                    if let Err(err) = io::stdout().write_all(bytes) {
                        eprintln!("error: failed to write wasm to stdout: {err}");
                        process::exit(1);
                    }
                }
            }
        }
    }

    if run {
        if let Some(bytes) = wasm_bytes.as_ref() {
            if let Err(err) = run_with_node(bytes) {
                eprintln!("error while running module with node: {err}");
                process::exit(1);
            }
        }
    }
}
