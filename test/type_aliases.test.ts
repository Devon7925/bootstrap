import { expect, test } from "bun:test";

import {
  compileWithAstCompiler,
  expectCompileFailure,
  runWasmMainWithGc,
} from "./helpers";

test("type aliases can rename builtin types", async () => {
  const wasm = await compileWithAstCompiler(`
        type MyInt = i32;

        fn main() -> i32 {
            let value: MyInt = 41;
            value + 1
        }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(42);
});

test("type aliases can rename array types", async () => {
  const wasm = await compileWithAstCompiler(`
        type MyArray = [i32; 2];

        fn main() -> i32 {
            let value: MyArray = [41, 42];
            value[1] + 1
        }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(43);
});

test("type aliases can be used as array element types", async () => {
  const wasm = await compileWithAstCompiler(`
        type MyInt = i32;

        fn main() -> i32 {
            let value: [MyInt; 2] = [41, 42];
            value[1] + 1
        }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(43);
});

test("type aliases can chain", async () => {
  const wasm = await compileWithAstCompiler(`
        type Base = i32;
        type Wrapper = Base;

        fn add_one(value: Wrapper) -> Wrapper {
            value + 1
        }

        fn main() -> i32 {
            add_one(41)
        }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(42);
});

test.skip("type aliases can chain backwards", async () => {
  const wasm = await compileWithAstCompiler(`
        type Wrapper = Base;
        type Base = i32;

        fn add_one(value: Wrapper) -> Wrapper {
            value + 1
        }

        fn main() -> i32 {
            add_one(41)
        }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(42);
});

test("missing type aliases are rejected", async () => {
  const failure = await expectCompileFailure(`
        fn main() -> Missing {
            0
        }
  `);
  expect(failure.failure.producedLength).toBeLessThanOrEqual(0);
});

test("self type aliases are rejected", async () => {
  const failure = await expectCompileFailure(`
        type Loop = Loop;

        fn main() -> i32 {
            0
        }
  `);
  expect(failure.failure.producedLength).toBeLessThanOrEqual(0);
});

test("cyclic type aliases are rejected", async () => {
  const failure = await expectCompileFailure(`
        type LoopA = LoopB;
        type LoopB = LoopC;
        type LoopC = LoopA;

        fn main() -> i32 {
            0
        }
  `);
  expect(failure.failure.producedLength).toBeLessThanOrEqual(0);
});
