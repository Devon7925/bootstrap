import { expect, test } from "bun:test";

import { compileToWasm } from "../src/index";

const COMPILER_SOURCE_PATH = new URL("../compiler/ast_compiler.bp", import.meta.url);

function loadFixture(path: URL) {
  return Bun.file(path).text();
}

test("compiles the stage1 compiler to wasm", async () => {
  const source = await loadFixture(COMPILER_SOURCE_PATH);
  const wasm = await compileToWasm(source);
  expect(wasm.byteLength).toBeGreaterThan(0);
});

test("fails when source is empty", async () => {
  await expect(compileToWasm("")).rejects.toThrow(/source must not be empty/);
});
