import { expect, test } from "bun:test";

import {
  compileWithAstCompiler,
  runWasmMainWithGc,
} from "./helpers";

test.todo("array element mutation updates value", async () => {
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

test.todo("array mutation inside loop accumulates", async () => {
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

test.todo("tuple field mutation updates field", async () => {
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

test.todo("tuple containing array can mutate element", async () => {
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

test.todo("array of tuples allows inner field mutation", async () => {
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
