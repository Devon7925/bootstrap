import { loadAstCompilerModuleSources } from "../test/helpers";
import { readModuleStorageTop } from "../test/helpers";
import {
  describeCompilationFailure,
  Target,
  compileToWasm,
  CompileError,
} from "../src/index";

async function main() {
  const modules = await loadAstCompilerModuleSources();
  const entry = modules.find((module) => module.path === "/compiler/ast_compiler.bp");
  if (!entry) {
    throw new Error("entry not found");
  }
  const extraModules = modules.filter((module) => module.path !== "/compiler/ast_compiler.bp");
  try {
    await compileToWasm(entry.source, { entryPath: entry.path, modules: extraModules });
  } catch (error) {
    console.error("compileToWasm error", error);
    if (error instanceof CompileError) {
      console.error("detail:", error.message);
    }
  }
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
