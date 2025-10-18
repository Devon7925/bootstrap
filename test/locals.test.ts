import { expect, test } from "bun:test";

import {
  compileWithAstCompiler,
  expectCompileFailure,
  runWasmMainWithGc,
} from "./helpers";

test("locals are scoped to blocks", async () => {
  const wasm = await compileWithAstCompiler(`
    fn main() -> i32 {
        let outer: i32 = 5;
        {
            let inner: i32 = outer + 10;
            inner
        } + outer
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(20);
});

test("locals can be shadowed in nested blocks", async () => {
  const wasm = await compileWithAstCompiler(`
    fn main() -> i32 {
        let value: i32 = 5;
        {
            let value: i32 = value + 1;
            value
        }
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(6);
});

test("using out of scope locals is rejected", async () => {
  const error = await expectCompileFailure(`
    fn use_out_of_scope() -> i32 {
        {
            let inner: i32 = 5;
            inner
        };
        inner
    }
  `);
  expect(error.failure.detail).toBe("identifier not found");
});

test("assignment to immutable locals is rejected", async () => {
  const error = await expectCompileFailure(`
    fn mutate_immutable() -> i32 {
        let value: i32 = 1;
        value = 2;
        value
    }
  `);
  expect(error.failure.detail).toBe("cannot assign to immutable local");
});

test("blocks must end with an expression", async () => {
  const error = await expectCompileFailure(`
    fn block_without_expression() -> i32 {
        let value: i32 = 1;
    }
  `);
  expect(error.failure.detail).toBe("block must end with an expression");
});
