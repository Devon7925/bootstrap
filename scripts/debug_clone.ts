import { fileURLToPath } from "node:url";
import { readdir } from "node:fs/promises";

import { describeCompilationFailure } from "../src/index";

const encoder = new TextEncoder();
const decoder = new TextDecoder();

const MODULE_STATE_BASE = 1_048_576;
const MODULE_STORAGE_TOP_OFFSET = 4;
const MODULE_COUNT_OFFSET = 0;
const MODULE_TABLE_OFFSET = 8;
const MODULE_ENTRY_SIZE = 20;
const MODULE_PATH_PTR = 1_024;
const MODULE_CONTENT_PTR = 4_096;
const SCRATCH_FAILURE_PATH_PTR_OFFSET = 4_048;
const SCRATCH_FAILURE_PATH_LEN_OFFSET = 4_052;

const CLONE_DEBUG_LOCAL_INDEX_OFFSET = 5_048;
const CLONE_DEBUG_INIT_TYPE_OFFSET = 5_052;
const CLONE_DEBUG_PUSHED_OFFSET = 5_056;
const CLONE_DEBUG_TARGET_FUNC_OFFSET = 5_060;
const CLONE_DEBUG_CALLER_FUNC_OFFSET = 5_064;
const CLONE_DEBUG_LOCATION_OFFSET = 5_068;

const WORD_SIZE = 4;
const SCRATCH_INSTR_CAPACITY = 131_072;
const SCRATCH_FN_BASE_OFFSET = 921_600;
const AST_MAX_FUNCTIONS = 1_024;
const AST_FUNCTION_ENTRY_SIZE = 68;
const AST_NAMES_CAPACITY = 131_072;
const AST_CONSTANTS_CAPACITY = 1_024;
const AST_CONSTANT_ENTRY_SIZE = 28;
const AST_ARRAY_TYPES_CAPACITY = 256;
const AST_ARRAY_TYPE_ENTRY_SIZE = 12;
const AST_TUPLE_TYPES_CAPACITY = 256;
const AST_TUPLE_TYPE_ENTRY_SIZE = 12;
const AST_STRUCT_TYPES_CAPACITY = 256;
const AST_STRUCT_TYPE_ENTRY_SIZE = 20;
const AST_FUNCTION_TYPES_CAPACITY = 256;
const AST_FUNCTION_TYPE_ENTRY_SIZE = 16;
const AST_EXPR_ENTRY_SIZE = 20;
const AST_CONSTANTS_SECTION_SIZE =
  WORD_SIZE + AST_CONSTANTS_CAPACITY * AST_CONSTANT_ENTRY_SIZE;
const AST_CONSTANTS_SECTION_WORDS = AST_CONSTANTS_SECTION_SIZE >> 2;
const AST_CALL_DATA_CAPACITY = 131_072 - AST_CONSTANTS_SECTION_WORDS;
const AST_ARRAY_TYPES_SECTION_SIZE =
  WORD_SIZE + AST_ARRAY_TYPES_CAPACITY * AST_ARRAY_TYPE_ENTRY_SIZE;
const AST_TUPLE_TYPES_SECTION_SIZE =
  WORD_SIZE + AST_TUPLE_TYPES_CAPACITY * AST_TUPLE_TYPE_ENTRY_SIZE;
const AST_STRUCT_TYPES_SECTION_SIZE =
  WORD_SIZE + AST_STRUCT_TYPES_CAPACITY * AST_STRUCT_TYPE_ENTRY_SIZE;
const AST_ARRAY_HEAP_INDEX_SECTION_SIZE = AST_ARRAY_TYPES_CAPACITY * WORD_SIZE;
const AST_TUPLE_HEAP_INDEX_SECTION_SIZE = AST_TUPLE_TYPES_CAPACITY * WORD_SIZE;
const AST_STRUCT_HEAP_INDEX_SECTION_SIZE = AST_STRUCT_TYPES_CAPACITY * WORD_SIZE;
const AST_FUNCTION_TYPES_SECTION_SIZE =
  WORD_SIZE + AST_FUNCTION_TYPES_CAPACITY * AST_FUNCTION_TYPE_ENTRY_SIZE;

