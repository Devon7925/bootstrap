import { fileURLToPath } from "node:url";
import {
  readAstCompilerModules,
  readExpressionCount,
  readExpressionEntry,
  readExpressionType,
  readCallDataInfo,
} from "../test/helpers";
import { describeCompilationFailure, FAILURE_DETAIL_CAPACITY } from "../src/index";

const encoder = new TextEncoder();
const decoder = new TextDecoder();

const MODULE_PATH_PTR = 1_024;
const MODULE_CONTENT_PTR = 4_096;
const MODULE_STATE_BASE = 1_048_576;
const MODULE_STORAGE_TOP_OFFSET = 4;
const FUNCTIONS_BASE_OFFSET = 851_968;
const FUNCTION_ENTRY_SIZE = 68;
const FUNCTIONS_COUNT_PTR_OFFSET = 851_960;
const INSTR_OFFSET_PTR_OFFSET = 4_096;
const CLONE_DEBUG_LOCAL_INDEX_OFFSET = 5_048;
const CLONE_DEBUG_INIT_TYPE_OFFSET = 5_052;
const CLONE_DEBUG_PUSHED_OFFSET = 5_056;
const SCRATCH_FAILURE_PATH_PTR_OFFSET = 4_048;
const SCRATCH_FAILURE_PATH_LEN_OFFSET = 4_052;
const SCRATCH_FAILURE_LINE_OFFSET = 4_056;
const SCRATCH_FAILURE_COLUMN_OFFSET = 4_060;
const SCRATCH_INSTR_CAPACITY = 131_072;
const SCRATCH_FN_BASE_OFFSET = 925_700;
const WORD_SIZE = 4;
const CALL_METADATA_CALLEE_PARAM_BASE = -1_024;

function growMemoryIfRequired(memory: WebAssembly.Memory, required: number) {
  const current = memory.buffer.byteLength;
  if (required <= current) {
    return;
  }
  const pageSize = 65_536;
  const additional = required - current;
  const pagesNeeded = Math.ceil(additional / pageSize);
  memory.grow(pagesNeeded);
}

function writeModuleString(memory: WebAssembly.Memory, ptr: number, text: string): number {
  const bytes = encoder.encode(text);
  growMemoryIfRequired(memory, ptr + bytes.length + 1);
  const view = new Uint8Array(memory.buffer);
  view.set(bytes, ptr);
  view[ptr + bytes.length] = 0;
  return bytes.length;
}

function readModuleStorageTop(memory: WebAssembly.Memory): number {
  const view = new DataView(memory.buffer);
  return view.getInt32(MODULE_STATE_BASE + MODULE_STORAGE_TOP_OFFSET, true);
}

function safeReadI32(view: DataView, offset: number): number {
  if (offset < 0 || offset + 4 > view.byteLength) {
    return -1;
  }
  try {
    return view.getInt32(offset, true);
  } catch {
    return -1;
  }
}

function astOutputReserve(inputLen: number): number {
  const optionA = inputLen + SCRATCH_INSTR_CAPACITY;
  const optionB = SCRATCH_FN_BASE_OFFSET + 16_384;
  return optionA > optionB ? optionA : optionB;
}

function astBase(outPtr: number, inputLen: number): number {
  return outPtr + astOutputReserve(inputLen);
}

function callMetadataConstKeyPtr(memory: WebAssembly.Memory, metadataPtr: number): number {
  if (metadataPtr <= 0) {
    return 0;
  }
  const view = new DataView(memory.buffer);
  const argCount = safeReadI32(view, metadataPtr + 8);
  if (argCount < 0) {
    return 0;
  }
  const extraBase = metadataPtr + 16 + argCount * WORD_SIZE;
  const keyPtrPtr = extraBase + 4 * WORD_SIZE;
  return safeReadI32(view, keyPtrPtr);
}

