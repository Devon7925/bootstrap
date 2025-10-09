import { expect, test } from "bun:test";

import {
  compileWithAstCompiler,
  expectCompileFailure,
  expectExportedFunction,
  instantiateWasmModuleWithGc,
  runWasmMainWithGc,
} from "./helpers";

test("numeric operations execute", async () => {
  const wasm = await compileWithAstCompiler(`
    fn add_offset(a: i32, b: i32) -> i32 {
        a + b + 1
    }

    fn sum_values() -> i32 {
        let mut total: i32 = 1;
        total = total + 2;
        total
    }

    fn main() -> i32 {
        0
    }
  `);
  const instance = await instantiateWasmModuleWithGc(wasm);
  const addOffset = expectExportedFunction(instance, "add_offset");
  const sumValues = expectExportedFunction(instance, "sum_values");
  const main = expectExportedFunction(instance, "main");

  expect(addOffset(10, 5)).toBe(16);
  expect(sumValues()).toBe(3);
  expect(main()).toBe(0);
});

test("float remainder is rejected", async () => {
  const error = await expectCompileFailure(`
    fn float_mod() -> f32 {
        5.0f32 % 2.0f32
    }

    fn main() -> i32 {
        0
    }
  `);
  expect(error.failure.producedLength).toBeLessThanOrEqual(0);
});

test("literal addition executes", async () => {
  const wasm = await compileWithAstCompiler(`
    fn main() -> i32 {
        1 + 2 + 3
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(6);
});

test("addition with function call executes", async () => {
  const wasm = await compileWithAstCompiler(`
    fn helper() -> i32 {
        5
    }

    fn main() -> i32 {
        helper() + 7
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(12);
});

test("comparison operators evaluate", async () => {
  const wasm = await compileWithAstCompiler(`
    fn evaluate(a: i32, b: i32) -> i32 {
        let mut total: i32 = 0;
        if a == b {
            total = total + 1;
            0
        } else {
            total = total + 2;
            0
        };
        if a != b {
            total = total + 4;
            0
        } else {
            total = total + 8;
            0
        };
        if a < b {
            total = total + 16;
            0
        } else {
            total = total + 32;
            0
        };
        if a > b {
            total = total + 64;
            0
        } else {
            total = total + 128;
            0
        };
        if a <= b {
            total = total + 256;
            0
        } else {
            total = total + 512;
            0
        };
        if a >= b {
            total = total + 1024;
            0
        } else {
            total = total + 2048;
            0
        };
        total
    }

    fn precedence() -> i32 {
        let mut total: i32 = 0;
        if 1 + 2 == 3 {
            total = total + 1000;
            0
        } else {
            total = total + 1;
            0
        };
        if 20 - 5 >= 15 {
            total = total + 2000;
            0
        } else {
            total = total + 2;
            0
        };
        if 3 * 3 < 10 {
            total = total + 4000;
            0
        } else {
            total = total + 4;
            0
        };
        total
    }

    fn main() -> i32 {
        evaluate(4, 4) + evaluate(2, 5) + precedence()
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(10903);
});

test("missing function in addition is rejected", async () => {
  const error = await expectCompileFailure(`
    fn main() -> i32 {
        missing() + 1
    }
  `);
  expect(error.failure.producedLength).toBeLessThanOrEqual(0);
});

test("literal subtraction executes", async () => {
  const wasm = await compileWithAstCompiler(`
    fn main() -> i32 {
        50 - 8
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(42);
});

test("subtraction with function call executes", async () => {
  const wasm = await compileWithAstCompiler(`
    fn helper() -> i32 {
        20
    }

    fn main() -> i32 {
        helper() - 7
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(13);
});

test("subtraction rejects unknown function calls", async () => {
  const error = await expectCompileFailure(`
    fn main() -> i32 {
        5 - missing()
    }
  `);
  expect(error.failure.producedLength).toBeLessThanOrEqual(0);
});

test("literal multiplication executes", async () => {
  const wasm = await compileWithAstCompiler(`
    fn main() -> i32 {
        6 * 7
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(42);
});

test("multiplication with function call executes", async () => {
  const wasm = await compileWithAstCompiler(`
    fn helper() -> i32 {
        6
    }

    fn main() -> i32 {
        helper() * 7
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(42);
});

test("literal division executes", async () => {
  const wasm = await compileWithAstCompiler(`
    fn main() -> i32 {
        126 / 3
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(42);
});

test("multiplication precedence is respected", async () => {
  const wasm = await compileWithAstCompiler(`
    fn main() -> i32 {
        2 + 3 * 4
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(14);
});

test("multiplication rejects unknown function calls", async () => {
  const error = await expectCompileFailure(`
    fn main() -> i32 {
        3 * missing()
    }
  `);
  expect(error.failure.producedLength).toBeLessThanOrEqual(0);
});

test("mixed addition and subtraction executes", async () => {
  const wasm = await compileWithAstCompiler(`
    fn main() -> i32 {
        10 + 5 - 3 + 2 - 4
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(10);
});
