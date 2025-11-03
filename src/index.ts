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
const SCRATCH_TYPE_METADATA_DEBUG_CONTEXT_OFFSET = 4_032;
const SCRATCH_TYPE_METADATA_DEBUG_SUBJECT_OFFSET = 4_036;
const SCRATCH_TYPE_METADATA_DEBUG_EXTRA_OFFSET = 4_040;
const TYPE_METADATA_DEBUG_LAST_CONTEXT_OFFSET = 5_020;
const TYPE_METADATA_DEBUG_LAST_SUBJECT_OFFSET = 5_024;
const TYPE_METADATA_DEBUG_LAST_EXTRA_OFFSET = 5_028;
const SCRATCH_MODULE_BASE_OFFSET = 4_080;
const SCRATCH_MODULE_LEN_OFFSET = 4_084;
const SCRATCH_MODULE_INDEX_OFFSET = 4_088;
const MODULE_TABLE_OFFSET = 8;
const MODULE_ENTRY_FIELD_COUNT = 6;
const MODULE_ENTRY_SIZE = MODULE_ENTRY_FIELD_COUNT * 4;
const MODULE_ENTRY_PATH_PTR_FIELD = 0;
const MODULE_ENTRY_PATH_LEN_FIELD = 1;
const MODULE_ENTRY_CONTENT_PTR_FIELD = 2;
const MODULE_ENTRY_CONTENT_LEN_FIELD = 3;
const MODULE_ENTRY_LINE_INDEX_FIELD = 4;
const WORD_SIZE = 4;
const AST_MAX_FUNCTIONS = 1_024;
const AST_FUNCTION_ENTRY_SIZE = 68;
const AST_NAMES_CAPACITY = 131_072;
const AST_CONSTANT_ENTRY_SIZE = 28;
const AST_CONSTANT_ENTRY_NAME_OFFSET = 0;
const AST_CONSTANT_ENTRY_NAME_LEN_OFFSET = 4;
const AST_CONSTANT_ENTRY_TYPE_OFFSET = 12;
const AST_CONSTANT_ENTRY_EXPR_INDEX_OFFSET = 16;
const AST_CONSTANT_ENTRY_EVAL_STATE_OFFSET = 20;
const AST_CONSTANT_ENTRY_MODULE_INDEX_OFFSET = 24;
const AST_CONSTANTS_CAPACITY = 1_024;
const AST_CALL_DATA_CAPACITY =
  131_072 - ((AST_CONSTANTS_CAPACITY * AST_CONSTANT_ENTRY_SIZE + WORD_SIZE) >> 2);
