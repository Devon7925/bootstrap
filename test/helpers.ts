import { readdir } from "node:fs/promises";
import { fileURLToPath } from "node:url";

import {
  compileToWasm,
  CompileError,
  COMPILER_INPUT_PTR,
  FUNCTIONS_BASE_OFFSET,
  FUNCTION_ENTRY_SIZE,
  FUNCTIONS_COUNT_PTR_OFFSET,
  INSTR_OFFSET_PTR_OFFSET,
  STAGE1_MAX_FUNCTIONS,
} from "../src/index";
import type { CompilerModuleSource } from "../src/index";

export type { CompilerModuleSource } from "../src/index";

export { COMPILER_INPUT_PTR } from "../src/index";

export const AST_COMPILER_ENTRY_PATH = "/compiler/ast_compiler.bp";
const AST_COMPILER_DIR_URL = new URL("../compiler/", import.meta.url);

const DEFAULT_INPUT_STRIDE = 256;
export const DEFAULT_OUTPUT_STRIDE = 4_096;
const TYPES_COUNT_PTR_OFFSET = 819_196;
const TYPES_BASE_OFFSET = 819_200;
const TYPE_ENTRY_SIZE = 16;
const MODULE_STATE_BASE = 1_048_576;
const MODULE_STORAGE_TOP_OFFSET = 4;
const MODULE_PATH_PTR = 1_024;
const MODULE_CONTENT_PTR = 4_096;
const DEFAULT_ENTRY_MODULE_PATH = "/entry.bp";
const MEMORY_INTRINSICS_MODULE_PATH = "/stdlib/memory.bp";

const memoryIntrinsicsSourceUrl = new URL("../stdlib/memory.bp", import.meta.url);

const encoder = new TextEncoder();
const decoder = new TextDecoder();

export interface TypeEntry {
  readonly nameStart: number;
  readonly nameLength: number;
  readonly valueStart: number;
  readonly valueLength: number;
}

export interface CompileFailureDetails {
  readonly producedLength: number;
  readonly functions: number;
  readonly instructionOffset: number;
  readonly compiledFunctions: number;
  readonly detail?: string;
}

export interface CompileWithAstCompilerOptions {
  readonly entryPath?: string;
  readonly modules?: ReadonlyArray<CompilerModuleSource>;
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

export class Stage1CompileFailure extends Error {
  readonly failure: CompileFailureDetails;

  constructor(message: string, failure: CompileFailureDetails, options?: ErrorOptions) {
    super(message, options);
    this.failure = failure;
    this.name = "Stage1CompileFailure";
  }
}

interface CompilerExports {
  readonly memory?: WebAssembly.Memory;
  readonly compile?: (inputPtr: number, inputLen: number, outputPtr: number) => number | bigint;
  readonly loadModuleFromSource?: (pathPtr: number, contentPtr: number) => number | bigint;
  readonly compileFromPath?: (pathPtr: number) => number | bigint;
}

export class CompilerInstance {
  #memory: WebAssembly.Memory;
  #compile: (inputPtr: number, inputLen: number, outputPtr: number) => number | bigint;
  #loadModuleFromSource: ((pathPtr: number, contentPtr: number) => number | bigint) | null;
  #compileFromPath: ((pathPtr: number) => number | bigint) | null;
  #memoryIntrinsicsSource: string | null;

  private constructor(
    memory: WebAssembly.Memory,
    compile: (inputPtr: number, inputLen: number, outputPtr: number) => number | bigint,
    loadModuleFromSource: ((pathPtr: number, contentPtr: number) => number | bigint) | undefined,
    compileFromPath: ((pathPtr: number) => number | bigint) | undefined,
    memoryIntrinsicsSource: string | null,
  ) {
    this.#memory = memory;
    this.#compile = compile;
    this.#loadModuleFromSource = loadModuleFromSource ?? null;
    this.#compileFromPath = compileFromPath ?? null;
    this.#memoryIntrinsicsSource = memoryIntrinsicsSource;
  }

