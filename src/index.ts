import { fileURLToPath } from "node:url";

export enum Target {
  Wasm = "wasm",
  Wgsl = "wgsl",
}

export const DEFAULT_TARGET = Target.Wasm;

export const FUNCTION_ENTRY_SIZE = 68;
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
export const FAILURE_DETAIL_CAPACITY = 256;
const SCRATCH_FAILURE_PATH_PTR_OFFSET = 4_048;
const SCRATCH_FAILURE_PATH_LEN_OFFSET = 4_052;
const SCRATCH_FAILURE_LINE_OFFSET = 4_056;
const SCRATCH_FAILURE_COLUMN_OFFSET = 4_060;

export interface CompilerModuleSource {
  readonly path: string;
  readonly source: string;
}

export interface CompileOptions {
  readonly modules?: ReadonlyArray<CompilerModuleSource>;
  readonly entryPath?: string;
}

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

  #ensureWasmTarget(): void {
    if (this.#target !== Target.Wasm) {
      throw new CompileError(`target '${this.#target}' cannot be emitted as Wasm`);
    }
  }

  get target(): Target {
    return this.#target;
  }

  get wasm(): Uint8Array {
    return new Uint8Array(this.#wasm);
  }

  toWasm(): Uint8Array {
    this.#ensureWasmTarget();
    return new Uint8Array(this.#wasm);
  }

  intoWasm(): Uint8Array {
    this.#ensureWasmTarget();

    if (this.#consumed) {
      return new Uint8Array(this.#wasm);
    }

    this.#consumed = true;
    return this.#wasm;
  }
}

let compilerModulePromise: Promise<WebAssembly.Module> | null = null;

export interface CompileFailureDetails {
  readonly producedLength: number;
  readonly functions: number;
  readonly instructionOffset: number;
  readonly compiledFunctions: number;
  readonly detail?: string;
}

function safeReadI32(view: DataView, offset: number): number {
  if (offset < 0 || offset + 4 > view.byteLength) {
    return -1;
  }
  try {
    return view.getInt32(offset, true);
  } catch {
    return -1;
  }
}

export function describeCompilationFailure(
  memory: WebAssembly.Memory,
  outputPtr: number,
  producedLength: number,
): CompileFailureDetails {
  const view = new DataView(memory.buffer);
  const functions = safeReadI32(view, outputPtr + FUNCTIONS_COUNT_PTR_OFFSET);
  const instrOffset = safeReadI32(view, outputPtr + INSTR_OFFSET_PTR_OFFSET);

  let compiledFunctions = 0;
  if (functions > 0) {
    for (let index = 0; index < functions; index += 1) {
      const entry = outputPtr + FUNCTIONS_BASE_OFFSET + index * FUNCTION_ENTRY_SIZE;
      const codeLen = safeReadI32(view, entry + 16);
      if (codeLen > 0) {
        compiledFunctions += 1;
      } else {
        break;
      }
    }
  }

  let detail: string | undefined;
  const start = outputPtr;
  const end = Math.min(outputPtr + FAILURE_DETAIL_CAPACITY, memory.buffer.byteLength);
  if (end > start) {
    const detailBytes = new Uint8Array(memory.buffer.slice(start, end));
    const zeroIndex = detailBytes.indexOf(0);
    const slice = zeroIndex >= 0 ? detailBytes.subarray(0, zeroIndex) : detailBytes;
    const text = decoder.decode(slice).trim();
    if (text.length > 0) {
      detail = text;
    }
  }

  const line = safeReadI32(view, outputPtr + SCRATCH_FAILURE_LINE_OFFSET);
  const column = safeReadI32(view, outputPtr + SCRATCH_FAILURE_COLUMN_OFFSET);
  if (line > 0 && column > 0) {
    let path = DEFAULT_ENTRY_MODULE_PATH;
    const pathPtr = safeReadI32(view, outputPtr + SCRATCH_FAILURE_PATH_PTR_OFFSET);
    const pathLen = safeReadI32(view, outputPtr + SCRATCH_FAILURE_PATH_LEN_OFFSET);
    if (pathPtr > 0 && pathLen > 0) {
      try {
        const bytes = new Uint8Array(memory.buffer, pathPtr, pathLen);
        path = decoder.decode(bytes);
      } catch {
        path = DEFAULT_ENTRY_MODULE_PATH;
      }
    }
    if (!detail || !detail.startsWith("/")) {
      const message = detail && detail.length > 0 ? detail : "";
      detail = `${path}:${line}:${column}: ${message}`.trimEnd();
    }
  }

  return {
    producedLength,
    functions,
    instructionOffset: instrOffset,
    compiledFunctions,
    detail,
  };
}

function loadMemoryIntrinsicsSource(): Promise<string> {
  if (!memoryIntrinsicsSourcePromise) {
    const file = Bun.file(memoryIntrinsicsSourceUrl);
    memoryIntrinsicsSourcePromise = file.text();
  }
  return memoryIntrinsicsSourcePromise;
}

function growMemoryIfRequired(memory: WebAssembly.Memory, required: number) {
  const current = memory.buffer.byteLength;
  if (required <= current) {
    return;
  }
  const pageSize = 65_536;
  const additional = required - current;
  const pagesNeeded = Math.ceil(additional / pageSize);
  memory.grow(pagesNeeded);
}

function writeModuleString(memory: WebAssembly.Memory, ptr: number, text: string): number {
  const bytes = encoder.encode(text);
  growMemoryIfRequired(memory, ptr + bytes.length + 1);
  const view = new Uint8Array(memory.buffer);
  view.set(bytes, ptr);
  view[ptr + bytes.length] = 0;
  return bytes.length;
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

function readStageFailure(
  stage: "stage1" | "stage2",
  memory: WebAssembly.Memory,
  outputPtr: number,
  producedLen: number,
): string {
  const description = describeCompilationFailure(memory, outputPtr, producedLen);
  const detail = description.detail ? `, detail=\"${description.detail}\"` : "";
  return `${stage} compilation failed (status ${producedLen}, functions=${description.functions}, instr_offset=${description.instructionOffset}, compiled_functions=${description.compiledFunctions}${detail})`;
}

export async function compile(
  source: string,
  target: Target = DEFAULT_TARGET,
  options: CompileOptions = {},
): Promise<Compilation> {
  if (!source) {
    throw new CompileError("source must not be empty");
  }

  if (target !== Target.Wasm) {
    throw new CompileError(`target '${target}' is not supported yet`);
  }

  const entryPath = options.entryPath ?? DEFAULT_ENTRY_MODULE_PATH;
  const extraModules = options.modules ?? [];

  const instance = await instantiateCompiler();
  const memory = instance.exports.memory as WebAssembly.Memory | undefined;
  const loadModuleFromSourceExport = instance.exports.loadModuleFromSource as
    | ((pathPtr: number, contentPtr: number) => number | bigint)
    | undefined;
  const compileFromPathExport = instance.exports.compileFromPath as
    | ((pathPtr: number) => number | bigint)
    | undefined;

  if (!memory) {
    throw new CompileError("stage2 compiler must export memory");
  }
  if (typeof loadModuleFromSourceExport !== "function") {
    throw new CompileError("stage2 compiler missing module loading exports");
  }
  if (typeof compileFromPathExport !== "function") {
    throw new CompileError("stage2 compiler missing module loading exports");
  }
  const memoryIntrinsicsSource = await loadMemoryIntrinsicsSource();

  const loadModule = (path: string, contents: string) => {
    writeModuleString(memory, MODULE_PATH_PTR, path);
    const contentLength = writeModuleString(memory, MODULE_CONTENT_PTR, contents);
    let status: number;
    try {
      const result = loadModuleFromSourceExport(MODULE_PATH_PTR, MODULE_CONTENT_PTR);
      status = coerceToI32(result);
    } catch (error) {
      const detail = error instanceof Error ? error.message : String(error);
      throw new CompileError(
        `stage2 compiler failed to load module '${path}': ${detail}`,
      );
    }
    if (status < 0) {
      const top = readModuleStorageTop(memory);
      throw new CompileError(readStageFailure("stage2", memory, top, status));
    }
  };

  loadModule(MEMORY_INTRINSICS_MODULE_PATH, memoryIntrinsicsSource);
  for (const module of extraModules) {
    if (module.path === MEMORY_INTRINSICS_MODULE_PATH) {
      continue;
    }
    if (module.path === entryPath) {
      continue;
    }
    loadModule(module.path, module.source);
  }

  loadModule(entryPath, source);

  let producedLen: number;
  try {
    const result = compileFromPathExport(MODULE_PATH_PTR);
    producedLen = coerceToI32(result);
  } catch (error) {
    const detail = error instanceof Error ? error.message : String(error);
    throw new CompileError(`stage2 compiler failed: ${detail}`);
  }

  const outputPtr = readModuleStorageTop(memory);
  if (producedLen <= 0) {
    throw new CompileError(readStageFailure("stage2", memory, outputPtr, producedLen));
  }

  const view = new Uint8Array(memory.buffer);
  const wasm = view.slice(outputPtr, outputPtr + producedLen);
  return new Compilation(target, wasm);
}

export async function compileToWasm(
  source: string,
  options?: CompileOptions,
): Promise<Uint8Array> {
  const compilation = await compile(source, Target.Wasm, options ?? {});
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
