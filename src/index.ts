import { fileURLToPath } from "node:url";

export enum Target {
  Wasm = "wasm",
  Wgsl = "wgsl",
}

export const DEFAULT_TARGET = Target.Wasm;

export const FUNCTION_ENTRY_SIZE = 36;
export const FUNCTIONS_BASE_OFFSET = 851_968;
export const STAGE1_MAX_FUNCTIONS = 512;

export const COMPILER_INPUT_PTR = 0;
export const INSTR_OFFSET_PTR_OFFSET = 4_096;
export const FUNCTIONS_COUNT_PTR_OFFSET = 851_960;

const encoder = new TextEncoder();
const decoder = new TextDecoder();

const MEMORY_INTRINSICS_MODULE_PATH = "/stdlib/memory.bp";
const memoryIntrinsicsSourceUrl = new URL("../stdlib/memory.bp", import.meta.url);
let memoryIntrinsicsSourcePromise: Promise<string> | null = null;

const MODULE_STATE_BASE = 1_048_576;
const MODULE_STORAGE_TOP_OFFSET = 4;
const MODULE_PATH_PTR = 1_024;
const MODULE_CONTENT_PTR = 4_096;
const DEFAULT_ENTRY_MODULE_PATH = "/entry.bp";

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

function loadMemoryIntrinsicsSource(): Promise<string> {
  if (!memoryIntrinsicsSourcePromise) {
    const file = Bun.file(memoryIntrinsicsSourceUrl);
    memoryIntrinsicsSourcePromise = file.text();
  }
  return memoryIntrinsicsSourcePromise;
}

function ensureModuleMemoryCapacity(memory: WebAssembly.Memory, required: number) {
  if (required <= memory.buffer.byteLength) {
    return;
  }
  const pageSize = 65_536;
  const additional = required - memory.buffer.byteLength;
  const pagesNeeded = Math.ceil(additional / pageSize);
  memory.grow(pagesNeeded);
}

function writeModuleString(memory: WebAssembly.Memory, ptr: number, text: string): number {
  const bytes = encoder.encode(text);
  ensureModuleMemoryCapacity(memory, ptr + bytes.length + 1);
  const view = new Uint8Array(memory.buffer);
  view.set(bytes, ptr);
  view[ptr + bytes.length] = 0;
  return bytes.length;
}

function zeroModuleMemory(memory: WebAssembly.Memory, ptr: number, length: number) {
  if (length <= 0) {
    return;
  }
  ensureModuleMemoryCapacity(memory, ptr + length);
  new Uint8Array(memory.buffer).fill(0, ptr, ptr + length);
}

function readModuleStorageTop(memory: WebAssembly.Memory): number {
  try {
    const view = new DataView(memory.buffer);
    return view.getInt32(MODULE_STATE_BASE + MODULE_STORAGE_TOP_OFFSET, true);
  } catch {
    return -1;
  }
}

function coerceToI32(value: number | bigint): number {
  return typeof value === "bigint" ? Number(value) : (value as number) | 0;
}

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
  return instance;
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

function readStageFailure(
  stage: "stage1" | "stage2",
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

  return `${stage} compilation failed (status ${producedLen}, functions=${functions}, instr_offset=${instrOffset}, compiled_functions=${compiledFunctions}${detail})`;
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
  const loadModuleFromSourceExport = instance.exports.loadModuleFromSource as
    | ((pathPtr: number, contentPtr: number) => number | bigint)
    | undefined;
  const compileFromPathExport = instance.exports.compileFromPath as
    | ((pathPtr: number) => number | bigint)
    | undefined;

  if (!memory) {
    throw new CompileError("stage2 compiler must export memory");
  }
  if (!compileExport) {
    throw new CompileError("stage2 compiler missing compile export");
  }

  if (typeof loadModuleFromSourceExport === "function" && typeof compileFromPathExport === "function") {
    const loadModuleFromSource = loadModuleFromSourceExport;
    const compileFromPath = compileFromPathExport;
    const memoryIntrinsicsSource = await loadMemoryIntrinsicsSource();

    const loadModule = (path: string, contents: string) => {
      let contentLength: number;
      writeModuleString(memory, MODULE_PATH_PTR, path);
      contentLength = writeModuleString(memory, MODULE_CONTENT_PTR, contents);
      let status: number;
      try {
        const result = loadModuleFromSource(MODULE_PATH_PTR, MODULE_CONTENT_PTR);
        status = coerceToI32(result);
      } catch (error) {
        const detail = error instanceof Error ? error.message : String(error);
        throw new CompileError(
          `stage2 compiler failed to load module '${path}': ${detail}`,
        );
      }
      if (!Number.isFinite(status) || status < 0) {
        const top = readModuleStorageTop(memory);
        throw new CompileError(readStageFailure("stage2", memory, top, status));
      }
      zeroModuleMemory(memory, MODULE_CONTENT_PTR, contentLength + 1);
    };

    loadModule(MEMORY_INTRINSICS_MODULE_PATH, memoryIntrinsicsSource);
    loadModule(DEFAULT_ENTRY_MODULE_PATH, source);

    let producedLen: number;
    try {
      const result = compileFromPath(MODULE_PATH_PTR);
      producedLen = coerceToI32(result);
    } catch (error) {
      const detail = error instanceof Error ? error.message : String(error);
      throw new CompileError(`stage2 compiler failed: ${detail}`);
    }

    const outputPtr = readModuleStorageTop(memory);
    if (!Number.isFinite(producedLen) || producedLen <= 0) {
      throw new CompileError(readStageFailure("stage2", memory, outputPtr, producedLen));
    }

    const view = new Uint8Array(memory.buffer);
    const wasm = view.slice(outputPtr, outputPtr + producedLen);
    return new Compilation(target, wasm);
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
    throw new CompileError(readStageFailure("stage2", memory, outputPtr, producedLen));
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
