import { expect, test } from "bun:test";

import {
  COMPILER_INPUT_PTR,
  DEFAULT_OUTPUT_STRIDE,
  expectCompileFailure,
  instantiateAstCompiler,
  runWasmMainWithGc,
} from "./helpers";

const textDecoder = new TextDecoder();

interface Cursor {
  index: number;
}

interface FunctionType {
  readonly params: readonly number[];
  readonly results: readonly number[];
}

interface ExportEntry {
  readonly kind: number;
  readonly index: number;
}

interface ParsedModule {
  readonly importedFunctionCount: number;
  readonly functionTypeIndices: readonly number[];
  readonly functionBodies: readonly Uint8Array[];
  readonly types: readonly FunctionType[];
  readonly exports: ReadonlyMap<string, ExportEntry>;
}

test("const parameters specialize functions and emit unique clones", async () => {
  const compiler = await instantiateAstCompiler();
  const source = `
    fn choose(const FLAG: bool, value: i32) -> i32 {
        if FLAG {
            value
        } else {
            value + 10
        }
    }

    fn main() -> i32 {
        let first: i32 = choose(true, 7);
        let second: i32 = choose(true, 3);
        let base: i32 = 5;
        let third: i32 = choose(false, base);
        first + second + third
    }
  `;

  const wasm = compiler.compileWithLayout(COMPILER_INPUT_PTR, DEFAULT_OUTPUT_STRIDE, source);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(25);

  const parsed = parseWasmModule(wasm);
  expect(parsed.functionBodies.length).toBe(3);

  const mainExport = parsed.exports.get("main");
  expect(mainExport).toBeDefined();
  expect(mainExport!.kind).toBe(0);

  const mainBody = getFunctionBody(parsed, mainExport!.index);
  const mainCalls = extractCallIndices(mainBody);
  expect(mainCalls.length).toBe(3);

  const uniqueCallTargets = Array.from(new Set(mainCalls));
  expect(uniqueCallTargets.length).toBe(2);
  expect(mainCalls[0]).toBe(mainCalls[1]);

  const specializedSummaries = uniqueCallTargets.map((index) => {
    const body = getFunctionBody(parsed, index);
    const type = getFunctionType(parsed, index);
    expect(type.params.length).toBe(1);
    return {
      index,
      initialConst: firstI32Const(body),
    };
  });

  const boolConstants = specializedSummaries
    .map((summary) => summary.initialConst)
    .sort((a, b) => Number(a) - Number(b));
  expect(boolConstants).toEqual([0, 1]);
});

test("const parameter specializations support loops", async () => {
  const compiler = await instantiateAstCompiler();
  const source = `
    fn identity(const VALUE: i32) -> i32 {
        loop {
            break VALUE;
        }
    }

    fn accumulate(const COUNT: i32) -> i32 {
        let mut index: i32 = 0;
        let mut total: i32 = 0;
        loop {
            if index >= COUNT {
                break total;
            };
            total = total + index;
            index = index + 1;
            0
        }
    }

    fn main() -> i32 {
        accumulate(5) + identity(7)
    }
  `;

  const wasm = compiler.compileWithLayout(COMPILER_INPUT_PTR, DEFAULT_OUTPUT_STRIDE, source);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(17);
});

test("const specialization overflow reports function limit detail", async () => {
  const cloneCount = 1_023;
  const lines: string[] = [
    "fn choose(const VALUE: i32) -> i32 {",
    "    VALUE",
    "}",
    "",
    "fn main() -> i32 {",
    "    let mut total: i32 = 0;",
  ];
  for (let value = 0; value < cloneCount; value += 1) {
    lines.push(`    total = total + choose(${value});`);
  }
  lines.push("    total");
  lines.push("}");
  const source = lines.join("\n");

  const failure = await expectCompileFailure(source);
  expect(failure.failure.detail).toBeDefined();
  expect(failure.failure.detail).toMatch(
    /\/entry\.bp:\d+:\d+: const specialization function limit exceeded$/,
  );
});