const AST_CONSTANT_EVAL_STATE_EVALUATED = 2;
const SCRATCH_INSTR_CAPACITY = 131_072;
const SCRATCH_FN_BASE_OFFSET = 921_600;
const AST_EXPR_ENTRY_SIZE = 20;
const AST_EXPR_LOCATION_OFFSET = 12;

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
  inputLength = -1,
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

  const improvedDetail = maybeFormatTypeMetadataFailure(
    memory,
    outputPtr,
    inputLength,
    detail,
  );
  if (improvedDetail) {
    detail = improvedDetail;
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

function maybeFormatTypeMetadataFailure(
  memory: WebAssembly.Memory,
  outputPtr: number,
  inputLength: number,
  existingDetail: string | undefined,
): string | null {
  if (existingDetail && existingDetail !== "type metadata resolution failed") {
    return null;
  }

  const view = new DataView(memory.buffer);
  let context = safeReadI32(view, outputPtr + SCRATCH_TYPE_METADATA_DEBUG_CONTEXT_OFFSET);
  let constantIndex = safeReadI32(
    view,
    outputPtr + SCRATCH_TYPE_METADATA_DEBUG_SUBJECT_OFFSET,
  );
  let constantType = safeReadI32(view, outputPtr + SCRATCH_TYPE_METADATA_DEBUG_EXTRA_OFFSET);
  let moduleIndex = safeReadI32(view, outputPtr + SCRATCH_MODULE_INDEX_OFFSET);
  if (context !== 300) {
    const lastContext = safeReadI32(view, TYPE_METADATA_DEBUG_LAST_CONTEXT_OFFSET);
    if (lastContext === 300) {
      constantIndex = safeReadI32(view, TYPE_METADATA_DEBUG_LAST_SUBJECT_OFFSET);
      constantType = safeReadI32(view, TYPE_METADATA_DEBUG_LAST_EXTRA_OFFSET);
    } else {
      const inferred = inferConstantMetadataFailure(memory, view, outputPtr, inputLength);
      if (!inferred) {
        return null;
      }
      constantIndex = inferred.constantIndex;
      constantType = inferred.constantType;
      if (inferred.moduleIndex >= 0) {
        moduleIndex = inferred.moduleIndex;
      }
    }
  }
  if (constantIndex < 0) {
    return null;
  }

  let moduleBase = safeReadI32(view, outputPtr + SCRATCH_MODULE_BASE_OFFSET);
  let moduleLen = safeReadI32(view, outputPtr + SCRATCH_MODULE_LEN_OFFSET);

  if (moduleBase <= 0) {
    moduleBase = COMPILER_INPUT_PTR;
  }
  if (moduleLen <= 0) {
    moduleLen = inputLength;
  }
  if (moduleBase < 0 || moduleLen <= 0) {
    return null;
  }

  const bufferLength = memory.buffer.byteLength;
  const maxReadable = bufferLength - moduleBase;
  if (maxReadable <= 0) {
    return null;
  }
  const clampedModuleLen = Math.min(moduleLen, maxReadable);
  if (clampedModuleLen <= 0) {
    return null;
  }

  const astBase = outputPtr + astOutputReserve(clampedModuleLen);
  const constantsCountPtr = astConstantsCountPtr(astBase);
  if (constantsCountPtr < 0 || constantsCountPtr + WORD_SIZE > bufferLength) {
    return null;
  }
  const constantCount = safeReadI32(view, constantsCountPtr);
  if (constantCount <= 0 || constantIndex >= constantCount) {
    return null;
  }

  const entryPtr = constantsCountPtr + WORD_SIZE + constantIndex * AST_CONSTANT_ENTRY_SIZE;
  if (entryPtr < 0 || entryPtr + AST_CONSTANT_ENTRY_SIZE > bufferLength) {
    return null;
  }

  const nameStart = safeReadI32(view, entryPtr + AST_CONSTANT_ENTRY_NAME_OFFSET);
  const nameLength = safeReadI32(view, entryPtr + AST_CONSTANT_ENTRY_NAME_LEN_OFFSET);
  if (nameStart < 0 || nameLength <= 0) {
    return null;
  }

  const sourceBytes = new Uint8Array(memory.buffer, moduleBase, clampedModuleLen);
  const sourceText = decoder.decode(sourceBytes);
  const nameText = sliceByBounds(sourceText, nameStart, nameLength);
  if (nameText.length === 0) {
    return null;
  }

  const locationOffset = Math.min(Math.max(nameStart, 0), sourceText.length);
  const position = computeLineAndColumn(sourceText, locationOffset);
  if (position.line <= 0 || position.column <= 0) {
    return null;
  }

  const path = resolveModulePath(memory, moduleIndex) ?? DEFAULT_ENTRY_MODULE_PATH;
  return `${path}:${position.line}:${position.column}: const initializer type metadata resolution failed for '${nameText}'`;
}

function inferConstantMetadataFailure(
  memory: WebAssembly.Memory,
  view: DataView,
  outputPtr: number,
  inputLength: number,
): { constantIndex: number; constantType: number; moduleIndex: number } | null {
  let effectiveLength = inputLength;
  if (effectiveLength <= 0) {
    const scratchLen = safeReadI32(view, outputPtr + SCRATCH_MODULE_LEN_OFFSET);
    if (scratchLen > 0) {
      effectiveLength = scratchLen;
    }
  }
  const astBase = outputPtr + astOutputReserve(effectiveLength > 0 ? effectiveLength : 0);
  const constantsCountPtr = astConstantsCountPtr(astBase);
  if (constantsCountPtr < 0 || constantsCountPtr + WORD_SIZE > memory.buffer.byteLength) {
    return null;
  }
  const constantCount = safeReadI32(view, constantsCountPtr);
  if (constantCount <= 0) {
    return null;
  }
  const firstEntry = constantsCountPtr + WORD_SIZE;
  const lastEntry = firstEntry + constantCount * AST_CONSTANT_ENTRY_SIZE;
  if (lastEntry > memory.buffer.byteLength) {
    return null;
  }
  for (let index = 0; index < constantCount; index += 1) {
    const entry = firstEntry + index * AST_CONSTANT_ENTRY_SIZE;
    const evalState = safeReadI32(view, entry + AST_CONSTANT_ENTRY_EVAL_STATE_OFFSET);
    if (evalState === AST_CONSTANT_EVAL_STATE_EVALUATED) {
      continue;
    }
    const typeId = safeReadI32(view, entry + AST_CONSTANT_ENTRY_TYPE_OFFSET);
    const moduleIndex = safeReadI32(view, entry + AST_CONSTANT_ENTRY_MODULE_INDEX_OFFSET);
    return { constantIndex: index, constantType: typeId, moduleIndex };
  }
  return null;
}

function astOutputReserve(inputLength: number): number {
  const afterOutput = inputLength + SCRATCH_INSTR_CAPACITY;
  const scratchEnd = SCRATCH_FN_BASE_OFFSET + 16_384;
  return afterOutput > scratchEnd ? afterOutput : scratchEnd;
}

function astConstantsCountPtr(astBase: number): number {
  return astCallDataBase(astBase) + AST_CALL_DATA_CAPACITY * WORD_SIZE;
}

function astCallDataBase(astBase: number): number {
  return astCallDataLenPtr(astBase) + WORD_SIZE;
}

function astCallDataLenPtr(astBase: number): number {
  return astNamesBase(astBase) + AST_NAMES_CAPACITY;
}

function astNamesBase(astBase: number): number {
  return astNamesLenPtr(astBase) + WORD_SIZE;
}

function astNamesLenPtr(astBase: number): number {
  return astBase + WORD_SIZE + AST_MAX_FUNCTIONS * AST_FUNCTION_ENTRY_SIZE;
}

function resolveModulePath(memory: WebAssembly.Memory, moduleIndex: number): string | null {
  if (moduleIndex < 0) {
    return null;
  }
  const entryBase = MODULE_STATE_BASE + MODULE_TABLE_OFFSET + moduleIndex * MODULE_ENTRY_SIZE;
  const view = new DataView(memory.buffer);
  const pathPtr = safeReadI32(view, entryBase + MODULE_ENTRY_PATH_PTR_FIELD * WORD_SIZE);
  const pathLen = safeReadI32(view, entryBase + MODULE_ENTRY_PATH_LEN_FIELD * WORD_SIZE);
  if (pathPtr <= 0 || pathLen <= 0 || pathPtr + pathLen > memory.buffer.byteLength) {
    return null;
  }
  try {
    const bytes = new Uint8Array(memory.buffer, pathPtr, pathLen);
    return decoder.decode(bytes);
  } catch {
    return null;
  }
}

function sliceByBounds(text: string, start: number, length: number): string {
  if (start < 0 || length <= 0) {
    return "";
  }
  const clampedStart = Math.min(start, text.length);
  const clampedEnd = Math.min(clampedStart + length, text.length);
  return text.slice(clampedStart, clampedEnd).trim();
}

function computeLineAndColumn(
  text: string,
  offset: number,
): { line: number; column: number } {
  const clampedOffset = Math.max(0, Math.min(offset, text.length));
  let line = 1;
  let column = 1;
  for (let index = 0; index < clampedOffset; index += 1) {
    const char = text.charCodeAt(index);
    if (char === 10) {
      line += 1;
      column = 1;
    } else if (char !== 13) {
      column += 1;
    }
  }
  return { line, column };
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
