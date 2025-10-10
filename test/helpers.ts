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
import { stubMemoryIntrinsicFunctions } from "../src/bootstrap";

export { COMPILER_INPUT_PTR } from "../src/index";

const AST_COMPILER_SOURCE_PATH = new URL("../compiler/ast_compiler.bp", import.meta.url);

const DEFAULT_INPUT_STRIDE = 256;
export const DEFAULT_OUTPUT_STRIDE = 4_096;
const TYPES_COUNT_PTR_OFFSET = 819_196;
const TYPES_BASE_OFFSET = 819_200;
const TYPE_ENTRY_SIZE = 16;

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
}

export class CompilerInstance {
  #memory: WebAssembly.Memory;
  #compile: (inputPtr: number, inputLen: number, outputPtr: number) => number | bigint;

  private constructor(memory: WebAssembly.Memory, compile: (inputPtr: number, inputLen: number, outputPtr: number) => number | bigint) {
    this.#memory = memory;
    this.#compile = compile;
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
    return new CompilerInstance(exports.memory, exports.compile);
  }

  get memory(): WebAssembly.Memory {
    return this.#memory;
  }

  compileAt(inputPtr: number, outputPtr: number, source: string): Uint8Array {
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
      producedLength = typeof result === "bigint" ? Number(result) : result | 0;
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

  #readFailure(outputPtr: number, producedLength: number): CompileFailureDetails {
    return describeCompilationFailure(this.#memory, outputPtr, producedLength);
  }
}

let astCompilerSourcePromise: Promise<string> | null = null;
let astCompilerWasmPromise: Promise<Uint8Array> | null = null;

async function loadAstCompilerSource(): Promise<string> {
  if (!astCompilerSourcePromise) {
    astCompilerSourcePromise = Bun.file(AST_COMPILER_SOURCE_PATH).text();
  }
  return astCompilerSourcePromise;
}

async function loadAstCompilerWasm(): Promise<Uint8Array> {
  if (!astCompilerWasmPromise) {
    astCompilerWasmPromise = (async () => {
      const source = await loadAstCompilerSource();
      const stubbedSource = stubMemoryIntrinsicFunctions(source);
      const stage1Wasm = await compileToWasm(stubbedSource);
      const stage1Compiler = await CompilerInstance.create(stage1Wasm);
      return stage1Compiler.compileAt(COMPILER_INPUT_PTR, source.length, source);
    })();
  }
  return astCompilerWasmPromise;
}

export async function readAstCompilerSource(): Promise<string> {
  return loadAstCompilerSource();
}

export async function instantiateAstCompiler(): Promise<CompilerInstance> {
  const wasm = await loadAstCompilerWasm();
  return CompilerInstance.create(wasm);
}

export async function tryCompileWithAstCompiler(source: string): Promise<Uint8Array> {
  const wasm = await loadAstCompilerWasm();
  const compiler = await CompilerInstance.create(wasm);
  const inputPtr = COMPILER_INPUT_PTR;
  const outputPtr = DEFAULT_OUTPUT_STRIDE;
  return compiler.compileWithLayout(inputPtr, outputPtr, source);
}

export async function compileWithAstCompiler(source: string): Promise<Uint8Array> {
  try {
    return await tryCompileWithAstCompiler(source);
  } catch (error) {
    if (error instanceof Stage1CompileFailure) {
      throw new Error(`ast compiler failed to compile source: ${error.message}`, { cause: error });
    }
    throw error;
  }
}

export async function expectCompileFailure(source: string): Promise<Stage1CompileFailure> {
  try {
    await tryCompileWithAstCompiler(source);
  } catch (error) {
    if (error instanceof Stage1CompileFailure) {
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