function parseWasmModule(bytes: Uint8Array): ParsedModule {
  if (bytes.length < 8 || bytes[0] !== 0x00 || bytes[1] !== 0x61 || bytes[2] !== 0x73 || bytes[3] !== 0x6d) {
    throw new Error("invalid wasm module");
  }

  const cursor: Cursor = { index: 8 };
  let importedFunctionCount = 0;
  const types: FunctionType[] = [];
  const functionTypeIndices: number[] = [];
  const functionBodies: Uint8Array[] = [];
  const exports = new Map<string, ExportEntry>();

  while (cursor.index < bytes.length) {
    const sectionId = bytes[cursor.index++];
    const sectionLength = readU32Leb(bytes, cursor);
    const sectionEnd = cursor.index + sectionLength;

    switch (sectionId) {
      case 1: {
        const typeCount = readU32Leb(bytes, cursor);
        for (let typeIndex = 0; typeIndex < typeCount; typeIndex += 1) {
          const form = bytes[cursor.index++];
          if (form !== 0x60) {
            throw new Error(`unsupported type form ${form}`);
          }
          const paramCount = readU32Leb(bytes, cursor);
          const params: number[] = [];
          for (let param = 0; param < paramCount; param += 1) {
            params.push(bytes[cursor.index++]);
          }
          const resultCount = readU32Leb(bytes, cursor);
          const results: number[] = [];
          for (let result = 0; result < resultCount; result += 1) {
            results.push(bytes[cursor.index++]);
          }
          types.push({ params, results });
        }
        break;
      }
      case 2: {
        const importCount = readU32Leb(bytes, cursor);
        for (let importIndex = 0; importIndex < importCount; importIndex += 1) {
          const moduleLen = readU32Leb(bytes, cursor);
          cursor.index += moduleLen;
          const fieldLen = readU32Leb(bytes, cursor);
          cursor.index += fieldLen;
          const kind = bytes[cursor.index++];
          if (kind === 0) {
            readU32Leb(bytes, cursor);
            importedFunctionCount += 1;
          } else if (kind === 1) {
            cursor.index += 1; // element type
            readLimits(bytes, cursor);
          } else if (kind === 2) {
            readLimits(bytes, cursor);
          } else if (kind === 3) {
            cursor.index += 1; // value type
            cursor.index += 1; // mutability
          } else {
            throw new Error(`unsupported import kind ${kind}`);
          }
        }
        break;
      }
      case 3: {
        const funcCount = readU32Leb(bytes, cursor);
        for (let func = 0; func < funcCount; func += 1) {
          functionTypeIndices.push(readU32Leb(bytes, cursor));
        }
        break;
      }
      case 7: {
        const exportCount = readU32Leb(bytes, cursor);
        for (let exportIndex = 0; exportIndex < exportCount; exportIndex += 1) {
          const nameLength = readU32Leb(bytes, cursor);
          const nameBytes = bytes.subarray(cursor.index, cursor.index + nameLength);
          cursor.index += nameLength;
          const name = textDecoder.decode(nameBytes);
          const kind = bytes[cursor.index++];
          const index = readU32Leb(bytes, cursor);
          exports.set(name, { kind, index });
        }
        break;
      }
      case 10: {
        const bodyCount = readU32Leb(bytes, cursor);
        for (let bodyIndex = 0; bodyIndex < bodyCount; bodyIndex += 1) {
          const bodySize = readU32Leb(bytes, cursor);
          const start = cursor.index;
          functionBodies.push(bytes.slice(start, start + bodySize));
          cursor.index = start + bodySize;
        }
        break;
      }
      default:
        cursor.index = sectionEnd;
        break;
    }

    cursor.index = sectionEnd;
  }

  return {
    importedFunctionCount,
    functionTypeIndices,
    functionBodies,
    types,
    exports,
  };
}

function getFunctionBody(parsed: ParsedModule, index: number): Uint8Array {
  const definedIndex = index - parsed.importedFunctionCount;
  if (definedIndex < 0 || definedIndex >= parsed.functionBodies.length) {
    throw new Error(`function body for index ${index} not found`);
  }
  return parsed.functionBodies[definedIndex];
}

function getFunctionType(parsed: ParsedModule, index: number): FunctionType {
  const definedIndex = index - parsed.importedFunctionCount;
  if (definedIndex < 0 || definedIndex >= parsed.functionTypeIndices.length) {
    throw new Error(`function type for index ${index} not found`);
  }
  const typeIndex = parsed.functionTypeIndices[definedIndex];
  const type = parsed.types[typeIndex];
  if (!type) {
    throw new Error(`missing type entry ${typeIndex}`);
  }
  return type;
}

