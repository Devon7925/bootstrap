#!/usr/bin/env bun
import { fileURLToPath } from "node:url";
import { dirname, extname } from "node:path";
import { mkdir } from "node:fs/promises";

import process from "node:process";

import { Target, compile, parseTarget, DEFAULT_TARGET, CompileError, Compilation } from "./index";

const COMPILER_SOURCE_PATH = new URL("../compiler/ast_compiler.bp", import.meta.url);
const COMPILER_OUTPUT_PATH = new URL("../compiler.wasm", import.meta.url);

function printUsage(program: string) {
  console.error(`Usage: ${program} <input.bp> [options]`);
  console.error("Options:");
  console.error("    -o <path>            Write output to file (.wasm)");
  console.error("    --emit wasm          Write wasm binary to stdout (default when no -o)");
  console.error("    --run                Execute the compiled module with Bun");
  console.error("    --target <wasm|wgsl> Select the compilation target (default: wasm)");
}

async function runWithBun(wasm: Uint8Array) {
  const { instance } = await WebAssembly.instantiate(wasm, {});
  const main = (instance.exports as Record<string, unknown>).main;
  if (typeof main !== "function") {
    throw new CompileError("wasm module does not export 'main'");
  }

  const result = (main as () => unknown)();
  if (typeof result === "bigint") {
    console.log(result.toString());
  } else if (result !== undefined) {
    console.log(result);
  }
}

async function buildStage2Wasm() {
  const source = await Bun.file(COMPILER_SOURCE_PATH).text();
  const compilation = await compile(source, Target.Wasm);
  const wasm = compilation.intoWasm();
  await Bun.write(COMPILER_OUTPUT_PATH, wasm);
  console.log(`wrote stage2 wasm to ${fileURLToPath(COMPILER_OUTPUT_PATH)}`);
}

async function ensureParentDirectory(path: string) {
  const directory = dirname(path);
  if (!directory || directory === "." || directory === "") {
    return;
  }
  await mkdir(directory, { recursive: true }).catch(() => undefined);
}

async function main() {
  const args = Bun.argv.slice(2);
  const program = Bun.argv[1] ?? "bootstrap";

  if (args.length === 0) {
    try {
      await buildStage2Wasm();
      return;
    } catch (error) {
      if (error instanceof CompileError) {
        console.error(error.message);
      } else {
        console.error(error);
      }
      process.exit(1);
    }
  }

  const inputPath = args.shift();
  if (typeof inputPath !== "string" || inputPath.length === 0) {
    printUsage(program);
    process.exit(1);
  }

  let outputPath: string | null = null;
  let emitFlag: boolean | null = null;
  let run = false;
  let target: Target = DEFAULT_TARGET;

  while (args.length > 0) {
    const arg = args.shift();
    if (arg === undefined) {
      break;
    }

    if (arg === "-o") {
      const next = args.shift();
      if (typeof next !== "string" || next.length === 0) {
        console.error("error: expected path after -o");
        process.exit(1);
      }
      outputPath = next;
    } else if (arg === "--emit") {
      const next = args.shift();
      if (typeof next !== "string" || next.length === 0) {
        console.error("error: expected format after --emit");
        process.exit(1);
      }
      if (next === "wasm") {
        emitFlag = true;
      } else if (next === "wat") {
        console.error("error: WAT output is no longer supported");
        process.exit(1);
      } else {
        console.error(`error: unsupported emit target '${next}'`);
        process.exit(1);
      }
    } else if (arg === "--run") {
      run = true;
    } else if (arg === "--target") {
      const next = args.shift();
      if (typeof next !== "string" || next.length === 0) {
        console.error("error: expected value after --target");
        process.exit(1);
      }
      try {
        target = parseTarget(next);
      } catch (error) {
        if (error instanceof CompileError) {
          console.error(error.message);
        } else {
          console.error(error);
        }
        process.exit(1);
      }
    } else {
      console.error(`error: unexpected argument '${arg}'`);
      printUsage(program);
      process.exit(1);
    }
  }

  if (run && target !== Target.Wasm) {
    console.error(`error: target '${target}' cannot be executed with --run`);
    process.exit(1);
  }

  if (target !== Target.Wasm && !outputPath && (emitFlag ?? true)) {
    console.error(`error: target '${target}' cannot be emitted to stdout as WebAssembly`);
    process.exit(1);
  }

  let source: string;
  try {
    source = await Bun.file(inputPath).text();
  } catch (error) {
    console.error(`error: failed to read '${inputPath}': ${error}`);
    process.exit(1);
  }

  let compilation: Compilation;
  try {
    compilation = await compile(source, target);
  } catch (error) {
    if (error instanceof CompileError) {
      console.error(error.message);
    } else {
      console.error(error);
    }
    process.exit(1);
  }

  let wasmBytes: Uint8Array;
  try {
    wasmBytes = compilation.toWasm();
  } catch (error) {
    if (error instanceof CompileError) {
      console.error(error.message);
    } else {
      console.error(error);
    }
    process.exit(1);
  }

  if (outputPath) {
    const resolved = outputPath;
    const ext = extname(resolved).toLowerCase();
    if (ext === ".wasm" && target !== Target.Wasm) {
      console.error(`error: target '${target}' cannot be written to '.wasm' files`);
      process.exit(1);
    }
    if (ext === ".wat") {
      console.error("error: WAT output is no longer supported");
      process.exit(1);
    }
    if (ext === ".wgsl" && target !== Target.Wgsl) {
      console.error(`error: target '${target}' cannot be written to '.wgsl' files`);
      process.exit(1);
    }
    if (ext && ext !== ".wasm" && ext !== ".wgsl" && ext !== "") {
      console.error(`error: unsupported output extension '${ext}'`);
      process.exit(1);
    }

    try {
      await ensureParentDirectory(resolved);
      await Bun.write(resolved, wasmBytes);
    } catch (error) {
      console.error(`error: failed to write '${resolved}': ${error}`);
      process.exit(1);
    }
  } else {
    const emitToStdout = emitFlag ?? true;
    if (emitToStdout) {
      try {
        await Bun.write(Bun.stdout, wasmBytes);
      } catch (error) {
        console.error(`error: failed to write wasm to stdout: ${error}`);
        process.exit(1);
      }
    }
  }

  if (run) {
    try {
      await runWithBun(wasmBytes);
    } catch (error) {
      if (error instanceof CompileError) {
        console.error(error.message);
      } else {
        console.error(error);
      }
      process.exit(1);
    }
  }
}

await main();
