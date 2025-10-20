import { expect, test } from "bun:test";

import {
  describeCompilationFailure,
  expectExportedFunction,
  expectExportedMemory,
  instantiateWasmModuleWithGc,
  loadAstCompilerWasm,
  readModuleStorageTop,
} from "./helpers";
import type { CompileFailureDetails } from "./helpers";

const encoder = new TextEncoder();

const MODULE_STATE_BASE = 1_048_576;
const MODULE_STORAGE_TOP_OFFSET = 4;

let stage2WasmPromise: Promise<Uint8Array> | undefined;

async function getStage2Wasm(): Promise<Uint8Array> {
  if (!stage2WasmPromise) {
    stage2WasmPromise = loadAstCompilerWasm();
  }
  return stage2WasmPromise;
}

interface Stage2Compiler {
  readonly memory: WebAssembly.Memory;
  readonly loadModuleFromSource: (pathPtr: number, contentPtr: number) => number;
  readonly compileFromPath: (pathPtr: number) => number;
}

async function instantiateStage2Compiler(): Promise<Stage2Compiler> {
  const wasmBytes = await getStage2Wasm();
  const { instance } = await WebAssembly.instantiate(wasmBytes, {});
  const memory = expectExportedMemory(instance);
  const loadModuleFromSource = expectExportedFunction(instance, "loadModuleFromSource");
  const compileFromPath = expectExportedFunction(instance, "compileFromPath");
  return { memory, loadModuleFromSource, compileFromPath };
}

function readCompileFailure(
  compiler: Stage2Compiler,
  producedLength: number,
): CompileFailureDetails {
  const outputPtr = readModuleStorageTop(compiler.memory);
  return describeCompilationFailure(compiler.memory, outputPtr, producedLength);
}

function ensureCapacity(memory: WebAssembly.Memory, required: number) {
  const { buffer } = memory;
  if (buffer.byteLength >= required) {
    return;
  }
  const pageSize = 65_536;
  const additional = required - buffer.byteLength;
  const pagesNeeded = Math.ceil(additional / pageSize);
  memory.grow(pagesNeeded);
}

function writeString(memory: WebAssembly.Memory, ptr: number, text: string): number {
  const bytes = encoder.encode(text);
  ensureCapacity(memory, ptr + bytes.length + 1);
  const view = new Uint8Array(memory.buffer);
  view.set(bytes, ptr);
  view[ptr + bytes.length] = 0;
  return bytes.length;
}

function zeroMemory(memory: WebAssembly.Memory, ptr: number, length: number) {
  if (length <= 0) {
    return;
  }
  ensureCapacity(memory, ptr + length);
  new Uint8Array(memory.buffer).fill(0, ptr, ptr + length);
}

function readOutput(memory: WebAssembly.Memory, producedLength: number): Uint8Array {
  const view = new DataView(memory.buffer);
  const outputPtr = view.getInt32(MODULE_STATE_BASE + MODULE_STORAGE_TOP_OFFSET, true);
  expect(outputPtr).toBeGreaterThan(0);
  expect(producedLength).toBeGreaterThanOrEqual(4);
  const wasmBytes = new Uint8Array(memory.buffer.slice(outputPtr, outputPtr + producedLength));
  expect(Array.from(wasmBytes.subarray(0, 4))).toEqual([0x00, 0x61, 0x73, 0x6d]);
  return wasmBytes;
}

async function loadAndCompile(
  compiler: Stage2Compiler,
  pathPtr: number,
  contentPtr: number,
  source: string,
): Promise<Uint8Array> {
  const contentLength = writeString(compiler.memory, contentPtr, source);
  expect(compiler.loadModuleFromSource(pathPtr, contentPtr)).toBe(0);
  zeroMemory(compiler.memory, contentPtr, contentLength + 1);
  const producedLength = compiler.compileFromPath(pathPtr);
  expect(producedLength).toBeGreaterThan(0);
  return readOutput(compiler.memory, producedLength);
}