async function main() {
  const compilerUrl = new URL("../compiler.wasm", import.meta.url);
  const compilerBytes = await Bun.file(compilerUrl).arrayBuffer();
  const { instance } = await WebAssembly.instantiate(compilerBytes, {});
  const memory = instance.exports.memory as WebAssembly.Memory | undefined;
  const loadModule = instance.exports.loadModuleFromSource as
    | ((pathPtr: number, contentPtr: number) => number | bigint)
    | undefined;
  const compileFromPath = instance.exports.compileFromPath as
    | ((pathPtr: number) => number | bigint)
    | undefined;
  if (!memory || typeof loadModule !== "function" || typeof compileFromPath !== "function") {
    throw new Error("stage2 exports missing");
  }

  const modules = await readAstCompilerModules();
  const entry = modules.find((module) => module.path === "/compiler/ast_compiler.bp");
  if (!entry) {
    throw new Error("entry not found");
  }
  const extraModules = modules.filter((module) => module.path !== entry.path);

  const memoryIntrinsicsUrl = new URL("../stdlib/memory.bp", import.meta.url);
  const memoryIntrinsicsSource = await Bun.file(memoryIntrinsicsUrl).text();

  const loadModuleText = (path: string, contents: string) => {
    writeModuleString(memory, MODULE_PATH_PTR, path);
    writeModuleString(memory, MODULE_CONTENT_PTR, contents);
    const result = loadModule(MODULE_PATH_PTR, MODULE_CONTENT_PTR);
    const status = typeof result === "bigint" ? Number(result) : (result as number) | 0;
    if (status < 0) {
      throw new Error(`load ${path} failed with ${status}`);
    }
  };

  loadModuleText("/stdlib/memory.bp", memoryIntrinsicsSource);
  for (const module of extraModules) {
    loadModuleText(module.path, module.source);
  }
  loadModuleText(entry.path, entry.source);

  const result = compileFromPath(MODULE_PATH_PTR);
  const producedLen = typeof result === "bigint" ? Number(result) : (result as number) | 0;
  const outputPtr = readModuleStorageTop(memory);
  console.log("producedLen", producedLen, "outputPtr", outputPtr);
  const failure = describeCompilationFailure(memory, outputPtr, producedLen);
  console.log("failure", failure);
  const view = new DataView(memory.buffer);
  console.log("clone debug", {
    localIndex: safeReadI32(view, outputPtr + CLONE_DEBUG_LOCAL_INDEX_OFFSET),
    initType: safeReadI32(view, outputPtr + CLONE_DEBUG_INIT_TYPE_OFFSET),
    pushed: safeReadI32(view, outputPtr + CLONE_DEBUG_PUSHED_OFFSET),
  });
  const line = safeReadI32(view, outputPtr + SCRATCH_FAILURE_LINE_OFFSET);
  const column = safeReadI32(view, outputPtr + SCRATCH_FAILURE_COLUMN_OFFSET);
  const pathPtr = safeReadI32(view, outputPtr + SCRATCH_FAILURE_PATH_PTR_OFFSET);
  const pathLen = safeReadI32(view, outputPtr + SCRATCH_FAILURE_PATH_LEN_OFFSET);
  console.log("location", { line, column, pathPtr, pathLen });
  if (pathPtr > 0 && pathLen > 0 && pathPtr + pathLen <= memory.buffer.byteLength) {
    const bytes = new Uint8Array(memory.buffer, pathPtr, pathLen);
    console.log("path", decoder.decode(bytes));
  } else {
    console.log("path skipped", { pathPtr, pathLen, buffer: memory.buffer.byteLength });
  }
  const detailBytes = new Uint8Array(memory.buffer.slice(outputPtr, outputPtr + FAILURE_DETAIL_CAPACITY));
  console.log("raw detail", decoder.decode(detailBytes));

  const inputLen = encoder.encode(entry.source).length;
  const astBasePtr = astBase(outputPtr, inputLen);
  const funcCount = safeReadI32(view, astBasePtr);
  console.log("function count", funcCount);
  const functions: Array<{ index: number; name: string; moduleBase: number; bodyKind: number; bodyData0: number }>= [];
  for (let index = 0; index < funcCount; index += 1) {
    const entryPtr = astBasePtr + WORD_SIZE + index * FUNCTION_ENTRY_SIZE;
    const namePtr = safeReadI32(view, entryPtr);
    const nameLen = safeReadI32(view, entryPtr + 4);
    const bodyKind = safeReadI32(view, entryPtr + 12);
    const bodyData0 = safeReadI32(view, entryPtr + 16);
    const moduleBase = safeReadI32(view, entryPtr + 44);
    const moduleLen = safeReadI32(view, entryPtr + 48);
    const nameStart = safeReadI32(view, entryPtr + 56);
    let name = "";
    if (moduleBase > 0 && nameStart >= 0 && nameLen > 0 && moduleBase + nameStart + nameLen <= memory.buffer.byteLength) {
      const bytes = new Uint8Array(memory.buffer, moduleBase + nameStart, nameLen);
      name = decoder.decode(bytes);
    } else if (namePtr > 0 && nameLen > 0 && namePtr + nameLen <= memory.buffer.byteLength) {
      const bytes = new Uint8Array(memory.buffer, namePtr, nameLen);
      name = decoder.decode(bytes);
    }
    functions.push({
      index,
      name,
      namePtr,
      nameLen,
      moduleBase,
      moduleLen,
      nameStart,
      bodyKind,
      bodyData0,
    });
  }
  const targetFunction = functions.find((fn) => fn.name.includes("collect_const_param_children"));
  console.log("target function", targetFunction);
  const recordEmit = functions.find((fn) => fn.name === "record_emit_failure");
  console.log("record_emit_failure function", recordEmit);
  const moduleEntryStore = functions.find((fn) => fn.name === "module_entry_store");
  console.log("module_entry_store function", moduleEntryStore);
  const interesting = [577, 578, 601];
  for (const idx of interesting) {
    const entry = functions[idx];
    if (entry) {
      console.log("function", idx, JSON.stringify(entry));
    }
  }
  const exprCount = readExpressionCount(memory, outputPtr, inputLen);
  const childMetadata: number[] = [];
  const paramMetadata: number[] = [];
  const literalInterest = new Set([5, 26, 27, 28, 33, 34, 40, 41, 43, 45, 47, 49, 51, 55, 56, 499]);
  const literalMatches: Array<{ index: number; value: number; type: number }> = [];
  for (let index = 0; index < exprCount; index += 1) {
    const expr = readExpressionEntry(memory, outputPtr, inputLen, index);
    if (expr.kind !== 1) {
      if (expr.kind === 0 && literalInterest.has(expr.data0)) {
        const exprType = readExpressionType(memory, outputPtr, inputLen, index);
        literalMatches.push({ index, value: expr.data0, type: exprType });
      }
      continue;
    }
    const metadataPtr = expr.data0;
    if (metadataPtr <= 0) {
      continue;
    }
    const calleeIndex = safeReadI32(view, metadataPtr + 12);
    if (calleeIndex <= CALL_METADATA_CALLEE_PARAM_BASE) {
      paramMetadata.push(metadataPtr);
    }
    const keyPtr = callMetadataConstKeyPtr(memory, metadataPtr);
    if (keyPtr > 0) {
      const entryCount = safeReadI32(view, keyPtr);
      for (let idx = 0; idx < entryCount; idx += 1) {
        const entryPtr = keyPtr + WORD_SIZE + idx * 3 * WORD_SIZE;
        const param = safeReadI32(view, entryPtr);
        if (param === 1) {
          childMetadata.push(metadataPtr);
          break;
        }
      }
    }
  }
  console.log("child metadata pointers", childMetadata);
  console.log("param callee metadata", paramMetadata);
  console.log("literal matches", literalMatches);
  for (const metadataPtr of childMetadata) {
    const argCount = safeReadI32(view, metadataPtr + 8);
    const calleeIndex = safeReadI32(view, metadataPtr + 12);
    const callee = functions[calleeIndex];
    const namePtr = safeReadI32(view, metadataPtr);
    const nameLen = safeReadI32(view, metadataPtr + 4);
    let callName = "";
    if (namePtr > 0 && nameLen > 0 && namePtr + nameLen <= memory.buffer.byteLength) {
      const bytes = new Uint8Array(memory.buffer, namePtr, nameLen);
      callName = decoder.decode(bytes);
    }
    const keyPtr = callMetadataConstKeyPtr(memory, metadataPtr);
    const runtimeArgs: number[] = [];
    const argsBase = metadataPtr + 16;
    for (let arg = 0; arg < argCount; arg += 1) {
      runtimeArgs.push(safeReadI32(view, argsBase + arg * WORD_SIZE));
    }
    if (calleeIndex === targetFunction?.index) {
      console.log(
        "target call metadata",
        JSON.stringify({ metadataPtr, argCount, keyPtr, runtimeArgs }),
      );
    }
    console.log(
      "metadata",
      JSON.stringify({
        metadataPtr,
        argCount,
        calleeIndex,
        calleeName: callee?.name,
        callName,
        keyPtr,
        runtimeArgs,
      }),
    );
    if (keyPtr > 0) {
      const entryCount = safeReadI32(view, keyPtr);
      const entries: Array<{ param: number; value: number; type: number }> = [];
      for (let idx = 0; idx < entryCount; idx += 1) {
        const entryPtr = keyPtr + WORD_SIZE + idx * 3 * WORD_SIZE;
        entries.push({
          param: safeReadI32(view, entryPtr),
          value: safeReadI32(view, entryPtr + WORD_SIZE),
          type: safeReadI32(view, entryPtr + 2 * WORD_SIZE),
        });
      }
      console.log("key entries", entries);
    }
  }
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
