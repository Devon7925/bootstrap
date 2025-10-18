import { expect, test } from "bun:test";

import {
    compileWithAstCompiler,
    expectCompileFailure,
    instantiateWasmModuleWithGc,
    runWasmMainWithGc,
} from "./helpers";

test("const parameters specialize array repeat lengths", async () => {
    const wasm = await compileWithAstCompiler(`
    fn helper(const COUNT: i32, value: i32) -> i32 {
        len([value; COUNT])
    }

    fn main() -> i32 {
        helper(3, 7)
    }
  `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(3);
});

test("const parameters specialize array repeat lengths in let", async () => {
    const wasm = await compileWithAstCompiler(`
    fn helper(const COUNT: i32, value: i32) -> i32 {
        let arr = [value; COUNT];
        len(arr)
    }

    fn main() -> i32 {
        helper(3, 7)
    }
  `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(3);
});

test("const parameters specialize array repeat lengths through let aliases", async () => {
    const wasm = await compileWithAstCompiler(`
    fn helper(const COUNT: i32, value: i32) -> i32 {
        let arr = [value; COUNT];
        let alias = arr;
        len(alias)
    }

    fn main() -> i32 {
        helper(3, 7)
    }
  `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(3);
});

test("const parameters specialize functions with let mut", async () => {
    const wasm = await compileWithAstCompiler(`
    fn helper(const COUNT: i32, value: i32) -> i32 {
        let mut res = COUNT;
        res = res + value;
        res
    }

    fn main() -> i32 {
        helper(3, 7)
    }
  `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(10);
});

test("const parameters specialize functions with array and expression", async () => {
    const wasm = await compileWithAstCompiler(`
    fn helper(const COUNT: i32, value: i32) -> i32 {
        let arr = [value; COUNT];
        COUNT
    }

    fn main() -> i32 {
        helper(3, 7)
    }
  `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(3);
});

test("const parameters specialize complex array repeat lengths as part of index access expression", async () => {
    const wasm = await compileWithAstCompiler(`
    fn helper(const COUNT: i32, value: i32) -> i32 {
        [value; COUNT][0]
    }

    fn main() -> i32 {
        helper(3, 7)
    }
  `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(7);
});

test("const parameter templates specialize if expressions", async () => {
    const wasm = await compileWithAstCompiler(`
    fn const_max(const A: i32, b: i32) -> i32 {
        if A > b {
            A
        } else {
            b    
        }
    }

    fn main() -> i32 {
        const_max(10, 5) + const_max(5, 10)
    }
  `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(15);
});

test("const parameter templates specialize if statement", async () => {
    const wasm = await compileWithAstCompiler(`
    fn const_max(const A: i32, b: i32) -> i32 {
        let mut acc = b;
        if acc > A {
            acc = acc + A;
        };
        
        acc
    }

    fn main() -> i32 {
        const_max(10, 4) + const_max(5, 6)
    }
  `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(15);
});

test("const parameter templates specialize simple loop", async () => {
    const wasm = await compileWithAstCompiler(`
    fn sum(const N: i32) -> i32 {
        let mut index: i32 = 0;
        loop {
            return index;
        }
    }

    fn main() -> i32 {
        sum(5)
    }
  `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(0);
});

test("const parameter templates specialize loop", async () => {
    const wasm = await compileWithAstCompiler(`
    fn sum(const N: i32) -> i32 {
        let mut index: i32 = 0;
        loop {
            if index >= N {
                return index;
            };
            index = index + 1;
            0
        }
    }

    fn main() -> i32 {
        sum(5)
    }
  `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(5);
});

test("const parameter templates specialize complex loop", async () => {
    const wasm = await compileWithAstCompiler(`
    fn sum(const N: i32) -> i32 {
        let mut total: i32 = 0;
        let mut index: i32 = 0;
        loop {
            if index >= N {
                return total;
            };
            total = total + index;
            index = index + 1;
            0
        }
    }

    fn main() -> i32 {
        sum(5)
    }
  `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(10);
});

test("const parameter templates specialize index into let defined array", async () => {
    const wasm = await compileWithAstCompiler(`
    fn sum(const N: i32) -> i32 {
        let values = [3; N];
        values[0]
    }

    fn main() -> i32 {
        sum(5)
    }
  `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(3);
});

test("const parameter templates specialize complex array loop functions", async () => {
    const wasm = await compileWithAstCompiler(`
    fn sum(const N: i32) -> i32 {
        let values = [3; N];
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
        sum(5)
    }
  `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(15);
});

test("const parameter templates specialize array arguments", async () => {
    const wasm = await compileWithAstCompiler(`
    fn head(const N: i32, values: [i32; N]) -> i32 {
        values[0]
    }

    fn main() -> i32 {
        head(3, [4, 5, 6])
    }
  `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(4);
});

test("const parameter templates specialize array arguments with arbitrary indicies", async () => {
    const wasm = await compileWithAstCompiler(`
    fn get(const N: i32, values: [i32; N], idx: i32) -> i32 {
        values[idx]
    }

    fn main() -> i32 {
        get(3, [4, 5, 6], 0) + get(3, [4, 5, 6], 2)
    }
  `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(10);
});

test("const parameter array templates specialize with expression using const parameter", async () => {
    const wasm = await compileWithAstCompiler(`
    fn sum(const N: i32, values: [i32; N]) -> i32 {
        N
    }

    fn main() -> i32 {
        let values: [i32; 3] = [4, 5, 6];
        sum(3, values)
    }
  `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(3);
});

test("const parameter templates specialize complex array arguments", async () => {
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

test("const parameter templates specialize complex string arguments", async () => {
    const wasm = await compileWithAstCompiler(`
    fn expect_keyword_literal(const LEN: i32, keyword: [u8; LEN]) -> i32 {
        let mut idx: i32 = 0;
        loop {
            if idx >= LEN { break; };
            let value: i32 = keyword[idx] as i32;
            idx = idx + 1;
            if value == 0 { return -1; };
        };
        0
    }
    fn main() -> i32 { expect_keyword_literal(3, "foo") }
  `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(0);
});

test("const parameter templates specialize through multiple calls", async () => {
    const wasm = await compileWithAstCompiler(`
    fn foo(const LEN: i32, keyword: [u8; LEN]) -> i32 { LEN }
    fn bar(const LEN: i32, keyword: [u8; LEN]) -> i32 {
        foo(LEN, keyword)
    }
    fn main() -> i32 { bar(2, "fn") }
  `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(2);
});

test("const parameter functions work with other functions", async () => {
    const wasm = await compileWithAstCompiler(`
        fn const_fn(const N: i32) -> i32 { 0 }
        fn foo() -> i32 { 3 }
        fn main() -> i32 { foo() + foo() }
  `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(6);
});

test("const parameter templates specialize return types", async () => {
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

test.todo("type-valued const parameters specialize signatures", async () => {
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
    expect(failure.failure.detail).toBe(
      "const parameter arguments must be compile-time constants",
    );
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
    expect(failure.failure.detail).toBe(
      "const parameter template expected type mismatch",
    );
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
    expect(failure.failure.detail).toBe(
      "const parameter template expected type mismatch",
    );
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