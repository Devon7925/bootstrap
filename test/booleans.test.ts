import { expect, test } from "bun:test";

import {
  compileWithAstCompiler,
  expectExportedFunction,
  instantiateWasmModuleWithGc,
  runWasmMainWithGc,
} from "./helpers";

test("boolean logic and loops execute", async () => {
  const wasm = await compileWithAstCompiler(`
    fn choose(flag: bool, a: i32, b: i32) -> i32 {
        if flag {
            a
        } else {
            b
        }
    }

    fn logic_chain(a: bool, b: bool, c: bool) -> bool {
        if a && (b || c) {
            true
        } else {
            false
        }
    }

    fn find_even(limit: i32) -> bool {
        let mut i: i32 = 0;
        let mut found: bool = false;
        loop {
            if i == limit {
                break;
            };
            let remainder: i32 = i - (i / 2) * 2;
            if remainder == 0 && i != 0 {
                found = true;
                break;
            };
            i = i + 1;
        }
        found
    }

    fn main() -> i32 {
        let first: i32 = choose(true, 10, 20);
        let second: i32 = choose(false, 1, 2);
        if logic_chain(true, false, true) && find_even(5) {
            first + second
        } else {
            0
        }
    }
  `);
  const instance = await instantiateWasmModuleWithGc(wasm);
  const choose = expectExportedFunction(instance, "choose");
  const logicChain = expectExportedFunction(instance, "logic_chain");
  const findEven = expectExportedFunction(instance, "find_even");
  const main = expectExportedFunction(instance, "main");

  expect(choose(1, 7, 3)).toBe(7);
  expect(choose(0, 7, 3)).toBe(3);

  expect(logicChain(1, 0, 1)).toBe(1);
  expect(logicChain(0, 1, 1)).toBe(0);

  expect(findEven(6)).toBe(1);
  expect(findEven(1)).toBe(0);

  expect(main()).toBe(12);
});

test("boolean types and literals execute", async () => {
  const wasm = await compileWithAstCompiler(`
    fn invert(flag: bool) -> bool {
        if flag {
            false
        } else {
            true
        }
    }

    fn main() -> i32 {
        let truth: bool = true;
        let falsity: bool = invert(truth);
        if falsity {
            0
        } else {
            1
        }
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(1);
});

test("logical operators short circuit", async () => {
  const wasm = await compileWithAstCompiler(`
    fn main() -> i32 {
        let mut count: i32 = 0;
        let result1: bool = true || { count = count + 1; false };
        let result2: bool = false || { count = count + 1; true };
        let result3: bool = false && { count = count + 1; true };
        let result4: bool = true && { count = count + 1; true };
        let toggled: bool = !false;
        let double_negated: bool = !(!true);
        let inverted: bool = !result4;
        if result1 && result2 && !result3 && result4 && toggled && !inverted && double_negated {
            count
        } else {
            0
        }
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(2);
});
