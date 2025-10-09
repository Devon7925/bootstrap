import { expect, test } from "bun:test";

import {
  COMPILER_INPUT_PTR,
  DEFAULT_OUTPUT_STRIDE,
  instantiateAstCompiler,
} from "./helpers";

type ValueType =
  | { readonly kind: "i32" }
  | { readonly kind: "i64" }
  | { readonly kind: "ref"; readonly nullable: boolean; readonly heapType: number }
  | { readonly kind: "other"; readonly code: number };

interface Cursor {
  index: number;
}

function readU32Leb(bytes: Uint8Array, cursor: Cursor): number {
  let result = 0;
  let shift = 0;
  while (true) {
    const byte = bytes[cursor.index];
    cursor.index += 1;
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
  while (true) {
    byte = bytes[cursor.index];
    cursor.index += 1;
    result |= (byte & 0x7f) << shift;
    shift += 7;
    if ((byte & 0x80) === 0) {
      break;
    }
  }
  if (shift < 32 && (byte & 0x40) !== 0) {
    result |= ~0 << shift;
  }
  return result | 0;
}

function readValueType(bytes: Uint8Array, cursor: Cursor): ValueType {
  const code = readI32Leb(bytes, cursor);
  switch (code) {
    case 0x7f:
    case -0x01:
      return { kind: "i32" };
    case 0x7e:
    case -0x02:
      return { kind: "i64" };
    case -0x1c: {
      const heapType = readI32Leb(bytes, cursor);
      return { kind: "ref", nullable: false, heapType };
    }
    case -0x1d: {
      const heapType = readI32Leb(bytes, cursor);
      return { kind: "ref", nullable: true, heapType };
    }
    default:
      return { kind: "other", code };
  }
}

function findSection(bytes: Uint8Array, targetId: number): Uint8Array | undefined {
  if (bytes.length < 8) {
    return undefined;
  }
  let index = 8;
  while (index < bytes.length) {
    const sectionId = bytes[index];
    index += 1;
    const cursor: Cursor = { index };
    const payloadLen = readU32Leb(bytes, cursor);
    index = cursor.index;
    if (index + payloadLen > bytes.length) {
      return undefined;
    }
    if (sectionId === targetId) {
      const start = index;
      const end = index + payloadLen;
      return bytes.slice(start, end);
    }
    index += payloadLen;
  }
  return undefined;
}

test("array types emit gc entries", async () => {
  const compiler = await instantiateAstCompiler();
  const wasm = compiler.compileWithLayout(
    COMPILER_INPUT_PTR,
    DEFAULT_OUTPUT_STRIDE,
    `
        fn accepts(arg: [i32; 4]) -> i32 {
            0
        }

        fn main() -> i32 {
            0
        }
    `,
  );

  expect(wasm.length).toBeGreaterThan(8);

  const typeSection = findSection(wasm, 1);
  expect(typeSection).toBeDefined();
  const typeCursor: Cursor = { index: 0 };
  const typeCount = readU32Leb(typeSection!, typeCursor);
  expect(typeCount).toBe(3);

  const arrayTag = readI32Leb(typeSection!, typeCursor);
  expect(arrayTag).toBe(-0x22);
  const elementType = readValueType(typeSection!, typeCursor);
  expect(elementType).toEqual({ kind: "i32" });
  const mutability = typeSection![typeCursor.index];
  typeCursor.index += 1;
  expect(mutability).toBe(1);

  const func0Tag = readI32Leb(typeSection!, typeCursor);
  expect(func0Tag).toBe(-0x20);
  const func0Params = readU32Leb(typeSection!, typeCursor);
  expect(func0Params).toBe(1);
  const func0ParamType = readValueType(typeSection!, typeCursor);
  expect(func0ParamType).toEqual({ kind: "ref", nullable: false, heapType: 0 });
  const func0Results = readU32Leb(typeSection!, typeCursor);
  expect(func0Results).toBe(1);
  const func0ResultType = readValueType(typeSection!, typeCursor);
  expect(func0ResultType).toEqual({ kind: "i32" });

  const func1Tag = readI32Leb(typeSection!, typeCursor);
  expect(func1Tag).toBe(-0x20);
  const func1Params = readU32Leb(typeSection!, typeCursor);
  expect(func1Params).toBe(0);
  const func1Results = readU32Leb(typeSection!, typeCursor);
  expect(func1Results).toBe(1);
  const func1ResultType = readValueType(typeSection!, typeCursor);
  expect(func1ResultType).toEqual({ kind: "i32" });
  expect(typeCursor.index).toBe(typeSection!.length);

  const functionSection = findSection(wasm, 3);
  expect(functionSection).toBeDefined();
  const funcCursor: Cursor = { index: 0 };
  const funcDeclCount = readU32Leb(functionSection!, funcCursor);
  expect(funcDeclCount).toBe(2);
  const acceptsTypeIndex = readU32Leb(functionSection!, funcCursor);
  expect(acceptsTypeIndex).toBe(1);
  const mainTypeIndex = readU32Leb(functionSection!, funcCursor);
  expect(mainTypeIndex).toBe(2);
  expect(funcCursor.index).toBe(functionSection!.length);
});
