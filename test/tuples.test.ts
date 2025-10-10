import { expect, test } from "bun:test";

import { compileWithAstCompiler, runWasmMainWithGc } from "./helpers";

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

