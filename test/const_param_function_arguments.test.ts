import { expect, test } from "bun:test";

import {
    compileWithAstCompiler,
    expectCompileFailure,
    runWasmMainWithGc,
} from "./helpers";

const sourceHeader = `
    fn add_one(value: i32) -> i32 {
        value + 1
    }

    fn multiply_by_three(value: i32) -> i32 {
        value * 3
    }
`;

test("const parameters accept named function references", async () => {
    const wasm = await compileWithAstCompiler(`
    ${sourceHeader}

    fn apply(const HANDLER: fn(i32) -> i32, value: i32) -> i32 {
        HANDLER(value)
    }

    fn main() -> i32 {
        apply(add_one, 5)
    }
  `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(6);
});

test("const parameters accept const bindings that store functions", async () => {
    const wasm = await compileWithAstCompiler(`
    ${sourceHeader}

    const CHOSEN: fn(i32) -> i32 = multiply_by_three;

    fn apply(const HANDLER: fn(i32) -> i32, value: i32) -> i32 {
        HANDLER(value)
    }

    fn main() -> i32 {
        apply(CHOSEN, 4)
    }
  `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(12);
});

test.todo("let bindings storing functions are rejected for const parameters", async () => {
    const failure = await expectCompileFailure(`
    ${sourceHeader}

    fn apply(const HANDLER: fn(i32) -> i32, value: i32) -> i32 {
        HANDLER(value)
    }

    fn main() -> i32 {
        let runtime = add_one;
        apply(runtime, 9)
    }
  `);
    expect(failure.failure.detail).toBe(
        "/entry.bp:14:9: const parameter arguments must be compile-time constants",
    );
});

test("function const parameters handle weird function signatures", async () => {
    const wasm = await compileWithAstCompiler(`
    fn merge_pair(value: (i32, bool)) -> (i32, bool) {
        value
    }

    fn array_summary(values: [i32; 4]) -> (i32, bool) {
        (values[0] + values[1] + values[2] + values[3], values[0] == values[3])
    }

    fn orchestrate(
        const PROJECT: fn([i32; 4]) -> (i32, bool),
        data: [i32; 4],
    ) -> (i32, bool) {
        merge_pair(PROJECT(data))
    }

    fn main() -> i32 {
        let result = orchestrate(array_summary, [1, 2, 3, 1]);
        if result.1 {
            result.0
        } else {
            0
        }
    }
  `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(7);
});

test("const parameter specialization differentiates functions by identity", async () => {
    const wasm = await compileWithAstCompiler(`
    ${sourceHeader}

    fn choose(const HANDLER: fn(i32) -> i32, a: i32, b: i32) -> i32 {
        HANDLER(a) + HANDLER(b)
    }

    fn main() -> i32 {
        let left = choose(add_one, 2, 3);
        let right = choose(multiply_by_three, 2, 3);
        left + right
    }
  `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(22);
});
