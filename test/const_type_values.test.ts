import { expect, test } from "bun:test";

import {
    compileWithAstCompiler,
    runWasmMainWithGc,
} from "./helpers";

test("type-valued const parameters specialize signatures", async () => {
    const wasm = await compileWithAstCompiler(`
    fn select(const T: type, flag: bool, on_true: T, on_false: T) -> T {
        if flag { on_true } else { on_false }
    }

    fn main() -> i32 {
        select(i32, true, 40, 2)
    }
  `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(40);
});

test("type-valued const parameters support multiple invocations", async () => {
    const wasm = await compileWithAstCompiler(`
    fn select(const T: type, flag: bool, on_true: T, on_false: T) -> T {
        if flag { on_true } else { on_false }
    }

    fn main() -> i32 {
        let base = select(i32, false, 2, 40);
        let bonus = if select(bool, true, true, false) { 100 } else { 0 };
        base + bonus
    }
  `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(140);
});

test("const fn returning a type can be used to pick const parameter types", async () => {
    const wasm = await compileWithAstCompiler(`
    const fn identity_type(const T: type) -> type {
        T
    }

    fn forward(const T: type, value: T) -> T {
        value
    }

    fn main() -> i32 {
        forward(identity_type(i32), 11)
    }
  `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(11);
});

test("const fn returning a type can define tuple return types", async () => {
    const wasm = await compileWithAstCompiler(`
    const fn duplicate_pair_type(const T: type) -> type {
        (T, T)
    }

    fn make_pair(value: i32) -> duplicate_pair_type(i32) {
        (value, value + 1)
    }

    fn main() -> i32 {
        let pair = make_pair(20);
        pair.0 + pair.1
    }
  `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(41);
});

test("const fn returning a type can define array return types", async () => {
    const wasm = await compileWithAstCompiler(`
    const fn array_of(const T: type, const LEN: i32) -> type {
        [T; LEN]
    }

    fn init_array() -> array_of(i32, 3) {
        [1, 2, 3]
    }

    fn main() -> i32 {
        let arr = init_array();
        arr[0] + arr[1] + arr[2]
    }
  `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(6);
});

test("nested const type functions support conditional selection", async () => {
    const wasm = await compileWithAstCompiler(`
    const fn select_type_by_index(
        const INDEX: i32,
        const FIRST: type,
        const SECOND: type,
        const THIRD: type,
    ) -> type {
        if INDEX == 0 {
            FIRST
        } else if INDEX == 1 {
            SECOND
        } else {
            THIRD
        }
    }

    fn first() -> select_type_by_index(0, i32, bool, i32) {
        7
    }

    fn second() -> select_type_by_index(1, i32, bool, i32) {
        true
    }

    fn third() -> select_type_by_index(2, i32, bool, i32) {
        9
    }

    fn main() -> i32 {
        let mut total = first();
        if second() {
            total = total + third();
        };
        total
    }
  `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(16);
});
