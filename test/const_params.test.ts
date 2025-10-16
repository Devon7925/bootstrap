import { expect, test } from "bun:test";

import {
  compileWithAstCompiler,
  expectCompileFailure,
  instantiateWasmModuleWithGc,
  runWasmMainWithGc,
} from "./helpers";

test("const parameters specialize array repeat lengths", async () => {
  const wasm = await compileWithAstCompiler(`
    fn helper(const COUNT: i32, value: i32) -> i32 {
        len([value; COUNT])
    }

    fn main() -> i32 {
        helper(3, 7)
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(3);
});

test.todo("const parameters specialize array repeat lengths in let", async () => {
  const wasm = await compileWithAstCompiler(`
    fn helper(const COUNT: i32, value: i32) -> i32 {
        let arr = [value; COUNT];
        len(arr)
    }

    fn main() -> i32 {
        helper(3, 7)
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(3);
});

test.todo("const parameters specialize complex array repeat lengths as part of index access expression", async () => {
  const wasm = await compileWithAstCompiler(`
    fn helper(const COUNT: i32, value: i32) -> i32 {
        [value; COUNT][0]
    }

    fn main() -> i32 {
        helper(3, 7)
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(7);
});

test.todo("const parameter templates specialize array arguments", async () => {
  const wasm = await compileWithAstCompiler(`
    fn sum(const N: i32, values: [i32; N]) -> i32 {
        let mut total: i32 = 0;
        let mut index: i32 = 0;
        loop {
            if index >= N {
                return total;
            };
            total = total + values[index];
            index = index + 1;
            0
        }
    }

    fn main() -> i32 {
        let values: [i32; 3] = [4, 5, 6];
        sum(3, values)
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(15);
});

test.todo("const parameter templates specialize return types", async () => {
  const wasm = await compileWithAstCompiler(`
    fn build(const N: i32, value: i32) -> [i32; N] {
        [value; N]
    }

    fn main() -> i32 {
        let values: [i32; 3] = build(3, 5);
        values[0] + values[1] + values[2]
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(15);
});

test.todo("type-valued const parameters specialize signatures", async () => {
  const wasm = await compileWithAstCompiler(`
    fn select(const T: type, flag: bool, on_true: T, on_false: T) -> T {
        if flag { on_true } else { on_false }
    }

    fn main() -> i32 {
        select(i32, true, 40, 2)
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(42);
});

test("const parameters accept literal arguments", async () => {
  const wasm = await compileWithAstCompiler(`
    fn add_count(const COUNT: i32, value: i32) -> i32 {
        value + COUNT
    }

    fn main() -> i32 {
        add_count(5, 37)
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(42);
});

test("const parameters require compile-time constant arguments", async () => {
  const failure = await expectCompileFailure(`
    fn scale(const FACTOR: i32, value: i32) -> i32 {
        value * FACTOR
    }

    fn main() -> i32 {
        let runtime: i32 = 6;
        scale(runtime, 7)
    }
  `);
  expect(failure.failure.producedLength).toBeLessThanOrEqual(0);
});

test("const parameters accept const fn results", async () => {
  const wasm = await compileWithAstCompiler(`
    const fn three() -> i32 {
        3
    }

    fn multiply(const TIMES: i32, value: i32) -> i32 {
        value * TIMES
    }

    fn main() -> i32 {
        multiply(three(), 14)
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(42);
});

test("const parameter templates reject mismatched parameter arrays", async () => {
  const failure = await expectCompileFailure(`
    fn sum(const N: i32, values: [i32; N]) -> i32 {
        let mut total: i32 = 0;
        let mut index: i32 = 0;
        loop {
            if index >= N {
                return total;
            };
            total = total + values[index];
            index = index + 1;
            0
        }
    }

    fn main() -> i32 {
        let values: [i32; 3] = [5, 6, 7];
        sum(4, values)
    }
  `);
  expect(failure.failure.producedLength).toBeLessThanOrEqual(0);
});

test("const parameter return templates reject mismatched bindings", async () => {
  const failure = await expectCompileFailure(`
    fn build(const N: i32, value: i32) -> [i32; N] {
        [value; N]
    }

    fn main() -> i32 {
        let values: [i32; 3] = build(2, 5);
        values[0]
    }
  `);
  expect(failure.failure.producedLength).toBeLessThanOrEqual(0);
});

test("const parameter templates are not exported", async () => {
  const wasm = await compileWithAstCompiler(`
    fn helper(const COUNT: i32, value: i32) -> i32 {
        value + COUNT
    }

    fn main() -> i32 {
        helper(5, 37)
    }
  `);
  const instance = await instantiateWasmModuleWithGc(wasm);
  const exportNames = Object.keys(instance.exports);
  expect(exportNames).toContain("memory");
  expect(exportNames).toContain("main");
  expect(exportNames).not.toContain("helper");
});