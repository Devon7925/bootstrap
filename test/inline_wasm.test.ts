import { expect, test } from "bun:test";

import {
  compileWithAstCompiler,
  expectCompileFailure,
  runWasmMainWithGc,
} from "./helpers";

test("inline_wasm inserts raw instructions", async () => {
  const wasm = await compileWithAstCompiler(`
    fn main() -> i32 {
        inline_wasm([0x41, 0x2a])
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(42);
});

test("inline_wasm supports const elements", async () => {
  const wasm = await compileWithAstCompiler(`
    const OPCODE: u8 = 0x41;
    const VALUE: u8 = 0x2a;

    fn main() -> i32 {
        inline_wasm([OPCODE, VALUE])
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(42);
});

test("inline_wasm requires literal u8 array", async () => {
  const failure = await expectCompileFailure(`
    fn main() -> i32 {
        let value: i32 = 0x2a;
        inline_wasm([value])
    }
  `);
  expect(failure.message).toContain("stage1 compilation failed");
});

test("inline_wasm enforces u8 range", async () => {
  const failure = await expectCompileFailure(`
    fn main() -> i32 {
        inline_wasm([256])
    }
  `);
  expect(failure.message).toContain("stage1 compilation failed");
});
