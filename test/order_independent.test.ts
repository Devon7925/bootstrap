import { expect, test } from "bun:test";

import {
  compileWithAstCompiler,
  expectCompileFailure,
  runWasmMainWithGc,
} from "./helpers";

test("Functions defined out of order still work", async () => {
  const wasm = await compileWithAstCompiler(`
    fn main() -> i32 {
        util()
    }

    fn util() -> i32 {
        7
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(7);
});

test("Const defined out of order still works", async () => {
  const wasm = await compileWithAstCompiler(`
    fn main() -> i32 {
        UTIL
    }

    const UTIL: i32 = 7;
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(7);
});

test("Const can depend on const out of order", async () => {
  const wasm = await compileWithAstCompiler(`
    const UTIL2: i32 = UTIL1;
    const UTIL1: i32 = 7;

    fn main() -> i32 {
        UTIL
    }

  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(7);
});
