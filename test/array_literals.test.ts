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

test("array literal emits array.new", async () => {
  const wasm = await compileWithAstCompiler(`
    fn build() -> [i32; 4] {
        [2; 4]
    }

    fn main() -> i32 {
        0
    }
  `);

  const pattern = [0x41, 0x02, 0x41, 0x04, 0xfb, 0x06, 0x00];
  expect(containsSequence(wasm, pattern)).toBe(true);
});

test("array list literal emits array.new_fixed", async () => {
  const wasm = await compileWithAstCompiler(`
    fn build() -> [i32; 4] {
        [1, 2, 3, 4]
    }

    fn main() -> i32 {
        0
    }
  `);

  const pattern = [0x41, 0x01, 0x41, 0x02, 0x41, 0x03, 0x41, 0x04, 0xfb, 0x08, 0x00, 0x04];
  expect(containsSequence(wasm, pattern)).toBe(true);
});

test("array literal uses expression default value", async () => {
  const wasm = await compileWithAstCompiler(`
    fn build(value: i32) -> [i32; 3] {
        [value; 3]
    }

    fn main() -> i32 {
        build(5);
        0
    }
  `);

  let found = false;
  for (let idx = 0; idx <= 10; idx += 1) {
    const pattern = [0x20, idx, 0x41, 0x03, 0xfb, 0x06, 0x00];
    if (containsSequence(wasm, pattern)) {
      found = true;
      break;
    }
  }

  expect(found).toBe(true);
});

test("array literal can be passed to function arguments", async () => {
  const wasm = await compileWithAstCompiler(`
    fn take(arg: [i32; 4]) -> i32 {
        0
    }

    fn main() -> i32 {
        take([7; 4])
    }
  `);

  let found = false;
  for (let callIndex = 0; callIndex <= 10; callIndex += 1) {
    const pattern = [0x41, 0x07, 0x41, 0x04, 0xfb, 0x06, 0x00, 0x10, callIndex];
    if (containsSequence(wasm, pattern)) {
      found = true;
      break;
    }
  }

  expect(found).toBe(true);

  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(0);
});

test("array literal length accepts constant expressions", async () => {
  const wasm = await compileWithAstCompiler(`
    const BASE: i32 = 2;

    const fn compute(value: i32) -> i32 {
        value + 1
    }

    fn main() -> i32 {
        len([5; compute(BASE) * 2])
    }
  `);

  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(6);
});

test("array list literal can be passed to function arguments", async () => {
  const wasm = await compileWithAstCompiler(`
    fn take(arg: [i32; 4]) -> i32 {
        0
    }

    fn main() -> i32 {
        take([1, 2, 3, 4,])
    }
  `);

  let found = false;
  for (let callIndex = 0; callIndex <= 10; callIndex += 1) {
    const pattern = [
      0x41,
      0x01,
      0x41,
      0x02,
      0x41,
      0x03,
      0x41,
      0x04,
      0xfb,
      0x08,
      0x00,
      0x04,
      0x10,
      callIndex,
    ];
    if (containsSequence(wasm, pattern)) {
      found = true;
      break;
    }
  }

  expect(found).toBe(true);

  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(0);
});

test("array literal rejects negative length", async () => {
  const failure = await expectCompileFailure(`
    fn build() -> [i32; 4] {
        [2; -1]
    }
  `);

  expect(failure.failure.detail).toBe(
    "/entry.bp:3:13: array literal length must be non-negative",
  );
});

test("array literal rejects constant expressions with negative length", async () => {
  const failure = await expectCompileFailure(`
    const SHIFT: i32 = 5;

    fn invalid_length() -> i32 {
        len([1; 3 - SHIFT])
    }
  `);

  expect(failure.failure.detail).toBe(
    "/entry.bp:5:19: array literal length must be non-negative",
  );
});

test("array literal length must match declared type", async () => {
  const failure = await expectCompileFailure(`
    fn build() -> [i32; 4] {
        [2; 3]
    }
  `);

  expect(failure.failure.detail).toBe(
    "/entry.bp:3:9: array expression type does not match expected array type",
  );
});

test("array list literal length must match declared type", async () => {
  const failure = await expectCompileFailure(`
    fn build() -> [i32; 4] {
        [1, 2, 3]
    }
  `);

  expect(failure.failure.detail).toBe(
    "/entry.bp:3:9: array expression type does not match expected array type",
  );
});

test("array local initialization enforces literal length", async () => {
  const failure = await expectCompileFailure(`
    fn main() {
        let values: [i32; 3] = [1, 2];
    }
  `);

  expect(failure.failure.detail).toBe(
    "/entry.bp:3:32: array expression type does not match expected array type",
  );
});

test("array locals reject extra literal elements", async () => {
  const failure = await expectCompileFailure(`
    fn main() {
        let values: [i32; 2] = [1, 2, 3];
    }
  `);

  expect(failure.failure.detail).toBe(
    "/entry.bp:3:32: array expression type does not match expected array type",
  );
});

test("array list literal requires uniform element types", async () => {
  const failure = await expectCompileFailure(`
    fn build() -> [i32; 3] {
        [1, true, 3]
    }
  `);

  expect(failure.failure.detail).toBe(
    "/entry.bp:3:9: array literal elements must have uniform type",
  );
});

test("array arguments passed from local variable", async () => {
    const wasm = await compileWithAstCompiler(`
    fn take(values: [i32; 3]) -> i32 {
        values[0]
    }

    fn main() -> i32 {
        let values: [i32; 3] = [4, 5, 6];
        take(values)
    }
  `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(4);
});

