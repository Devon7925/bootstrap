import { expect, test } from "bun:test";

import {
  compileWithAstCompiler,
  expectCompileFailure,
  runWasmMainWithGc,
} from "./helpers";

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

test("unterminated block comments report failures", async () => {
  const failure = await expectCompileFailure(`
    /* comment start
    fn main() -> i32 {
        42
    }
  `);
  expect(failure.failure.detail).toBe(
    "/entry.bp:2:5: unterminated block comment",
  );
});

test("stray block comment terminators report failures", async () => {
  const failure = await expectCompileFailure(`
    fn main() -> i32 {
        */
        42
    }
  `);
  expect(failure.failure.detail).toBe(
    "/entry.bp:3:9: unexpected block comment terminator",
  );
});
