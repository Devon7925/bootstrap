import { fileURLToPath } from "node:url";

export enum Target {
  Wasm = "wasm",
  Wgsl = "wgsl",
}

export const DEFAULT_TARGET = Target.Wasm;

export const FUNCTION_ENTRY_SIZE = 32;
export const FUNCTIONS_BASE_OFFSET = 851_968;
export const STAGE1_MAX_FUNCTIONS = 512;

export const COMPILER_INPUT_PTR = 0;
export const INSTR_OFFSET_PTR_OFFSET = 4_096;
export const FUNCTIONS_COUNT_PTR_OFFSET = 851_960;

const encoder = new TextEncoder();
const decoder = new TextDecoder();

export class CompileError extends Error {
  override readonly name = "CompileError";

  constructor(message: string) {
    super(`error: ${message}`);
  }
}

export class Compilation {
  #target: Target;
  #wasm: Uint8Array;
  #consumed = false;

  constructor(target: Target, wasm: Uint8Array) {
    this.#target = target;
    this.#wasm = wasm;
  }

  get target(): Target {
    return this.#target;
  }

  get wasm(): Uint8Array {
    return new Uint8Array(this.#wasm);
  }

  toWasm(): Uint8Array {
    if (this.#target !== Target.Wasm) {
      throw new CompileError(`target '${this.#target}' cannot be emitted as Wasm`);
    }
    return new Uint8Array(this.#wasm);
  }

  intoWasm(): Uint8Array {
    if (this.#target !== Target.Wasm) {
      throw new CompileError(`target '${this.#target}' cannot be emitted as Wasm`);
    }

    if (this.#consumed) {
      return new Uint8Array(this.#wasm);
    }

    this.#consumed = true;
    return this.#wasm;
  }
}

let compilerModulePromise: Promise<WebAssembly.Module> | null = null;

async function loadCompilerModule(): Promise<WebAssembly.Module> {
  if (!compilerModulePromise) {
    const wasmUrl = new URL("../compiler.wasm", import.meta.url);
    compilerModulePromise = (async () => {
      const wasmFile = Bun.file(wasmUrl);
      if (!(await wasmFile.exists())) {
        const path = fileURLToPath(wasmUrl);
        throw new CompileError(`stage2 compiler not found at '${path}'`);
      }
      const wasmBytes = await wasmFile.arrayBuffer();
      return WebAssembly.compile(wasmBytes);
    })();
  }

  return compilerModulePromise;
}

async function instantiateCompiler(): Promise<WebAssembly.Instance> {
  const module = await loadCompilerModule();
  const instance = await WebAssembly.instantiate(module, {});
  return instance.instance ?? instance;
}

function ensureCapacity(memory: WebAssembly.Memory, required: number) {
  const pageSize = 65_536;
  const current = memory.buffer.byteLength;
  if (current >= required) {
    return;
  }

  const additional = required - current;
  const pagesNeeded = Math.ceil(additional / pageSize);
  try {
    memory.grow(pagesNeeded);
  } catch {
    throw new CompileError(
      `stage2 compiler memory layout does not leave space for output buffer (required ${required}, current ${current})`,
    );
  }
}

function readStage2Failure(
  memory: WebAssembly.Memory,
  outputPtr: number,
  producedLen: number,
): string {
  const view = new DataView(memory.buffer);
  let functions = -1;
  let instrOffset = -1;
  try {
    functions = view.getInt32(outputPtr + FUNCTIONS_COUNT_PTR_OFFSET, true);
  } catch {
    functions = -1;
  }
  try {
    instrOffset = view.getInt32(outputPtr + INSTR_OFFSET_PTR_OFFSET, true);
  } catch {
    instrOffset = -1;
  }

  let compiledFunctions = 0;
  if (functions > 0) {
    for (let index = 0; index < functions; index++) {
      const entry = outputPtr + FUNCTIONS_BASE_OFFSET + index * FUNCTION_ENTRY_SIZE;
      try {
        const codeLen = view.getInt32(entry + 16, true);
        if (codeLen > 0) {
          compiledFunctions += 1;
        } else {
          break;
        }
      } catch {
        break;
      }
    }
  }

  let detail = "";
  const start = outputPtr;
  const end = Math.min(outputPtr + 256, memory.buffer.byteLength);
  if (end > start) {
    const detailBytes = new Uint8Array(memory.buffer.slice(start, end));
    const zeroIndex = detailBytes.indexOf(0);
    const slice = zeroIndex >= 0 ? detailBytes.subarray(0, zeroIndex) : detailBytes;
    const text = decoder.decode(slice).trim();
    if (text.length > 0) {
      detail = `, detail=\"${text}\"`;
    }
  }

  return `stage2 compilation failed (status ${producedLen}, functions=${functions}, instr_offset=${instrOffset}, compiled_functions=${compiledFunctions}${detail})`;
}

export async function compile(source: string, target: Target = DEFAULT_TARGET): Promise<Compilation> {
  if (!source) {
    throw new CompileError("source must not be empty");
  }

  if (target !== Target.Wasm) {
    throw new CompileError(`target '${target}' is not supported yet`);
  }

  const instance = await instantiateCompiler();
  const memory = instance.exports.memory as WebAssembly.Memory | undefined;
  const compileExport = instance.exports.compile as
    | ((inputPtr: number, inputLen: number, outputPtr: number) => number)
    | undefined;

  if (!memory) {
    throw new CompileError("stage2 compiler must export memory");
  }
  if (!compileExport) {
    throw new CompileError("stage2 compiler missing compile export");
  }

  const reserved = FUNCTIONS_BASE_OFFSET + STAGE1_MAX_FUNCTIONS * FUNCTION_ENTRY_SIZE;
  if (memory.buffer.byteLength <= reserved) {
    throw new CompileError(
      "stage2 compiler memory layout does not leave space for output buffer",
    );
  }

  const sourceBytes = encoder.encode(source);
  const outputPtr = sourceBytes.length;

  ensureCapacity(memory, outputPtr + reserved + 1);

  let memoryView = new Uint8Array(memory.buffer);
  memoryView.set(sourceBytes, COMPILER_INPUT_PTR);

  const producedLen = compileExport(COMPILER_INPUT_PTR, sourceBytes.length, outputPtr);
  memoryView = new Uint8Array(memory.buffer);

  if (producedLen <= 0) {
    throw new CompileError(readStage2Failure(memory, outputPtr, producedLen));
  }

  const wasm = memoryView.slice(outputPtr, outputPtr + producedLen);
  return new Compilation(target, wasm);
}

export async function compileToWasm(source: string): Promise<Uint8Array> {
  const compilation = await compile(source, Target.Wasm);
  return compilation.intoWasm();
}

export function parseTarget(value: string): Target {
  switch (value) {
    case "wasm":
      return Target.Wasm;
    case "wgsl":
      return Target.Wgsl;
    default:
      throw new CompileError(`unsupported compilation target '${value}'`);
  }
}
