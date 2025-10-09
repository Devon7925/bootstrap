import { expect, test } from "bun:test";

import { compileWithAstCompiler, runWasmMainWithGc } from "./helpers";

test("parenthesized expressions evaluate correctly", async () => {
  const wasm = await compileWithAstCompiler(`
    fn compute() -> i32 {
        let base: i32 = 2;
        (base + 3) * (4 + 1)
    }

    fn bool_gate(flag: bool) -> i32 {
        if (flag && (false || true)) {
            1
        } else {
            0
        }
    }

    fn main() -> i32 {
        (compute() / (3 - 1)) + bool_gate(true)
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(13);
});

test("parenthesized literal executes", async () => {
  const wasm = await compileWithAstCompiler(`
    fn main() -> i32 {
        (42)
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(42);
});

test("nested parentheses in addition execute", async () => {
  const wasm = await compileWithAstCompiler(`
    fn helper() -> i32 {
        10
    }

    fn main() -> i32 {
        (helper()) + (1 + (2 + 3))
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(16);
});

test("parentheses affect multiplication order", async () => {
  const wasm = await compileWithAstCompiler(`
    fn main() -> i32 {
        (2 + 3) * 4
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(20);
});
