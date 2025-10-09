import { expect, test } from "bun:test";

import { compileWithAstCompiler, expectCompileFailure, runWasmMainWithGc } from "./helpers";

test("constant main returns literal value", async () => {
  const wasm = await compileWithAstCompiler(`
    fn main() -> i32 {
        42
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(42);
});

test("global constants can be referenced from main", async () => {
  const wasm = await compileWithAstCompiler(`
    const ANSWER: i32 = 42;

    fn main() -> i32 {
        ANSWER
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(42);
});

test("constants can reference other constants", async () => {
  const wasm = await compileWithAstCompiler(`
    const BASE: i32 = 40;
    const VALUE: i32 = BASE;

    fn main() -> i32 {
        VALUE + 2
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(42);
});

test("duplicate constants are rejected", async () => {
  const failure = await expectCompileFailure(`
    const VALUE: i32 = 1;
    const VALUE: i32 = 2;

    fn main() -> i32 {
        VALUE
    }
  `);
  expect(failure.failure.producedLength).toBeLessThanOrEqual(0);
});

test("non literal constant initializers are evaluated", async () => {
  const wasm = await compileWithAstCompiler(`
    const VALUE: i32 = (1 + 2) * 3 - 5;

    fn main() -> i32 {
        VALUE
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(4);
});

test("non-const function calls in constant initializers are rejected", async () => {
  const failure = await expectCompileFailure(`
    const VALUE: i32 = helper();

    fn helper() -> i32 {
        42
    }

    fn main() -> i32 {
        VALUE
    }
  `);
  expect(failure.failure.producedLength).toBeLessThanOrEqual(0);
});

test("const functions can be used in constant initializers", async () => {
  const wasm = await compileWithAstCompiler(`
    const fn add(a: i32, b: i32) -> i32 {
        a + b
    }

    const VALUE: i32 = add(40, 2);

    fn main() -> i32 {
        VALUE
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(42);
});

test("const functions can call other const functions in constant initializers", async () => {
  const wasm = await compileWithAstCompiler(`
    const fn base() -> i32 {
        40
    }

    const fn plus_two(value: i32) -> i32 {
        value + 2
    }

    const VALUE: i32 = plus_two(base());

    fn main() -> i32 {
        VALUE
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(42);
});

test("const functions cannot call non-const functions in constant evaluation", async () => {
  const failure = await expectCompileFailure(`
    const fn call_helper() -> i32 {
        helper()
    }

    fn helper() -> i32 {
        7
    }

    const VALUE: i32 = call_helper();

    fn main() -> i32 {
        VALUE
    }
  `);
  expect(failure.failure.producedLength).toBeLessThanOrEqual(0);
});

test("function names cannot conflict with constants", async () => {
  const failure = await expectCompileFailure(`
    const helper: i32 = 1;

    fn helper() -> i32 {
        0
    }

    fn main() -> i32 {
        helper
    }
  `);
  expect(failure.failure.producedLength).toBeLessThanOrEqual(0);
});

