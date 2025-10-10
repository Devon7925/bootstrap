import { expect, test } from "bun:test";

import {
  compileWithAstCompiler,
  expectExportedFunction,
  instantiateWasmModuleWithGc,
  runWasmMainWithGc,
} from "./helpers";

test("bitwise functions export operations", async () => {
  const source = String.raw`
fn bit_ops(a: i32, b: i32) -> i32 {
    let and_value: i32 = a & b;
    let or_value: i32 = a | b;
    (and_value << 1) + or_value
}

fn shifts(value: i32, amount: i32) -> i32 {
    (value << amount) + (value >> amount)
}

fn main() -> i32 {
    bit_ops(12, 5) + shifts(-8, 1)
}
`;

  const wasm = await compileWithAstCompiler(source);
  const instance = await instantiateWasmModuleWithGc(wasm);

  const bitOps = expectExportedFunction(instance, "bit_ops");
  const shifts = expectExportedFunction(instance, "shifts");
  const main = expectExportedFunction(instance, "main");

  const bitResult = bitOps(0b1100, 0b0101);
  const expectedBit = ((0b1100 & 0b0101) << 1) + (0b1100 | 0b0101);
  expect(bitResult).toBe(expectedBit);

  const shiftResult = shifts(-32, 2);
  expect(shiftResult).toBe((-32 << 2) + (-32 >> 2));

  const mainResult = main();
  const expectedMain = ((12 & 5) << 1) + (12 | 5) + ((-8 << 1) + (-8 >> 1));
  expect(mainResult).toBe(expectedMain);
});

test("bitwise expressions execute", async () => {
  const source = String.raw`
fn evaluate(a: i32, b: i32, shift: i32) -> i32 {
    let mask: i32 = (a & b) | ((a | b) >> shift);
    (mask << 1) + (a >> shift)
}

fn main() -> i32 {
    let first: i32 = evaluate(29, 23, 2);
    let second: i32 = evaluate(-64, 7, 3);
    first + second
}
`;

  const wasm = await compileWithAstCompiler(source);
  const result = await runWasmMainWithGc(wasm);

  const evaluate = (a: number, b: number, shift: number) => {
    const mask = (a & b) | ((a | b) >> shift);
    return (mask << 1) + (a >> shift);
  };
  const expected = evaluate(29, 23, 2) + evaluate(-64, 7, 3);

  expect(result).toBe(expected);
});
