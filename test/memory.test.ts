import { expect, test } from "bun:test";

import {
  compileWithAstCompiler,
  expectExportedFunction,
  expectExportedMemory,
  instantiateWasmModuleWithGc,
} from "./helpers";

const MEMORY_INTRINSICS_PATH = "/stdlib/memory.bp";
const memoryIntrinsicsSourceUrl = new URL("../stdlib/memory.bp", import.meta.url);
const memoryIntrinsicsSourcePromise = Bun.file(memoryIntrinsicsSourceUrl).text();

async function compileMemoryProgram(source: string, entryPath: string): Promise<Uint8Array> {
  const memoryIntrinsicsSource = await memoryIntrinsicsSourcePromise;
  return compileWithAstCompiler(source, {
    entryPath,
    modules: [{ path: MEMORY_INTRINSICS_PATH, source: memoryIntrinsicsSource }],
  });
}

test("exports multi-page memory", async () => {
  const wasm = await compileWithAstCompiler(`
    fn slice_len(_ptr: i32, len: i32) -> i32 {
        len
    }

    fn main() -> i32 {
        0
    }
  `);
  const instance = await instantiateWasmModuleWithGc(wasm);
  const memory = expectExportedMemory(instance);
  const sliceLen = expectExportedFunction(instance, "slice_len");

  expect(memory.buffer.byteLength).toBeGreaterThanOrEqual(1_048_576);
  expect(sliceLen(0, 42)).toBe(42);
});

test("reads last byte from input slice", async () => {
  const wasm = await compileMemoryProgram(
    `
    use "/stdlib/memory.bp";

    fn last_byte(ptr: i32, len: i32) -> i32 {
        if len == 0 {
            return -1;
        };

        let last: i32 = len - 1;
        load_u8(ptr + last)
    }

    fn main() -> i32 {
        0
    }
  `,
    "/tests/memory/last_byte.bp",
  );
  const instance = await instantiateWasmModuleWithGc(wasm);
  const memory = expectExportedMemory(instance);
  const lastByte = expectExportedFunction(instance, "last_byte");

  const input = new TextEncoder().encode("bootstrap");
  const offset = 32;
  new Uint8Array(memory.buffer).set(input, offset);

  const result = lastByte(offset, input.length);
  expect(result & 0xff).toBe(input[input.length - 1]);
});

test("writes byte into memory", async () => {
  const wasm = await compileMemoryProgram(
    `
    use "/stdlib/memory.bp";

    fn write_then_read(ptr: i32, value: i32) -> i32 {
        store_u8(ptr, value);
        load_u8(ptr)
    }

    fn main() -> i32 {
        0
    }
  `,
    "/tests/memory/write_then_read.bp",
  );
  const instance = await instantiateWasmModuleWithGc(wasm);
  const memory = expectExportedMemory(instance);
  const writeThenRead = expectExportedFunction(instance, "write_then_read");

  const offset = 128;
  const value = 173;
  expect(writeThenRead(offset, value)).toBe(value & 0xff);

  const view = new Uint8Array(memory.buffer, offset, 1);
  expect(view[0]).toBe(value & 0xff);
});

test("stores and loads word values", async () => {
  const wasm = await compileMemoryProgram(
    `
    use "/stdlib/memory.bp";

    fn roundtrip_i32(ptr: i32, value: i32) -> i32 {
        store_i32(ptr, value);
        load_i32(ptr)
    }

    fn main() -> i32 {
        0
    }
  `,
    "/tests/memory/roundtrip_i32.bp",
  );
  const instance = await instantiateWasmModuleWithGc(wasm);
  const roundtripI32 = expectExportedFunction(instance, "roundtrip_i32");

  expect(roundtripI32(256, 0x7fff_ff12)).toBe(0x7fff_ff12);
});

test("stores and loads halfword values", async () => {
  const wasm = await compileMemoryProgram(
    `
    use "/stdlib/memory.bp";

    fn roundtrip_u16(ptr: i32, value: i32) -> i32 {
        store_u16(ptr, value);
        load_u16(ptr)
    }

    fn main() -> i32 {
        0
    }
  `,
    "/tests/memory/roundtrip_u16.bp",
  );
  const instance = await instantiateWasmModuleWithGc(wasm);
  const memory = expectExportedMemory(instance);
  const roundtripU16 = expectExportedFunction(instance, "roundtrip_u16");

  const offset = 512;
  const value = 0xfe12;
  expect(roundtripU16(offset, value)).toBe(value & 0xffff);

  const view = new Uint8Array(memory.buffer, offset, 2);
  expect(view[0]).toBe(value & 0xff);
  expect(view[1]).toBe((value >> 8) & 0xff);
});
