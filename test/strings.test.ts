import { expect, test } from "bun:test";

import { compileWithAstCompiler, runWasmMainWithGc } from "./helpers";

function containsSequence(haystack: Uint8Array, needle: Uint8Array): boolean {
  if (needle.length === 0) {
    return true;
  }
  outer: for (let i = 0; i <= haystack.length - needle.length; i += 1) {
    for (let j = 0; j < needle.length; j += 1) {
      if (haystack[i + j] !== (needle[j] & 0xff)) {
        continue outer;
      }
    }
    return true;
  }
  return false;
}

test("string literal emits array.new_fixed", async () => {
  const wasm = await compileWithAstCompiler(`
    fn build() -> [u8; 5] {
        "hello"
    }

    fn main() -> i32 {
        0
    }
  `);

  const pattern = new Uint8Array([
    0x41, 0xe8, 0x00, 0x41, 0xe5, 0x00, 0x41, 0xec, 0x00, 0x41, 0xec, 0x00, 0x41, 0xef, 0x00, 0xfb,
    0x08, 0x00, 0x05,
  ]);
  expect(containsSequence(wasm, pattern)).toBe(true);
});

test("string literals support escape sequences", async () => {
  const wasm = await compileWithAstCompiler(`
    fn newline_tab() -> [u8; 2] {
        "\\n\\t"
    }

    fn slash_quote() -> [u8; 2] {
        "\\\\\\\""
    }

    fn main() -> i32 {
        0
    }
  `);

  const newlineTabPattern = new Uint8Array([0x41, 0x0a, 0x41, 0x09, 0xfb, 0x08, 0x00, 0x02]);
  const slashQuotePattern = new Uint8Array([0x41, 0xdc, 0x00, 0x41, 0x22, 0xfb, 0x08, 0x00, 0x02]);

  expect(containsSequence(wasm, newlineTabPattern)).toBe(true);
  expect(containsSequence(wasm, slashQuotePattern)).toBe(true);
});

test("empty string literal has zero length", async () => {
  const wasm = await compileWithAstCompiler(`
    fn main() -> i32 {
        len("")
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(0);
});

test("assign to array local", async () => {
  const wasm = await compileWithAstCompiler(`
    fn main() -> i32 {
        let test: [u8; 4] = "test";
        len(test)
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(4);
});

test("string indices match char casts", async () => {
  const wasm = await compileWithAstCompiler(`
    fn main() -> i32 {
        let word: [u8; 5] = "hello";
        let mut score: i32 = 0;

        if word[0] == ('h' as u8) {
            score = score + 1;
            0
        } else {
            0
        };

        if word[1] == ('e' as u8) {
            score = score + 1;
            0
        } else {
            0
        };

        if word[2] == ('l' as u8) {
            score = score + 1;
            0
        } else {
            0
        };

        if word[3] == ('l' as u8) {
            score = score + 1;
            0
        } else {
            0
        };

        if word[4] == ('o' as u8) {
            score = score + 1;
            0
        } else {
            0
        };

        score
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(5);
});
