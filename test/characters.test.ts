import { expect, test } from "bun:test";

import {
  compileWithAstCompiler,
  expectCompileFailure,
  expectExportedFunction,
  instantiateWasmModuleWithGc,
} from "./helpers";

test("character literals execute", async () => {
  const source = String.raw`
fn char_math() -> i32 {
    let letter: i32 = 'a';
    let newline: i32 = '\n';
    let quote: i32 = '\'';
    letter + newline + quote
}

fn slash() -> i32 {
    '\\'
}

fn main() -> i32 {
    if '\\' == 92 {
        char_math() - '\\' + 'A'
    } else {
        0
    }
}
`;

  const wasm = await compileWithAstCompiler(source);
  const instance = await instantiateWasmModuleWithGc(wasm);

  const charMath = expectExportedFunction(instance, "char_math");
  const slash = expectExportedFunction(instance, "slash");
  const main = expectExportedFunction(instance, "main");

  expect(charMath()).toBe(146);
  expect(slash()).toBe(92);
  expect(main()).toBe(119);
});

test("invalid character literals are rejected", async () => {
  const source = String.raw`
fn invalid() -> i32 {
    let bad: i32 = 'ab';
    bad
}
`;

  const failure = await expectCompileFailure(source);
  expect(failure.failure.detail).toBe(
    "/entry.bp:3:20: character literal must have one character",
  );
});