function extractCallIndices(body: Uint8Array): number[] {
  const cursor: Cursor = { index: 0 };
  const localsCount = readU32Leb(body, cursor);
  for (let localIndex = 0; localIndex < localsCount; localIndex += 1) {
    const count = readU32Leb(body, cursor);
    cursor.index += 1; // local type
  }

  const calls: number[] = [];
  walkInstructions(body, cursor, (opcode, cursorRef) => {
    switch (opcode) {
      case 0x10:
        calls.push(readU32Leb(body, cursorRef));
        break;
      case 0x41:
        readI32Leb(body, cursorRef);
        break;
      case 0x20:
      case 0x21:
      case 0x22:
        readU32Leb(body, cursorRef);
        break;
      case 0x6a:
      case 0x6b:
      case 0x6c:
      case 0x6d:
      case 0x6e:
      case 0x6f:
      case 0x1a:
      case 0x0f:
      case 0x45:
      case 0x46:
      case 0x47:
      case 0x48:
      case 0x49:
      case 0x4a:
      case 0x4b:
      case 0x4c:
      case 0x4d:
      case 0x4e:
      case 0x4f:
      case 0x0c:
        break;
      default:
        throw new Error(`unsupported opcode 0x${opcode.toString(16)}`);
    }
  });
  return calls;
}

function firstI32Const(body: Uint8Array): number | null {
  const cursor: Cursor = { index: 0 };
  const localsCount = readU32Leb(body, cursor);
  for (let localIndex = 0; localIndex < localsCount; localIndex += 1) {
    const count = readU32Leb(body, cursor);
    cursor.index += 1;
  }

  let found: number | null = null;
  walkInstructions(body, cursor, (opcode, cursorRef) => {
    switch (opcode) {
      case 0x41: {
        const value = readI32Leb(body, cursorRef);
        if (found === null) {
          found = value;
        }
        break;
      }
      case 0x10:
        readU32Leb(body, cursorRef);
        break;
      case 0x20:
      case 0x21:
      case 0x22:
        readU32Leb(body, cursorRef);
        break;
      case 0x6a:
      case 0x6b:
      case 0x6c:
      case 0x6d:
      case 0x6e:
      case 0x6f:
      case 0x1a:
      case 0x0f:
        break;
      default:
        throw new Error(`unsupported opcode 0x${opcode.toString(16)}`);
    }
  });
  return found;
}

function walkInstructions(
  bytes: Uint8Array,
  cursor: Cursor,
  visit: (opcode: number, cursor: Cursor) => void,
): void {
  while (cursor.index < bytes.length) {
    const opcode = bytes[cursor.index++];
    if (opcode === 0x0b) {
      return;
    }
    if (opcode === 0x05) {
      return;
    }
    if (opcode === 0x02 || opcode === 0x03 || opcode === 0x04) {
      cursor.index += 1; // block type
      walkInstructions(bytes, cursor, visit);
      if (opcode === 0x04 && cursor.index < bytes.length && bytes[cursor.index] === 0x05) {
        cursor.index += 1; // else
        walkInstructions(bytes, cursor, visit);
      }
      continue;
    }
    visit(opcode, cursor);
  }
}

function readU32Leb(bytes: Uint8Array, cursor: Cursor): number {
  let result = 0;
  let shift = 0;
  while (true) {
    const byte = bytes[cursor.index++];
    result |= (byte & 0x7f) << shift;
    if ((byte & 0x80) === 0) {
      break;
    }
    shift += 7;
  }
  return result >>> 0;
}

function readI32Leb(bytes: Uint8Array, cursor: Cursor): number {
  let result = 0;
  let shift = 0;
  let byte = 0;
  do {
    byte = bytes[cursor.index++];
    result |= (byte & 0x7f) << shift;
    shift += 7;
  } while (byte & 0x80);

  if (shift < 32 && (byte & 0x40) !== 0) {
    result |= ~0 << shift;
  }
  return result | 0;
}

function readLimits(bytes: Uint8Array, cursor: Cursor): void {
  const flags = readU32Leb(bytes, cursor);
  readU32Leb(bytes, cursor);
  if ((flags & 0x01) !== 0) {
    readU32Leb(bytes, cursor);
  }
}
