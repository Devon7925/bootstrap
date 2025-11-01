import { expect, test } from "bun:test";

import { compileWithAstCompiler, runWasmMainWithGc } from "./helpers";

test("block comments are skipped during lexing", async () => {
  const wasm = await compileWithAstCompiler(`
    /* leading comment */
    fn main() -> i32 {
        40 /* trailing comment */ + 2
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(42);
});

test("nested block comments are supported", async () => {
  const wasm = await compileWithAstCompiler(`
    fn main() -> i32 {
        /* level one /* level two */ still level one */
        42
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(42);
});