const COMPILER_DIR_URL = new URL("../compiler/", import.meta.url);
const STD_MEMORY_URL = new URL("../stdlib/memory.bp", import.meta.url);
const COMPILER_ENTRY_PATH = "/compiler/ast_compiler.bp";

function astOutputReserve(inputLen: number): number {
  const optionA = inputLen + SCRATCH_INSTR_CAPACITY;
  const optionB = SCRATCH_FN_BASE_OFFSET + 16_384;
  return optionA > optionB ? optionA : optionB;
}

function astBase(outPtr: number, inputLen: number): number {
  return outPtr + astOutputReserve(inputLen);
}

function astConstantsCountPtr(astBasePtr: number): number {
  const functionsSection = WORD_SIZE + AST_MAX_FUNCTIONS * AST_FUNCTION_ENTRY_SIZE;
  const namesSection = WORD_SIZE + AST_NAMES_CAPACITY;
  const callDataSection = WORD_SIZE + AST_CALL_DATA_CAPACITY * WORD_SIZE;
  return astBasePtr + functionsSection + namesSection + callDataSection;
}

function astArrayTypesCountPtr(astBasePtr: number): number {
  return astConstantsCountPtr(astBasePtr) + AST_CONSTANTS_SECTION_SIZE;
}

function astTupleTypesCountPtr(astBasePtr: number): number {
  return astArrayTypesCountPtr(astBasePtr) + AST_ARRAY_TYPES_SECTION_SIZE;
}

function astArrayHeapIndicesBase(astBasePtr: number): number {
  return astTupleTypesCountPtr(astBasePtr) + AST_TUPLE_TYPES_SECTION_SIZE;
}

function astTupleHeapIndicesBase(astBasePtr: number): number {
  return astArrayHeapIndicesBase(astBasePtr) + AST_ARRAY_HEAP_INDEX_SECTION_SIZE;
}

function astStructTypesCountPtr(astBasePtr: number): number {
  return astTupleHeapIndicesBase(astBasePtr) + AST_TUPLE_HEAP_INDEX_SECTION_SIZE;
}

function astStructHeapIndicesBase(astBasePtr: number): number {
  return astStructTypesCountPtr(astBasePtr) + AST_STRUCT_TYPES_SECTION_SIZE;
}

function astFunctionTypesCountPtr(astBasePtr: number): number {
  return astStructHeapIndicesBase(astBasePtr) + AST_STRUCT_HEAP_INDEX_SECTION_SIZE;
}

function astExtraBase(astBasePtr: number): number {
  return astFunctionTypesCountPtr(astBasePtr) + AST_FUNCTION_TYPES_SECTION_SIZE;
}

function astExprEntryPtr(astBasePtr: number, index: number): number {
  return astExtraBase(astBasePtr) + WORD_SIZE + index * AST_EXPR_ENTRY_SIZE;
}

function readBytes(memory: WebAssembly.Memory, ptr: number, length: number): string {
  if (ptr <= 0 || length <= 0) {
    return "";
  }
  const view = new Uint8Array(memory.buffer);
  const end = Math.min(ptr + length, view.length);
  return decoder.decode(view.subarray(ptr, end));
}

function writeModuleString(memory: WebAssembly.Memory, ptr: number, text: string): number {
  const normalized = text.replace(/\r\n?/g, "\n");
  const bytes = encoder.encode(normalized);
  const view = new Uint8Array(memory.buffer);
  if (ptr + bytes.length + 1 > view.length) {
    throw new Error("module string overflow");
  }
  view.set(bytes, ptr);
  view[ptr + bytes.length] = 0;
  return bytes.length;
}

function readModuleStorageTop(memory: WebAssembly.Memory): number {
  const view = new DataView(memory.buffer);
  return view.getInt32(MODULE_STATE_BASE + MODULE_STORAGE_TOP_OFFSET, true);
}

