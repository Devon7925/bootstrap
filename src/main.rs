use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::{self, Command, Stdio};

use bootstrap::FUNCTION_ENTRY_SIZE;
use bootstrap::FUNCTIONS_BASE_OFFSET;
use bootstrap::STAGE1_MAX_FUNCTIONS;
use bootstrap::{Target, compile};
use wasmi::{Engine, Linker, Memory, Module, Store, TypedFunc};

const COMPILER_SOURCE_PATH: &str = "compiler/ast_compiler.bp";
const COMPILER_OUTPUT_PATH: &str = "compiler.wasm";
const COMPILER_INPUT_PTR: usize = 0;

fn build_stage2_wasm() -> Result<(), String> {
    let source = fs::read_to_string(COMPILER_SOURCE_PATH)
        .map_err(|err| format!("failed to read '{COMPILER_SOURCE_PATH}': {err}"))?;
    let compilation = compile(&source, Target::Wasm).map_err(|err| err.to_string())?;
    let stage1_wasm = compilation.to_wasm().map_err(|err| err.to_string())?;

    let engine = Engine::default();
    let module = Module::new(&engine, stage1_wasm.as_slice())
        .map_err(|err| format!("failed to create stage1 module: {err}"))?;
    let mut store = Store::new(&engine, ());
    let linker = Linker::new(&engine);
    let instance = linker
        .instantiate(&mut store, &module)
        .and_then(|inst| inst.start(&mut store))
        .map_err(|err| format!("failed to instantiate stage1 module: {err}"))?;

    let memory: Memory = instance
        .get_memory(&mut store, "memory")
        .ok_or_else(|| "stage1 module does not export memory".to_string())?;
    let compile: TypedFunc<(i32, i32, i32), i32> =
        instance
            .get_typed_func(&mut store, "compile")
            .map_err(|err| format!("failed to find stage1 compile function: {err}"))?;

    let memory_size = memory
        .current_pages(&store)
        .to_bytes()
        .ok_or_else(|| "failed to compute stage1 memory size".to_string())?
        as usize;
    let reserved = FUNCTIONS_BASE_OFFSET + STAGE1_MAX_FUNCTIONS * FUNCTION_ENTRY_SIZE;
    if memory_size <= reserved {
        return Err("stage1 memory is smaller than reserved layout".into());
    }

    let output_ptr = source.len();

    memory
        .write(&mut store, COMPILER_INPUT_PTR, source.as_bytes())
        .map_err(|err| format!("failed to write stage1 source into memory: {err}"))?;

    let produced_len = compile
        .call(
            &mut store,
            (COMPILER_INPUT_PTR as i32, source.len() as i32, output_ptr as i32),
        )
        .map_err(|err| format!("stage1 compile trapped: {err}"))?;
    if produced_len <= 0 {
        return Err(format!(
            "stage1 compile returned non-positive length: {produced_len}"
        ));
    }

    let mut stage2_wasm = vec![0u8; produced_len as usize];
    memory
        .read(&store, output_ptr as usize, &mut stage2_wasm)
        .map_err(|err| format!("failed to read stage2 wasm from memory: {err}"))?;

    fs::write(COMPILER_OUTPUT_PATH, &stage2_wasm)
        .map_err(|err| format!("failed to write '{COMPILER_OUTPUT_PATH}': {err}"))?;

    println!("wrote stage2 wasm to {COMPILER_OUTPUT_PATH}");

    Ok(())
}

fn print_usage(program: &str) {
    eprintln!("Usage: {program} <input.bp> [options]");
    eprintln!("Options:");
    eprintln!("    -o <path>           Write output to file (.wasm)");
    eprintln!("    --emit wasm         Write wasm binary to stdout (default when no -o)");
    eprintln!("    --run               Execute the compiled module with Node.js");
    eprintln!("    --target <wasm|wgsl> Select the compilation target (default: wasm)");
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
        if let Err(err) = build_stage2_wasm() {
            eprintln!("{err}");
            process::exit(1);
        }
        return;
    }

    let input_path = args.remove(0);
    let mut output_path: Option<String> = None;
    let mut emit_flag: Option<bool> = None;
    let mut run = false;
    let mut target = Target::DEFAULT;

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
                    match value.as_str() {
                        "wasm" => emit_flag = Some(true),
                        "wat" => {
                            eprintln!("error: WAT output is no longer supported");
                            process::exit(1);
                        }
                        other => {
                            eprintln!("error: unsupported emit target '{other}'");
                            process::exit(1);
                        }
                    };
                }
                None => {
                    eprintln!("error: expected format after --emit");
                    process::exit(1);
                }
            }
        } else if arg == "--run" {
            run = true;
        } else if arg == "--target" {
            match iter.next() {
                Some(value) => match value.as_str() {
                    "wasm" => target = Target::Wasm,
                    "wgsl" => target = Target::Wgsl,
                    other => {
                        eprintln!("error: unsupported compilation target '{other}'");
                        process::exit(1);
                    }
                },
                None => {
                    eprintln!("error: expected value after --target");
                    process::exit(1);
                }
            }
        } else {
            eprintln!("error: unexpected argument '{arg}'");
            print_usage(&env::args().next().unwrap_or_else(|| "bootstrapc".into()));
            process::exit(1);
        }
    }

    if run && target != Target::Wasm {
        eprintln!("error: target '{target}' cannot be executed with --run");
        process::exit(1);
    }

    if target != Target::Wasm && output_path.is_none() && emit_flag.unwrap_or(true) {
        eprintln!("error: target '{target}' cannot be emitted to stdout as WebAssembly");
        process::exit(1);
    }

    let source = match fs::read_to_string(&input_path) {
        Ok(src) => src,
        Err(err) => {
            eprintln!("error: failed to read '{input_path}': {err}");
            process::exit(1);
        }
    };

    let compilation = match compile(&source, target) {
        Ok(compilation) => compilation,
        Err(err) => {
            eprintln!("{err}");
            process::exit(1);
        }
    };

    if let Some(path) = output_path.as_ref() {
        if let Some(ext) = Path::new(path)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase())
        {
            match ext.as_str() {
                "wasm" => {
                    if target != Target::Wasm {
                        eprintln!("error: target '{target}' cannot be written to '.wasm' files");
                        process::exit(1);
                    }
                }
                "wat" => {
                    eprintln!("error: WAT output is no longer supported");
                    process::exit(1);
                }
                "wgsl" => {
                    if target != Target::Wgsl {
                        eprintln!("error: target '{target}' cannot be written to '.wgsl' files");
                        process::exit(1);
                    }
                }
                other => {
                    eprintln!("error: unsupported output extension '.{other}'");
                    process::exit(1);
                }
            }
        }
    }

    let wasm_bytes = match compilation.to_wasm() {
        Ok(bytes) => bytes,
        Err(err) => {
            eprintln!("{err}");
            process::exit(1);
        }
    };

    if let Some(path) = output_path.as_ref() {
        if let Err(err) = fs::write(Path::new(path), &wasm_bytes) {
            eprintln!("error: failed to write '{path}': {err}");
            process::exit(1);
        }
    } else if emit_flag.unwrap_or(true) {
        if let Err(err) = io::stdout().write_all(&wasm_bytes) {
            eprintln!("error: failed to write wasm to stdout: {err}");
            process::exit(1);
        }
    }

    if run {
        if let Err(err) = run_with_node(&wasm_bytes) {
            eprintln!("error while running module with node: {err}");
            process::exit(1);
        }
    }
}
