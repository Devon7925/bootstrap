import { expect, test } from "bun:test";

import {
  compileWithAstCompiler,
  expectCompileFailure,
  expectExportedFunction,
  expectExportedMemory,
  instantiateWasmModuleWithGc,
} from "./helpers";

const MEMORY_INTRINSICS = `
    fn load_u8(ptr: i32) -> u8 {
        let value: i32 = inline_wasm([0x20, 0x00, 0x2d, 0x00, 0x00]);
        value as u8
    }

    fn load_u16(ptr: i32) -> u16 {
        let value: i32 = inline_wasm([0x20, 0x00, 0x2f, 0x01, 0x00]);
        value as u16
    }

    fn store_u8(ptr: i32, value: i32) -> i32 {
        inline_wasm([0x20, 0x00, 0x20, 0x01, 0x3a, 0x00, 0x00, 0x41, 0x00])
    }

    fn store_u16(ptr: i32, value: i32) -> i32 {
        inline_wasm([0x20, 0x00, 0x20, 0x01, 0x3b, 0x01, 0x00, 0x41, 0x00])
    }
`;

test("integer width programs execute", async () => {
  const wasm = await compileWithAstCompiler(`
${MEMORY_INTRINSICS}
    fn add_i8(a: i8, b: i8) -> i8 {
        let mut total: i8 = a;
        total = total + b;
        total
    }

    fn less_than_i8(a: i8, b: i8) -> bool {
        a < b
    }

    fn add_i16(a: i16, b: i16) -> i16 {
        let mut total: i16 = a;
        total = total + b;
        total
    }

    fn less_than_i16(a: i16, b: i16) -> bool {
        a < b
    }

    fn add_i64(a: i64, b: i64) -> i64 {
        a + b
    }

    fn less_than_i64(a: i64, b: i64) -> bool {
        a < b
    }

    fn add_u8(a: u8, b: u8) -> u8 {
        let mut total: u8 = a;
        total = total + b;
        total
    }

    fn max_u8(a: u8, b: u8) -> u8 {
        if a > b { a } else { b }
    }

    fn roundtrip_u8(ptr: i32, value: u8) -> u8 {
        store_u8(ptr, value as i32);
        load_u8(ptr)
    }

    fn add_u16(a: u16, b: u16) -> u16 {
        let mut total: u16 = a;
        total = total + b;
        total
    }

    fn roundtrip_u16(ptr: i32, value: u16) -> u16 {
        store_u16(ptr, value as i32);
        load_u16(ptr)
    }

    fn add_u32(a: u32, b: u32) -> u32 {
        a + b
    }

    fn add_u64(a: u64, b: u64) -> u64 {
        a + b
    }

    fn less_than_u64(a: u64, b: u64) -> bool {
        a < b
    }

    fn mix_call(a: i8, b: i16, c: u32, d: u64) -> u64 {
        let doubled_small: i8 = add_i8(a, a);
        let doubled_mid: i16 = add_i16(b, b);
        let doubled_mid_unsigned: u32 = add_u32(c, c);
        let doubled_large: u64 = add_u64(d, d);
        let mut result: u64 = d;

        if less_than_i16(doubled_mid, b) {
            result = add_u64(d, d);
        } else {
            if doubled_small < a {
                result = doubled_large;
            } else {
                if doubled_mid_unsigned > c {
                    result = doubled_large;
                } else {
                    result = d;
                };
            };
        };

        result
    }

    fn main() -> i32 {
        0
    }
  `);
  const instance = await instantiateWasmModuleWithGc(wasm);
  const memory = expectExportedMemory(instance);

  const addI8 = expectExportedFunction(instance, "add_i8");
  const lessThanI8 = expectExportedFunction(instance, "less_than_i8");
  const addI16 = expectExportedFunction(instance, "add_i16");
  const lessThanI16 = expectExportedFunction(instance, "less_than_i16");
  const addI64 = expectExportedFunction(instance, "add_i64");
  const lessThanI64 = expectExportedFunction(instance, "less_than_i64");
  const addU8 = expectExportedFunction(instance, "add_u8");
  const maxU8 = expectExportedFunction(instance, "max_u8");
  const roundtripU8 = expectExportedFunction(instance, "roundtrip_u8");
  const addU16 = expectExportedFunction(instance, "add_u16");
  const roundtripU16 = expectExportedFunction(instance, "roundtrip_u16");
  const addU32 = expectExportedFunction(instance, "add_u32");
  const addU64 = expectExportedFunction(instance, "add_u64");
  const lessThanU64 = expectExportedFunction(instance, "less_than_u64");
  const mixCall = expectExportedFunction(instance, "mix_call");

  expect(addI8(120, 5)).toBe(125);
  expect(lessThanI8(5, 9)).toBe(1);

  expect(addI16(3000, 1234)).toBe(4234);
  expect(lessThanI16(4000, 1999)).toBe(0);

  expect(addI64(1_000_000_000n, 2_000_000_000n)).toBe(3_000_000_000);
  expect(lessThanI64(9_000_000_000n, 1_000_000_000n)).toBe(0);

  expect(addU8(200, 50)).toBe(250);
  expect(maxU8(17, 42)).toBe(42);

  const u8Offset = 128;
  const u8Value = 0xab;
  expect(roundtripU8(u8Offset, u8Value)).toBe(u8Value);
  const u8View = new Uint8Array(memory.buffer, u8Offset, 1);
  expect(u8View[0]).toBe(u8Value & 0xff);

  expect(addU16(1000, 2300)).toBe(3300);

  const u16Offset = 256;
  const u16Value = 0xbeef;
  expect(roundtripU16(u16Offset, u16Value)).toBe(u16Value & 0xffff);
  const u16View = new Uint8Array(memory.buffer, u16Offset, 2);
  expect(u16View[0]).toBe(u16Value & 0xff);
  expect(u16View[1]).toBe((u16Value >> 8) & 0xff);

  expect(addU32(1_000_000, 2_000_000)).toBe(3_000_000);
  expect(addU64(5n, 7n)).toBe(12);
  expect(lessThanU64(99n, 42n)).toBe(0);
  expect(mixCall(3, 10, 7, 9n)).toBe(18);
});

test("mixed integer widths are rejected without casts", async () => {
  const error = await expectCompileFailure(`
    fn main() -> i32 {
        let lhs: i16 = 12;
        let rhs: i64 = 34;
        if lhs < rhs { 0 } else { 1 }
    }
  `);
  expect(error.failure.producedLength).toBeLessThanOrEqual(0);
});

test("signed and unsigned mixes require casts", async () => {
  const error = await expectCompileFailure(`
    fn difference(a: u16, b: i16) -> u16 {
        a - b
    }

    fn main() -> i32 {
        0
    }
  `);
  expect(error.failure.producedLength).toBeLessThanOrEqual(0);
});

test("integer arguments must match parameter widths", async () => {
  const error = await expectCompileFailure(`
    fn take_i8(value: i8) -> i8 {
        value
    }

    fn main() -> i32 {
        let sample: i16 = 10;
        let _ = take_i8(sample);
        0
    }
  `);
  expect(error.failure.producedLength).toBeLessThanOrEqual(0);
});
