import { expect, test } from "bun:test";

import {
  COMPILER_INPUT_PTR,
  CompilerInstance,
  instantiateAstCompiler,
  readAstCompilerModules,
  readAstCompilerSource,
  runWasmMainWithGc,
  AST_COMPILER_ENTRY_PATH,
} from "./helpers";

test("ast compiler bootstraps itself", async () => {
  const compiler = await instantiateAstCompiler();
  const source = await readAstCompilerSource();
  const modules = await readAstCompilerModules();
  const entry = modules.find((module) => module.path === AST_COMPILER_ENTRY_PATH);
  if (!entry) {
    throw new Error("ast compiler entry module not found");
  }
  const extraModules = modules.filter((module) => module.path !== AST_COMPILER_ENTRY_PATH);

  const stage2 = compiler.compileModule(AST_COMPILER_ENTRY_PATH, entry.source, extraModules);
  const stage2Compiler = await CompilerInstance.create(stage2);
  const stage3 = stage2Compiler.compileModule(AST_COMPILER_ENTRY_PATH, entry.source, extraModules);

  expect(stage3).toEqual(stage2);

  const stage3Compiler = await CompilerInstance.create(stage3);
  const program = stage3Compiler.compileAt(
    COMPILER_INPUT_PTR,
    source.length,
    `
      fn main() -> i32 {
          let mut total: i32 = 0;
          let mut idx: i32 = 0;
          loop {
              if idx >= 5 {
                  break;
              };
              total = total + idx;
              idx = idx + 1;
          };
          total
      }
    `,
  );

  const result = await runWasmMainWithGc(program);
  expect(result).toBe(10);
});
