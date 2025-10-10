import { expect, test } from "bun:test";

import {
  compileWithAstCompiler,
  expectCompileFailure,
  runWasmMainWithGc,
} from "./helpers";

function containsSequence(haystack: Uint8Array, needle: readonly number[]): boolean {
  if (needle.length === 0) {
    return true;
  }
  outer: for (let index = 0; index <= haystack.length - needle.length; index += 1) {
    for (let offset = 0; offset < needle.length; offset += 1) {
      if (haystack[index + offset] !== (needle[offset] & 0xff)) {
        continue outer;
      }
    }
    return true;
  }
  return false;
}

test("tuple literal emits struct.new", async () => {
  const wasm = await compileWithAstCompiler(`
    fn build() -> (i32, i32) {
        (1, 2)
    }

    fn main() -> i32 {
        0
    }
  `);

  const pattern = [0x41, 0x01, 0x41, 0x02, 0xfb, 0x00, 0x00];
  expect(containsSequence(wasm, pattern)).toBe(true);
});

test("tuple literal can be passed to function arguments", async () => {
  const wasm = await compileWithAstCompiler(`
    fn take(arg: (i32, bool)) -> i32 {
        0
    }

    fn main() -> i32 {
        take((7, true))
    }
  `);

  let found = false;
  for (let callIndex = 0; callIndex <= 10; callIndex += 1) {
    const pattern = [0x41, 0x07, 0x41, 0x01, 0xfb, 0x00, 0x00, 0x10, callIndex];
    if (containsSequence(wasm, pattern)) {
      found = true;
      break;
    }
  }

  expect(found).toBe(true);

  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(0);
});

test("tuple field access emits struct.get", async () => {
  const wasm = await compileWithAstCompiler(`
    fn first(pair: (i32, bool)) -> i32 {
        pair.0
    }

    fn main() -> i32 {
        first((42, true))
    }
  `);

  expect(containsSequence(wasm, [0xfb, 0x02])).toBe(true);

  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(42);
});

test("tuple field indices must exist", async () => {
  const failure = await expectCompileFailure(`
    fn main() -> i32 {
        let pair: (i32, bool) = (1, true);
        pair.2;
        0
    }
  `);

  expect(failure.failure.producedLength).toBeLessThanOrEqual(0);
});

test("tuple fields can be chained", async () => {
  const wasm = await compileWithAstCompiler(`
    fn main() -> i32 {
        let nested: ((i32, i32), i32) = ((5, 7), 9);
        nested.0.1
    }
  `);

  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(7);
});

test("array of tuples can be indexed", async () => {
  const wasm = await compileWithAstCompiler(`
    fn main() -> i32 {
      let arr: [(i32, i32); 2] = [(1, 2), (3, 4)];
      arr[1].0
    }
  `);

  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(3);
});

test("tuple containing array field can be indexed", async () => {
  const wasm = await compileWithAstCompiler(`
    fn main() -> i32 {
      let t: (i32, [i32; 3]) = (5, [10, 20, 30]);
      t.1[2]
    }
  `);

  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(30);
});

test("array of tuples can be passed to function", async () => {
  const wasm = await compileWithAstCompiler(`
    fn sum_first_and_flag(arr: [(i32, bool); 2]) -> i32 {
      let a = arr[0].0;
      let b = if arr[1].1 { 1 } else { 0 };
      a + b
    }

    fn main() -> i32 {
      sum_first_and_flag([(7, false), (0, true)])
    }
  `);

  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(8);
});

test("nested tuple and array fields can be chained", async () => {
  const wasm = await compileWithAstCompiler(`
    fn main() -> i32 {
      let t: ((i32, [i32; 2]), i32) = ((1, [2, 3]), 4);
      t.0.1[1]
    }
  `);

  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(3);
});

