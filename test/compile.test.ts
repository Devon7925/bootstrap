import { expect, test } from "bun:test";

import { compileToWasm } from "../src/index";

import { AST_COMPILER_ENTRY_PATH, readAstCompilerModules } from "./helpers";

test("compiles the stage1 compiler to wasm", async () => {
  const modules = await readAstCompilerModules();
  const entry = modules.find((module) => module.path === AST_COMPILER_ENTRY_PATH);
  if (!entry) {
    throw new Error("ast compiler entry module not found");
  }
  const extraModules = modules.filter((module) => module.path !== AST_COMPILER_ENTRY_PATH);
  const wasm = await compileToWasm(entry.source, {
    entryPath: AST_COMPILER_ENTRY_PATH,
    modules: extraModules,
  });
  expect(wasm.byteLength).toBeGreaterThan(0);
});

test("fails when source is empty", async () => {
  await expect(compileToWasm("")).rejects.toThrow(/source must not be empty/);
});
