import { expect, test } from "bun:test";

import {
  compileWithAstCompiler,
  expectCompileFailure,
  instantiateAstCompiler,
  readAstCompilerModules,
  readAstCompilerSource,
  runWasmMainWithGc,
  instantiateWasmModuleWithGc,
  DEFAULT_OUTPUT_STRIDE,
  AST_COMPILER_ENTRY_PATH,
} from "./helpers";

test("functions can call other functions", async () => {
  const wasm = await compileWithAstCompiler(`
    fn helper() -> i32 {
        40
    }

    fn main() -> i32 {
        helper()
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(40);
});

test("functions can accept parameters", async () => {
  const wasm = await compileWithAstCompiler(`
    fn add(a: i32, b: i32) -> i32 {
        a + b
    }

    fn main() -> i32 {
        add(40, 2)
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(42);
});

test("forward function calls are supported", async () => {
  const wasm = await compileWithAstCompiler(`
    fn main() -> i32 {
        helper()
    }

    fn helper() -> i32 {
        42
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(42);
});

test("unknown function calls are rejected", async () => {
  const failure = await expectCompileFailure(`
    fn call_missing() -> i32 {
        missing()
    }
  `);
  expect(failure.failure.producedLength).toBeLessThanOrEqual(0);
});

test("call argument counts must match function signature", async () => {
  const failure = await expectCompileFailure(`
    fn add(a: i32, b: i32) -> i32 {
        a + b
    }

    fn call_add() -> i32 {
        add(1)
    }
  `);
  expect(failure.failure.producedLength).toBeLessThanOrEqual(0);
});

test("duplicate function names are rejected", async () => {
  const failure = await expectCompileFailure(`
    fn helper() -> i32 {
        1
    }

    fn helper() -> i32 {
        2
    }
  `);
  expect(failure.failure.producedLength).toBeLessThanOrEqual(0);
});

test("functions may omit return types", async () => {
  const wasm = await compileWithAstCompiler(`
    fn helper() {
        let mut counter: i32 = 0;
        counter = counter + 1;
    }

    fn main() -> i32 {
        helper();
        42
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(42);
});

test("functions support many parameters", async () => {
  const wasm = await compileWithAstCompiler(`
    fn wide(
        a0: i32,
        a1: i32,
        a2: i32,
        a3: i32,
        a4: i32,
        a5: i32,
        a6: i32,
        a7: i32,
        a8: i32,
        a9: i32,
        a10: i32,
        a11: i32,
        a12: i32,
        a13: i32,
        a14: i32,
        a15: i32,
        a16: i32,
        a17: i32,
        a18: i32,
        a19: i32,
    ) -> i32 {
        a0 + a1 + a2 + a3 + a4 + a5 + a6 + a7 + a8 + a9
            + a10 + a11 + a12 + a13 + a14 + a15 + a16 + a17 + a18 + a19
    }

    fn main() -> i32 {
        wide(
            0,
            1,
            2,
            3,
            4,
            5,
            6,
            7,
            8,
            9,
            10,
            11,
            12,
            13,
            14,
            15,
            16,
            17,
            18,
            19,
        )
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(190);
});

test("functions can return from multiple paths", async () => {
  const wasm = await compileWithAstCompiler(`
    fn choose(flag: i32) -> i32 {
        if flag {
            return 10;
        } else {
            return 20;
        }
    }

    fn accumulate(limit: i32) -> i32 {
        let mut total: i32 = 0;
        let mut current: i32 = limit;
        loop {
            if current <= 0 {
                return total;
            } else {
                total = total + current;
                current = current - 1;
                0
            };
        }
    }

    fn main() -> i32 {
        choose(1) + choose(0) + accumulate(3)
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(36);
});

test("functions can use local variables", async () => {
  const wasm = await compileWithAstCompiler(`
    fn compute() -> i32 {
        let base: i32 = 40;
        let mut total: i32 = base + 1;
        total = total + 1;
        total
    }

    fn main() -> i32 {
        compute()
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(42);
});

test("const parameters accept literal arguments", async () => {
  const wasm = await compileWithAstCompiler(`
    fn add_count(const COUNT: i32, value: i32) -> i32 {
        value + COUNT
    }

    fn main() -> i32 {
        add_count(5, 37)
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(42);
});

test("const parameters require compile-time constant arguments", async () => {
  const failure = await expectCompileFailure(`
    fn scale(const FACTOR: i32, value: i32) -> i32 {
        value * FACTOR
    }

    fn main() -> i32 {
        let runtime: i32 = 6;
        scale(runtime, 7)
    }
  `);
  expect(failure.failure.producedLength).toBeLessThanOrEqual(0);
});

test("const parameters accept const fn results", async () => {
  const wasm = await compileWithAstCompiler(`
    const fn three() -> i32 {
        3
    }

    fn multiply(const TIMES: i32, value: i32) -> i32 {
        value * TIMES
    }

    fn main() -> i32 {
        multiply(three(), 14)
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(42);
});

test("const parameter templates reject mismatched parameter arrays", async () => {
  const failure = await expectCompileFailure(`
    fn sum(const N: i32, values: [i32; N]) -> i32 {
        let mut total: i32 = 0;
        let mut index: i32 = 0;
        loop {
            if index >= N {
                return total;
            };
            total = total + values[index];
            index = index + 1;
            0
        }
    }

    fn main() -> i32 {
        let values: [i32; 3] = [5, 6, 7];
        sum(4, values)
    }
  `);
  expect(failure.failure.producedLength).toBeLessThanOrEqual(0);
});

test("const parameter return templates reject mismatched bindings", async () => {
  const failure = await expectCompileFailure(`
    fn build(const N: i32, value: i32) -> [i32; N] {
        [value; N]
    }

    fn main() -> i32 {
        let values: [i32; 3] = build(2, 5);
        values[0]
    }
  `);
  expect(failure.failure.producedLength).toBeLessThanOrEqual(0);
});

test("const parameter templates are not exported", async () => {
  const wasm = await compileWithAstCompiler(`
    fn helper(const COUNT: i32, value: i32) -> i32 {
        value + COUNT
    }

    fn main() -> i32 {
        helper(5, 37)
    }
  `);
  const instance = await instantiateWasmModuleWithGc(wasm);
  const exportNames = Object.keys(instance.exports);
  expect(exportNames).toContain("memory");
  expect(exportNames).toContain("main");
  expect(exportNames).not.toContain("helper");
});

test("array repeat length reports runtime parameter usage", async () => {
  const failure = await expectCompileFailure(`
    fn helper(count: i32, value: i32) -> i32 {
        len([value; count])
    }

    fn main() -> i32 {
        helper(3, 7)
    }
  `);
  expect(failure.failure.detail).toBe("array literal length requires const parameters");
});

test.skip("const parameter templates specialize array arguments", async () => {
  const wasm = await compileWithAstCompiler(`
    fn sum(const N: i32, values: [i32; N]) -> i32 {
        let mut total: i32 = 0;
        let mut index: i32 = 0;
        loop {
            if index >= N {
                return total;
            };
            total = total + values[index];
            index = index + 1;
            0
        }
    }

    fn main() -> i32 {
        let values: [i32; 3] = [4, 5, 6];
        sum(3, values)
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(15);
});

test.skip("const parameter templates specialize return types", async () => {
  const wasm = await compileWithAstCompiler(`
    fn build(const N: i32, value: i32) -> [i32; N] {
        [value; N]
    }

    fn main() -> i32 {
        let values: [i32; 3] = build(3, 5);
        values[0] + values[1] + values[2]
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(15);
});

test.skip("type-valued const parameters specialize signatures", async () => {
  const wasm = await compileWithAstCompiler(`
    fn select(const T: type, flag: bool, on_true: T, on_false: T) -> T {
        if flag { on_true } else { on_false }
    }

    fn main() -> i32 {
        select(i32, true, 40, 2)
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(42);
});

test("function section handles multibyte type indices", async () => {
  const helperCount = (1 << 7) + 2;
  const parts: string[] = [];
  for (let idx = 0; idx < helperCount; idx += 1) {
    parts.push(`fn helper_${idx}() -> i32 {`);
    parts.push(`    ${idx}`);
    parts.push("}");
    parts.push("");
  }
  parts.push("fn main() -> i32 {");
  parts.push(`    helper_${helperCount - 1}()`);
  parts.push("}");

  const source = parts.join("\n");
  const wasm = await compileWithAstCompiler(source);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(helperCount - 1);
});

test("ast compiler source can be compiled once", async () => {
  const compiler = await instantiateAstCompiler();
  const modules = await readAstCompilerModules();
  const entry = modules.find((module) => module.path === AST_COMPILER_ENTRY_PATH);
  if (!entry) {
    throw new Error("ast compiler entry module not found");
  }
  const extraModules = modules.filter((module) => module.path !== AST_COMPILER_ENTRY_PATH);
  const wasm = compiler.compileModule(AST_COMPILER_ENTRY_PATH, entry.source, extraModules);
  expect(wasm.length).toBeGreaterThan(DEFAULT_OUTPUT_STRIDE);
});
