import { expect, test } from "bun:test";

import {
  compileWithAstCompiler,
  expectCompileFailure,
  runWasmMainWithGc,
} from "./helpers";

test("array index reads element", async () => {
  const wasm = await compileWithAstCompiler(`
    fn select(values: [i32; 3], idx: i32) -> i32 {
        values[idx]
    }

    fn main() -> i32 {
        select([7; 3], 1)
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(7);
});

test("array index requires integer indices", async () => {
  const failure = await expectCompileFailure(`
    fn index_with_bool() -> i32 {
        let values: [i32; 2] = [1; 2];
        values[true]
    }
  `);
  expect(failure.failure.detail).toBe("array index requires integer indices");
});

test("len intrinsic returns array length", async () => {
  const wasm = await compileWithAstCompiler(`
    fn main() -> i32 {
        len([7; 3]) + len([1, 2, 3, 4])
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(7);
});

test("array elements can be summed", async () => {
  const wasm = await compileWithAstCompiler(`
    fn sum(values: [i32; 4]) -> i32 {
        let mut total: i32 = 0;
        let mut idx: i32 = 0;
        loop {
            if idx >= len(values) {
                break;
            };
            total = total + values[idx];
            idx = idx + 1;
        }
        total
    }

    fn main() -> i32 {
        sum([1, 2, 3, 4])
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(10);
});