async function readModuleSources() {
  const dirPath = fileURLToPath(COMPILER_DIR_URL);
  const entries = await readdir(dirPath, { withFileTypes: true });
  const modules: Array<{ path: string; source: string }> = [];
  for (const entry of entries) {
    if (!entry.isFile() || !entry.name.endsWith(".bp")) {
      continue;
    }
    const fileUrl = new URL(entry.name, COMPILER_DIR_URL);
    const source = await Bun.file(fileUrl).text();
    modules.push({ path: `/compiler/${entry.name}`, source });
  }
  modules.sort((a, b) => a.path.localeCompare(b.path));
  return modules;
}

async function main() {
  const wasmUrl = new URL("../compiler.wasm", import.meta.url);
  const wasmBytes = await Bun.file(wasmUrl).arrayBuffer();
  const module = await WebAssembly.compile(wasmBytes);
  const instance = await WebAssembly.instantiate(module, {});
  const memory = (instance.exports.memory ?? null) as WebAssembly.Memory | null;
  if (!memory) {
    throw new Error("stage2 missing memory");
  }
  const loadModuleFromSource = instance.exports.loadModuleFromSource as
    | ((pathPtr: number, contentPtr: number) => number | bigint)
    | undefined;
  const compileFromPath = instance.exports.compileFromPath as
    | ((pathPtr: number) => number | bigint)
    | undefined;
  if (!loadModuleFromSource || !compileFromPath) {
    throw new Error("stage2 missing exports");
  }

  const memoryIntrinsics = await Bun.file(STD_MEMORY_URL).text();
  const modules = await readModuleSources();
  const entry = modules.find((moduleInfo) => moduleInfo.path === COMPILER_ENTRY_PATH);
  if (!entry) {
    throw new Error("entry module missing");
  }
  const entrySource = entry.source.replace(/\r\n?/g, "\n");
  const entryLength = encoder.encode(entrySource).length;
  const others = modules.filter((moduleInfo) => moduleInfo.path !== COMPILER_ENTRY_PATH);

  const load = (path: string, source: string) => {
    writeModuleString(memory, MODULE_PATH_PTR, path);
    writeModuleString(memory, MODULE_CONTENT_PTR, source);
    const result = loadModuleFromSource(MODULE_PATH_PTR, MODULE_CONTENT_PTR);
    const status = typeof result === "bigint" ? Number(result) : (result | 0);
    if (status < 0) {
      throw new Error(`load failure for ${path} => ${status}`);
    }
  };

  load("/stdlib/memory.bp", memoryIntrinsics);
  for (const moduleInfo of others) {
    load(moduleInfo.path, moduleInfo.source);
  }
  load(entry.path, entrySource);

  const compileResult = compileFromPath(MODULE_PATH_PTR);
  const status = typeof compileResult === "bigint" ? Number(compileResult) : (compileResult | 0);
  const outPtr = readModuleStorageTop(memory);
  if (status <= 0) {
    const failure = describeCompilationFailure(memory, outPtr, status);
    console.log("compile status", status);
    console.log("failure detail", failure.detail);
    const view = new DataView(memory.buffer);
    const debugInfo = {
      localIndex: view.getInt32(CLONE_DEBUG_LOCAL_INDEX_OFFSET, true),
      initType: view.getInt32(CLONE_DEBUG_INIT_TYPE_OFFSET, true),
      pushed: view.getInt32(CLONE_DEBUG_PUSHED_OFFSET, true),
      targetFunc: view.getInt32(CLONE_DEBUG_TARGET_FUNC_OFFSET, true),
      callerFunc: view.getInt32(CLONE_DEBUG_CALLER_FUNC_OFFSET, true),
      location: view.getInt32(CLONE_DEBUG_LOCATION_OFFSET, true),
    };
    console.log("clone debug", debugInfo);
    const constEval = {
      expr: view.getInt32(5_032, true),
      kind: view.getInt32(5_036, true),
      step: view.getInt32(5_040, true),
      local: view.getInt32(5_044, true),
    };
    console.log("const eval", constEval);
    const astBasePtr = astBase(outPtr, entryLength);
    if (constEval.expr >= 0) {
      const exprPtr = astExprEntryPtr(astBasePtr, constEval.expr);
      const expr = {
        kind: view.getInt32(exprPtr, true),
        data0: view.getInt32(exprPtr + WORD_SIZE, true),
        data1: view.getInt32(exprPtr + 2 * WORD_SIZE, true),
        data2: view.getInt32(exprPtr + 3 * WORD_SIZE, true),
        type: view.getInt32(exprPtr + 4 * WORD_SIZE, true),
      };
      console.log("expr", expr);
    }
    const pathPtr = view.getInt32(outPtr + SCRATCH_FAILURE_PATH_PTR_OFFSET, true);
    const pathLen = view.getInt32(outPtr + SCRATCH_FAILURE_PATH_LEN_OFFSET, true);
    if (pathPtr > 0 && pathLen > 0) {
      console.log("failure path", readBytes(memory, pathPtr, pathLen));
    }
    const funcCount = view.getInt32(astBasePtr, true);
    console.log("function count", funcCount);
    const describeFn = (index: number): string => {
      if (index < 0 || index >= funcCount) {
        return "<invalid>";
      }
      const entryPtr = astBasePtr + WORD_SIZE + index * AST_FUNCTION_ENTRY_SIZE;
      const namePtr = view.getInt32(entryPtr, true);
      const nameLen = view.getInt32(entryPtr + 4, true);
      const moduleIndex = view.getInt32(entryPtr + 52, true);
      return `fn[${index}] module=${moduleIndex} name='${readBytes(memory, namePtr, nameLen)}'`;
    };
    console.log("target", describeFn(debugInfo.targetFunc));
    console.log("caller", describeFn(debugInfo.callerFunc));
    const moduleCount = view.getInt32(MODULE_STATE_BASE + MODULE_COUNT_OFFSET, true);
    const moduleTable = MODULE_STATE_BASE + MODULE_TABLE_OFFSET;
    const moduleNames: string[] = [];
    for (let i = 0; i < moduleCount; i += 1) {
      const entryPtr = moduleTable + i * MODULE_ENTRY_SIZE;
      const pathPtr = view.getInt32(entryPtr, true);
      const pathLen = view.getInt32(entryPtr + WORD_SIZE, true);
      moduleNames[i] = readBytes(memory, pathPtr, pathLen);
    }

    const constFns: Array<string> = [];
    const heavyConstFns: Array<string> = [];
    const FUNCTION_FLAG_HAS_CONST_PARAMS = 2;
    for (let i = 0; i < funcCount; i += 1) {
      const entryPtr = astBasePtr + WORD_SIZE + i * AST_FUNCTION_ENTRY_SIZE;
      const flags = view.getInt32(entryPtr + 32, true);
      if ((flags & FUNCTION_FLAG_HAS_CONST_PARAMS) !== 0) {
        const namePtr = view.getInt32(entryPtr, true);
        const nameLen = view.getInt32(entryPtr + 4, true);
        const moduleIndex = view.getInt32(entryPtr + 52, true);
        const locals = view.getInt32(entryPtr + 20, true);
        const name = readBytes(memory, namePtr, nameLen);
        const moduleName = moduleNames[moduleIndex] ?? `<module ${moduleIndex}>`;
        constFns.push(
          `fn[${i}] module=${moduleIndex}(${moduleName}) locals=${locals} name='${name}'`,
        );
        if (locals >= 15) {
          heavyConstFns.push(
            `fn[${i}] module=${moduleIndex}(${moduleName}) locals=${locals} name='${name}'`,
          );
        }
      }
    }
    console.log("const fns", constFns.slice(0, 10));
    if (heavyConstFns.length > 0) {
      console.log("heavy const fns", heavyConstFns.slice(0, 10));
    }
    for (let i = 0; i < Math.min(funcCount, 8); i += 1) {
      const entryPtr = astBasePtr + WORD_SIZE + i * AST_FUNCTION_ENTRY_SIZE;
      const namePtr = view.getInt32(entryPtr, true);
      const nameLen = view.getInt32(entryPtr + 4, true);
      const moduleIndex = view.getInt32(entryPtr + 52, true);
      const name = readBytes(memory, namePtr, nameLen);
      console.log(`fn[${i}] module=${moduleIndex} name='${name}'`);
    }
    return;
  }

  console.log("compile succeeded", status);
}

await main();
