import { expect, test } from "bun:test";

import {
  compileWithAstCompiler,
  expectCompileFailure,
  runWasmMainWithGc,
} from "./helpers";

test("array element mutation updates value", async () => {
  const wasm = await compileWithAstCompiler(`
    fn main() -> i32 {
        let mut values: [i32; 3] = [1, 2, 3];
        values[1] = 9;
        values[1]
    }
  `);

  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(9);
});

test("array mutation inside loop accumulates", async () => {
  const wasm = await compileWithAstCompiler(`
    fn main() -> i32 {
        let mut values: [i32; 4] = [1, 2, 3, 4];
        let mut idx: i32 = 0;
        loop {
            if idx >= 4 { break; };
            values[idx] = values[idx] * 2;
            idx = idx + 1;
            0
        };
        values[0] + values[1] + values[2] + values[3]
    }
  `);

  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(20);
});

test("tuple field mutation updates field", async () => {
  const wasm = await compileWithAstCompiler(`
    fn main() -> i32 {
        let mut pair: (i32, i32) = (5, 10);
        pair.1 = 20;
        pair.0 + pair.1
    }
  `);

  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(25);
});

test("tuple containing array can mutate element", async () => {
  const wasm = await compileWithAstCompiler(`
    fn main() -> i32 {
        let mut pair: (i32, [i32; 3]) = (2, [3, 4, 5]);
        pair.1[2] = 9;
        pair.0 + pair.1[2]
    }
  `);

  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(11);
});

test("array of tuples allows inner field mutation", async () => {
  const wasm = await compileWithAstCompiler(`
    fn main() -> i32 {
        let mut items: [(i32, bool); 2] = [(1, false), (2, true)];
        items[0].1 = true;
        if items[0].1 { 42 } else { 0 }
    }
  `);

  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(42);
});

test("array element assignment reports type mismatch location", async () => {
  const failure = await expectCompileFailure(`
    fn assign(values: [i32; 2]) {
        let mut local: [i32; 2] = values;
        local[0] = true;
    }
  `);

  expect(failure.failure.detail).toBe(
    "/entry.bp:4:15: array element assignment type mismatch",
  );
});

test("tuple field assignment reports type mismatch location", async () => {
  const failure = await expectCompileFailure(`
    fn assign(pair: (i32, bool)) {
        let mut local: (i32, bool) = pair;
        local.1 = 5;
    }
  `);

  expect(failure.failure.detail).toBe(
    "/entry.bp:4:15: tuple field assignment type mismatch",
  );
});