async function loadModuleSource(
  compiler: Stage2Compiler,
  pathPtr: number,
  contentPtr: number,
  path: string,
  source: string,
): Promise<void> {
  writeString(compiler.memory, pathPtr, path);
  const contentLength = writeString(compiler.memory, contentPtr, source);
  expect(compiler.loadModuleFromSource(pathPtr, contentPtr)).toBe(0);
  zeroMemory(compiler.memory, contentPtr, contentLength + 1);
}

test("loadModuleFromSource persists content for compileFromPath", async () => {
  const compiler = await instantiateStage2Compiler();
  const pathPtr = 1_024;
  writeString(compiler.memory, pathPtr, "/fixtures/module.bp");
  const contentPtr = 4_096;

  const wasm = await loadAndCompile(compiler, pathPtr, contentPtr, "fn main() -> i32 { 42 }");
  const instance = await instantiateWasmModuleWithGc(wasm);
  const main = expectExportedFunction(instance, "main");
  expect(main()).toBe(42);
});

test("compileFromPath uses the latest module contents", async () => {
  const compiler = await instantiateStage2Compiler();
  const pathPtr = 1_024;
  writeString(compiler.memory, pathPtr, "/fixtures/module.bp");
  const contentPtr = 4_096;

  const wasm1 = await loadAndCompile(compiler, pathPtr, contentPtr, "fn main() -> i32 { 1 }");
  const instance1 = await instantiateWasmModuleWithGc(wasm1);
  const main1 = expectExportedFunction(instance1, "main");
  expect(main1()).toBe(1);

  const wasm2 = await loadAndCompile(compiler, pathPtr, contentPtr, "fn main() -> i32 { 7 }");
  const instance2 = await instantiateWasmModuleWithGc(wasm2);
  const main2 = expectExportedFunction(instance2, "main");
  expect(main2()).toBe(7);
});

test("loadModuleFromSource reports linear memory exhaustion", async () => {
  const compiler = await instantiateStage2Compiler();
  const pathPtr = 1_024;
  const contentPtr = 4_096;
  const chunk = "fn main() -> i32 { 0 }\n";
  const targetLength = 900_000;
  const repeatCount = Math.ceil(targetLength / chunk.length);
  const largeModule = chunk.repeat(repeatCount).slice(0, targetLength);

  let failure: CompileFailureDetails | null = null;
  for (let index = 0; index < 64; index += 1) {
    writeString(compiler.memory, pathPtr, `/fixtures/huge-${index}.bp`);
    writeString(compiler.memory, contentPtr, largeModule);
    const status = compiler.loadModuleFromSource(pathPtr, contentPtr);
    if (status < 0) {
      failure = readCompileFailure(compiler, status);
      break;
    }
    zeroMemory(compiler.memory, contentPtr, largeModule.length + 1);
  }

  expect(failure).not.toBeNull();
  expect(failure?.detail).toBe("failed to reserve linear memory for module storage");
});

test("loadModuleFromSource reports null module path pointer", async () => {
  const compiler = await instantiateStage2Compiler();
  const contentPtr = 4_096;

  writeString(compiler.memory, contentPtr, "fn main() -> i32 { 0 }");

  const status = compiler.loadModuleFromSource(0, contentPtr);
  expect(status).toBeLessThan(0);

  const failure = readCompileFailure(compiler, status);
  expect(failure.detail).toBe("module path missing");
});

test("loadModuleFromSource reports empty module path", async () => {
  const compiler = await instantiateStage2Compiler();
  const pathPtr = 1_024;
  const contentPtr = 4_096;

  writeString(compiler.memory, pathPtr, "");
  writeString(compiler.memory, contentPtr, "fn main() -> i32 { 0 }");

  const status = compiler.loadModuleFromSource(pathPtr, contentPtr);
  expect(status).toBeLessThan(0);

  const failure = readCompileFailure(compiler, status);
  expect(failure.detail).toBe("module path missing");
});

