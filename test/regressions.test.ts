import { expect, test } from "bun:test";

import {
  AST_COMPILER_ENTRY_PATH,
  instantiateAstCompiler,
  readAstCompilerModules,
  readExpressionCount,
  readModuleStorageTop,
} from "./helpers";

const encoder = new TextEncoder();

test("stage1 compiler handles expression-heavy programs", async () => {
  const modules = await readAstCompilerModules();
  const entry = modules.find((module) => module.path === AST_COMPILER_ENTRY_PATH);
  if (!entry) {
    throw new Error("ast compiler entry module not found");
  }
  const extraModules = modules.filter((module) => module.path !== AST_COMPILER_ENTRY_PATH);
  const compiler = await instantiateAstCompiler();
  const wasm = await compiler.compileModule(AST_COMPILER_ENTRY_PATH, entry.source, extraModules);
  const sourceLength = encoder.encode(entry.source).length;
  const outputPtr = readModuleStorageTop(compiler.memory);
  const expressionCount = readExpressionCount(compiler.memory, outputPtr, sourceLength);
  expect(expressionCount).toBeGreaterThan(65_536);
});

test("stage1 compiler handles modules with CRLF newlines", async () => {
  const toCRLF = (source: string): string => source.replace(/\r?\n/g, "\r\n");
  const modules = await readAstCompilerModules();
  const entry = modules.find((module) => module.path === AST_COMPILER_ENTRY_PATH);
  if (!entry) {
    throw new Error("ast compiler entry module not found");
  }
  const entrySource = toCRLF(entry.source);
  const extraModules = modules
    .filter((module) => module.path !== AST_COMPILER_ENTRY_PATH)
    .map((module) => ({ path: module.path, source: toCRLF(module.source) }));
  const compiler = await instantiateAstCompiler();
  const wasm = await compiler.compileModule(AST_COMPILER_ENTRY_PATH, entrySource, extraModules);
  expect(wasm.length).toBeGreaterThan(0);
});
