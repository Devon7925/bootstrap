import { expect, test } from "bun:test";

import { compileWithAstCompiler, runWasmMainWithGc } from "./helpers";

test("trailing commas in params and calls are accepted", async () => {
  const wasm = await compileWithAstCompiler(`
    fn add(
        a: i32,
        b: i32,
    ) -> i32 {
        a + b
    }

    fn main() -> i32 {
        add(1, 2,)
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(3);
});