  static async create(wasm: Uint8Array): Promise<CompilerInstance> {
    const { instance } = await WebAssembly.instantiate(wasm, {});
    const exports = instance.exports as CompilerExports;
    if (!(exports.memory instanceof WebAssembly.Memory)) {
      throw new CompileError("stage1 compiler must export memory");
    }
    if (typeof exports.compile !== "function") {
      throw new CompileError("stage1 compiler missing compile export");
    }
    const supportsModules =
      typeof exports.loadModuleFromSource === "function" && typeof exports.compileFromPath === "function";
    const memoryIntrinsicsSource = supportsModules ? await loadMemoryIntrinsicsSource() : null;

    return new CompilerInstance(
      exports.memory,
      exports.compile,
      typeof exports.loadModuleFromSource === "function" ? exports.loadModuleFromSource : undefined,
      typeof exports.compileFromPath === "function" ? exports.compileFromPath : undefined,
      memoryIntrinsicsSource,
    );
  }

  get memory(): WebAssembly.Memory {
    return this.#memory;
  }

  compileAt(inputPtr: number, outputPtr: number, source: string): Uint8Array {
    if (this.#loadModuleFromSource && this.#compileFromPath) {
      return this.#compileUsingModules(DEFAULT_ENTRY_MODULE_PATH, source, []);
    }

    const sourceBytes = encoder.encode(source);
    let view = new Uint8Array(this.#memory.buffer);
    if (inputPtr + sourceBytes.length > view.length) {
      const failure = this.#readFailure(outputPtr, -1);
      throw new Stage1CompileFailure(
        "stage1 compiler memory layout does not leave space for input buffer",
        failure,
      );
    }

    const reserved = FUNCTIONS_BASE_OFFSET + STAGE1_MAX_FUNCTIONS * FUNCTION_ENTRY_SIZE;
    if (outputPtr + reserved > view.length) {
      const failure = this.#readFailure(outputPtr, -1);
      throw new Stage1CompileFailure(
        "stage1 compiler memory layout does not leave space for output buffer",
        failure,
      );
    }

    view.set(sourceBytes, inputPtr);

    let producedLength: number;
    try {
      const result = this.#compile(inputPtr, sourceBytes.length, outputPtr);
      producedLength = coerceToI32(result);
    } catch (cause) {
      throw this.#failure(outputPtr, -1, cause);
    }

    view = new Uint8Array(this.#memory.buffer);

    if (!Number.isFinite(producedLength)) {
      throw this.#failure(outputPtr, -1);
    }

    if (producedLength <= 0) {
      throw this.#failure(outputPtr, producedLength);
    }

    return view.slice(outputPtr, outputPtr + producedLength);
  }

  compileWithStride(
    inputPtr: number,
    outputPtr: number,
    inputStride: number,
    outputStride: number,
    source: string,
  ): Uint8Array {
    const wasm = this.compileAt(inputPtr, outputPtr, source);
    const nextInput = inputPtr + inputStride;
    const nextOutput = outputPtr + outputStride;
    if (nextInput > this.#memory.buffer.byteLength || nextOutput > this.#memory.buffer.byteLength) {
      // Growing the cursors beyond the memory is unexpected but mirror the Rust harness behaviour.
      throw this.#failure(outputPtr, -1);
    }
    return wasm;
  }

  compileWithLayout(inputPtr: number, outputPtr: number, source: string): Uint8Array {
    return this.compileWithStride(inputPtr, outputPtr, DEFAULT_INPUT_STRIDE, DEFAULT_OUTPUT_STRIDE, source);
  }

  compileModule(entryPath: string, source: string, modules: ReadonlyArray<CompilerModuleSource>): Uint8Array {
    if (!this.#loadModuleFromSource || !this.#compileFromPath) {
      throw new Error("stage1 compiler missing module loading exports");
    }

    return this.#compileUsingModules(entryPath, source, modules);
  }

  readTypesCount(outputPtr: number): number {
    const view = new DataView(this.#memory.buffer);
    return safeReadI32(view, outputPtr + TYPES_COUNT_PTR_OFFSET);
  }

  readTypeEntry(outputPtr: number, index: number): TypeEntry {
    const entryPtr = outputPtr + TYPES_BASE_OFFSET + index * TYPE_ENTRY_SIZE;
    const view = new DataView(this.#memory.buffer, entryPtr, TYPE_ENTRY_SIZE);
    return {
      nameStart: view.getInt32(0, true),
      nameLength: view.getInt32(4, true),
      valueStart: view.getInt32(8, true),
      valueLength: view.getInt32(12, true),
    };
  }

  #failure(outputPtr: number, producedLength: number, cause?: unknown): Stage1CompileFailure {
    const failure = this.#readFailure(outputPtr, producedLength);
    const detail = failure.detail ? `, detail=\"${failure.detail}\"` : "";
    return new Stage1CompileFailure(
      `stage1 compilation failed (status ${failure.producedLength}, functions=${failure.functions}, instr_offset=${failure.instructionOffset}, compiled_functions=${failure.compiledFunctions}${detail})`,
      failure,
      cause ? { cause } : undefined,
    );
  }

  #compileUsingModules(
    entryPath: string,
    source: string,
    extraModules: ReadonlyArray<CompilerModuleSource>,
  ): Uint8Array {
    if (!this.#loadModuleFromSource || !this.#compileFromPath) {
      throw new Error("stage1 compiler missing module loading exports");
    }
    if (!this.#memoryIntrinsicsSource) {
      throw new Error("memory intrinsics source not loaded");
    }

    const modules: CompilerModuleSource[] = [
      { path: MEMORY_INTRINSICS_MODULE_PATH, source: this.#memoryIntrinsicsSource },
      ...extraModules.filter((module) => module.path !== MEMORY_INTRINSICS_MODULE_PATH),
    ];

    const loadModule = this.#loadModuleFromSource;
    const compileFromPath = this.#compileFromPath;

    for (const module of modules) {
      let contentLength: number;
      try {
        writeModuleString(this.#memory, MODULE_PATH_PTR, module.path);
        contentLength = writeModuleString(this.#memory, MODULE_CONTENT_PTR, module.source);
      } catch (cause) {
        throw this.#failure(readModuleStorageTop(this.#memory), -1, cause);
      }

      let loadResult: number | bigint;
      try {
        loadResult = loadModule(MODULE_PATH_PTR, MODULE_CONTENT_PTR);
      } catch (cause) {
        throw this.#failure(readModuleStorageTop(this.#memory), -1, cause);
      }

      const status = coerceToI32(loadResult);
      if (!Number.isFinite(status)) {
        throw this.#failure(readModuleStorageTop(this.#memory), -1);
      }
      if (status < 0) {
        throw this.#failure(readModuleStorageTop(this.#memory), status);
      }

      try {
        zeroModuleMemory(this.#memory, MODULE_CONTENT_PTR, contentLength + 1);
      } catch (cause) {
        throw this.#failure(readModuleStorageTop(this.#memory), -1, cause);
      }
    }

    let entryContentLength: number;
    try {
      writeModuleString(this.#memory, MODULE_PATH_PTR, entryPath);
      entryContentLength = writeModuleString(this.#memory, MODULE_CONTENT_PTR, source);
    } catch (cause) {
      throw this.#failure(readModuleStorageTop(this.#memory), -1, cause);
    }

    let loadEntryResult: number | bigint;
    try {
      loadEntryResult = loadModule(MODULE_PATH_PTR, MODULE_CONTENT_PTR);
    } catch (cause) {
      throw this.#failure(readModuleStorageTop(this.#memory), -1, cause);
    }

    const entryStatus = coerceToI32(loadEntryResult);
    if (!Number.isFinite(entryStatus)) {
      throw this.#failure(readModuleStorageTop(this.#memory), -1);
    }
    if (entryStatus < 0) {
      throw this.#failure(readModuleStorageTop(this.#memory), entryStatus);
    }

    try {
      zeroModuleMemory(this.#memory, MODULE_CONTENT_PTR, entryContentLength + 1);
    } catch (cause) {
      throw this.#failure(readModuleStorageTop(this.#memory), -1, cause);
    }

    let producedLength: number;
    try {
      const result = compileFromPath(MODULE_PATH_PTR);
      producedLength = coerceToI32(result);
    } catch (cause) {
      throw this.#failure(readModuleStorageTop(this.#memory), -1, cause);
    }

    const outputPtr = readModuleStorageTop(this.#memory);

    if (!Number.isFinite(producedLength)) {
      throw this.#failure(outputPtr, -1);
    }

    if (producedLength <= 0) {
      throw this.#failure(outputPtr, producedLength);
    }

    const view = new Uint8Array(this.#memory.buffer);
    return view.slice(outputPtr, outputPtr + producedLength);
  }

  #readFailure(outputPtr: number, producedLength: number): CompileFailureDetails {
    return describeCompilationFailure(this.#memory, outputPtr, producedLength);
  }
}

let memoryIntrinsicsSourcePromise: Promise<string> | null = null;
let astCompilerModuleSourcesPromise: Promise<CompilerModuleSource[]> | null = null;
let astCompilerSourcePromise: Promise<string> | null = null;
let astCompilerWasmPromise: Promise<Uint8Array> | null = null;

async function loadMemoryIntrinsicsSource(): Promise<string> {
  if (!memoryIntrinsicsSourcePromise) {
    memoryIntrinsicsSourcePromise = Bun.file(memoryIntrinsicsSourceUrl).text();
  }
  return memoryIntrinsicsSourcePromise;
}

async function loadAstCompilerModuleSources(): Promise<CompilerModuleSource[]> {
  if (!astCompilerModuleSourcesPromise) {
    astCompilerModuleSourcesPromise = (async () => {
      const directoryPath = fileURLToPath(AST_COMPILER_DIR_URL);
      const entries = await readdir(directoryPath, { withFileTypes: true });
      const modules: CompilerModuleSource[] = [];
      for (const entry of entries) {
        if (!entry.isFile()) {
          continue;
        }
        if (!entry.name.endsWith(".bp")) {
          continue;
        }
        const fileUrl = new URL(entry.name, AST_COMPILER_DIR_URL);
        const source = await Bun.file(fileUrl).text();
        modules.push({ path: `/compiler/${entry.name}`, source });
      }
      modules.sort((a, b) => a.path.localeCompare(b.path));
      return modules;
    })();
  }
  return astCompilerModuleSourcesPromise;
}

async function loadAstCompilerSource(): Promise<string> {
  if (!astCompilerSourcePromise) {
    astCompilerSourcePromise = (async () => {
      const modules = await loadAstCompilerModuleSources();
      const entry = modules.find((module) => module.path === AST_COMPILER_ENTRY_PATH);
      if (!entry) {
        throw new Error("ast compiler entry module not found");
      }
      return entry.source;
    })();
  }
  return astCompilerSourcePromise;
}

export async function loadAstCompilerWasm(): Promise<Uint8Array> {
  if (!astCompilerWasmPromise) {
    astCompilerWasmPromise = (async () => {
      const modules = await loadAstCompilerModuleSources();
      const entry = modules.find((module) => module.path === AST_COMPILER_ENTRY_PATH);
      if (!entry) {
        throw new Error("ast compiler entry module not found");
      }
      const extraModules = modules.filter((module) => module.path !== AST_COMPILER_ENTRY_PATH);
      const wasm = await compileToWasm(entry.source, {
        entryPath: AST_COMPILER_ENTRY_PATH,
        modules: extraModules,
      });
      return wasm;
    })();
  }
  return astCompilerWasmPromise;
}

export async function readAstCompilerSource(): Promise<string> {
  return loadAstCompilerSource();
}

export async function readAstCompilerModules(): Promise<CompilerModuleSource[]> {
  const modules = await loadAstCompilerModuleSources();
  return modules.map((module) => ({ ...module }));
}

export async function instantiateAstCompiler(): Promise<CompilerInstance> {
  const wasm = await loadAstCompilerWasm();
  return CompilerInstance.create(wasm);
}

export async function tryCompileWithAstCompiler(
  source: string,
  options: CompileWithAstCompilerOptions = {},
): Promise<Uint8Array> {
  const wasm = await loadAstCompilerWasm();
  const compiler = await CompilerInstance.create(wasm);
  const modules = options.modules ?? [];
  if (modules.length > 0) {
    const entryPath = options.entryPath ?? "/tests/main.bp";
    return compiler.compileModule(entryPath, source, modules);
  }
  const inputPtr = COMPILER_INPUT_PTR;
  const outputPtr = DEFAULT_OUTPUT_STRIDE;
  return compiler.compileWithLayout(inputPtr, outputPtr, source);
}

export async function compileWithAstCompiler(
  source: string,
  options?: CompileWithAstCompilerOptions,
): Promise<Uint8Array> {
  try {
    return await tryCompileWithAstCompiler(source, options);
  } catch (error) {
    if (error instanceof Stage1CompileFailure) {
      throw new Error(`ast compiler failed to compile source: ${error.message}`, { cause: error });
    }
    throw error;
  }
}

export async function expectCompileFailure(
  source: string,
  options?: CompileWithAstCompilerOptions,
): Promise<Stage1CompileFailure> {
  try {
    await tryCompileWithAstCompiler(source, options);
  } catch (error) {
    if (error instanceof Stage1CompileFailure) {
      if (!error.failure.detail || error.failure.detail.length === 0) {
        throw new Error(`stage1 compilation failed without diagnostic detail`);
      }
      return error;
    }
    throw error;
  }
  throw new Error("expected stage1 compilation to fail");
}

export async function instantiateWasmModuleWithGc(wasm: Uint8Array): Promise<WebAssembly.Instance> {
  const { instance } = await WebAssembly.instantiate(wasm, {});
  return instance;
}

export function expectExportedFunction(
  instance: WebAssembly.Instance,
  name: string,
): (...args: Array<number | bigint>) => number {
  const value = (instance.exports as Record<string, unknown>)[name];
  if (typeof value !== "function") {
    throw new Error(`compiled module should export function '${name}'`);
  }

  return (...args: Array<number | bigint>) => {
    const result = (value as (...args: Array<number | bigint>) => unknown)(...args);
    if (typeof result === "number") {
      return Number(result);
    }
    if (typeof result === "bigint") {
      return Number(result);
    }
    if (result === undefined) {
      throw new Error(`exported function '${name}' returned no value`);
    }
    throw new Error(`exported function '${name}' returned unsupported value type ${typeof result}`);
  };
}

export function expectExportedMemory(
  instance: WebAssembly.Instance,
  name = "memory",
): WebAssembly.Memory {
  const value = (instance.exports as Record<string, unknown>)[name];
  if (!(value instanceof WebAssembly.Memory)) {
    throw new Error(`compiled module should export memory '${name}'`);
  }
  return value;
}

export async function runWasmMainWithGc(wasm: Uint8Array): Promise<number> {
  const instance = await instantiateWasmModuleWithGc(wasm);
  const main = expectExportedFunction(instance, "main");
  return main();
}

function describeCompilationFailure(
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
  const end = Math.min(outputPtr + 256, memory.buffer.byteLength);
  if (end > start) {
    const detailBytes = new Uint8Array(memory.buffer.slice(start, end));
    const zeroIndex = detailBytes.indexOf(0);
    const slice = zeroIndex >= 0 ? detailBytes.subarray(0, zeroIndex) : detailBytes;
    const text = decoder.decode(slice).trim();
    if (text.length > 0) {
      detail = text;
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