test("loadModuleFromSource reports null module content pointer", async () => {
  const compiler = await instantiateStage2Compiler();
  const pathPtr = 1_024;

  writeString(compiler.memory, pathPtr, "/fixtures/module.bp");

  const status = compiler.loadModuleFromSource(pathPtr, 0);
  expect(status).toBeLessThan(0);

  const failure = readCompileFailure(compiler, status);
  expect(failure.detail).toBe("module content missing");
});

test("compileFromPath reports empty module path", async () => {
  const compiler = await instantiateStage2Compiler();
  const pathPtr = 1_024;

  writeString(compiler.memory, pathPtr, "");

  const producedLength = compiler.compileFromPath(pathPtr);
  expect(producedLength).toBeLessThan(0);

  const failure = readCompileFailure(compiler, producedLength);
  expect(failure.detail).toBe("module path missing");
});

test("compileFromPath reports null module path pointer", async () => {
  const compiler = await instantiateStage2Compiler();

  const producedLength = compiler.compileFromPath(0);
  expect(producedLength).toBeLessThan(0);

  const failure = readCompileFailure(compiler, producedLength);
  expect(failure.detail).toBe("module path missing");
});

test("compileFromPath returns failure for unknown modules", async () => {
  const compiler = await instantiateStage2Compiler();
  const pathPtr = 1_024;
  writeString(compiler.memory, pathPtr, "/fixtures/missing.bp");
  const producedLength = compiler.compileFromPath(pathPtr);
  const failure = readCompileFailure(compiler, producedLength);
  expect(failure.detail).toBe("module has not been loaded");
});

test("compileFromPath resolves use imports relative to module", async () => {
  const compiler = await instantiateStage2Compiler();
  const pathPtr = 1_024;
  const contentPtr = 4_096;

  await loadModuleSource(
    compiler,
    pathPtr,
    contentPtr,
    "/fixtures/lib.bp",
    `
    fn provide() -> i32 { 7 }
  `,
  );

  writeString(compiler.memory, pathPtr, "/fixtures/main.bp");
  const wasm = await loadAndCompile(
    compiler,
    pathPtr,
    contentPtr,
    `
    use "./lib.bp";

    fn main() -> i32 {
        provide()
    }
  `,
  );
  const instance = await instantiateWasmModuleWithGc(wasm);
  const main = expectExportedFunction(instance, "main");
  expect(main()).toBe(7);
});

test("compileFromPath resolves use imports with parent segments", async () => {
  const compiler = await instantiateStage2Compiler();
  const pathPtr = 1_024;
  const contentPtr = 4_096;

  await loadModuleSource(
    compiler,
    pathPtr,
    contentPtr,
    "/fixtures/lib.bp",
    `
    fn provide() -> i32 { 11 }
  `,
  );

  writeString(compiler.memory, pathPtr, "/fixtures/nested/main.bp");
  const wasm = await loadAndCompile(
    compiler,
    pathPtr,
    contentPtr,
    `
    use "../lib.bp";

    fn main() -> i32 {
        provide()
    }
  `,
  );
  const instance = await instantiateWasmModuleWithGc(wasm);
  const main = expectExportedFunction(instance, "main");
  expect(main()).toBe(11);
});

test("compileFromPath fails when use import is missing", async () => {
  const compiler = await instantiateStage2Compiler();
  const pathPtr = 1_024;
  const contentPtr = 4_096;

  writeString(compiler.memory, pathPtr, "/fixtures/main.bp");
  const contentLength = writeString(
    compiler.memory,
    contentPtr,
    `
    use "./missing.bp";

    fn main() -> i32 {
        0
    }
  `,
  );
  expect(compiler.loadModuleFromSource(pathPtr, contentPtr)).toBe(0);
  zeroMemory(compiler.memory, contentPtr, contentLength + 1);

  const producedLength = compiler.compileFromPath(pathPtr);
  const failure = readCompileFailure(compiler, producedLength);
  expect(failure.detail).toBe(
    "/fixtures/main.bp:2:9: module import not found",
  );
});
