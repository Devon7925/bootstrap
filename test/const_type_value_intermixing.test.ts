import { expect, test } from "bun:test";

import { compileWithAstCompiler, expectCompileFailure, runWasmMainWithGc } from "./helpers";

test("const tuples can mix type and value entries", async () => {
    const wasm = await compileWithAstCompiler(`
    const fn type_value_pair() -> (type, i32) {
        (i32, 2)
    }

    const fn pair_type() -> type {
        type_value_pair().0
    }

    const fn pair_value() -> i32 {
        type_value_pair().1
    }

    fn main() -> i32 {
        let value: pair_type() = pair_value();
        value
    }
  `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(2);
});

test("const arrays can hold type entries", async () => {
    const wasm = await compileWithAstCompiler(`
    const fn numeric_types() -> [type; 3] {
        [i32, u32, u8]
    }

    const fn third_numeric_type() -> type {
        numeric_types()[2]
    }

    fn main() -> i32 {
        let value: third_numeric_type() = 200;
        value as i32
    }
  `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(200);
});

test("can make tuple from types", async () => {
    const wasm = await compileWithAstCompiler(`
    const TYPE_VALUE_PAIR: (type, type) = (i32, u8);

    const PAIR_TYPE: type = TYPE_VALUE_PAIR.0;

    fn main() -> i32 {
        let value: PAIR_TYPE = 2;
        value
    }
  `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(2);
});

test("const tuple bindings can mix type and value entries", async () => {
    const wasm = await compileWithAstCompiler(`
    const TYPE_VALUE_PAIR: (type, i32) = (i32, 2);

    const PAIR_TYPE: type = TYPE_VALUE_PAIR.0;
    const PAIR_VALUE: i32 = TYPE_VALUE_PAIR.1;

    fn main() -> i32 {
        let value: PAIR_TYPE = PAIR_VALUE;
        value
    }
  `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(2);
});

test("const arrays of types can be bound to names", async () => {
    const wasm = await compileWithAstCompiler(`
    const NUMERIC_TYPES: [type; 3] = [i32, u32, u8];

    const THIRD_TYPE: type = NUMERIC_TYPES[2];

    fn main() -> i32 {
        let value: THIRD_TYPE = 200;
        value as i32
    }
  `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(200);
});

test("const parameters can read tuple value entries", async () => {
    const wasm = await compileWithAstCompiler(`
    fn use_pair(const PAIR: (type, i32)) -> i32 {
        PAIR.1
    }

    fn main() -> i32 {
        use_pair((i32, 3))
    }
  `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(3);
});

test("const parameters can accept tuples mixing types and values", async () => {
    const wasm = await compileWithAstCompiler(`
    fn use_pair(const PAIR: (type, i32)) -> i32 {
        let value: PAIR.0 = PAIR.1;
        value
    }

    fn main() -> i32 {
        use_pair((i32, 3))
    }
  `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(3);
});

test("const parameters can accept arrays of types", async () => {
    const wasm = await compileWithAstCompiler(`
    fn use_types(const TYPES: [type; 3]) -> i32 {
        let value: TYPES[1] = 100;
        value as i32
    }

    fn main() -> i32 {
        use_types([i32, u32, u8])
    }
  `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(100);
});

test("const parameter templates returning type rejects mismatched parameter arrays", async () => {
    const failure = await expectCompileFailure(`
    const fn sum(const N: i32, values: [i32; N]) -> type {
        i32
    }

    fn main() -> i32 {
        let x: sum(4, [5, 6, 7]) = 5;
        0
    }
  `);
    expect(failure.failure.detail).toBe(
        "/entry.bp:7:16: const parameter template expected type mismatch",
    );
});

test("struct like function signiture compiles", async () => {
    const wasm = await compileWithAstCompiler(`
    const fn struct_like(const STR_LEN: i32, const PROP_COUNT: i32, const PROPS: [([u8; STR_LEN], type); PROP_COUNT]) -> type {
        i32
    }

    fn main() -> i32 {
        let x: struct_like(3, 1, [("foo", i32)]) = 3;
        x
    }
  `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(3);
});

test("const fn with only const parameters can use let for array types", async () => {
    const wasm = await compileWithAstCompiler(`
    const fn foo(const STR_LEN: i32) -> type {
        let entries: [type; STR_LEN] = [i32; STR_LEN];
        i32
    }

    fn main() -> i32 {
        let set: foo(3) = 42;
        set
    }
    `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(42);
});

test("const fn with only const parameters can use let for tuple types", async () => {
    const wasm = await compileWithAstCompiler(`
    const fn foo(const STR_LEN: i32) -> type {
        let entries: (type, type) =
            (u32, i32);
        i32
    }

    fn main() -> i32 {
        let set: foo(3) = 42;
        set
    }
    `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(42);
});

test("const fn with only const parameters can use let for array in tuple types", async () => {
    const wasm = await compileWithAstCompiler(`
    const fn foo(const STR_LEN: i32) -> type {
        let entries: ([type; STR_LEN], type) =
            ([i32; STR_LEN], i32);
        i32
    }

    fn main() -> i32 {
        let set: foo(3) = 42;
        set
    }
    `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(42);
});

test("const fn with only const parameters can use let for tuple in array types", async () => {
    const wasm = await compileWithAstCompiler(`
    const fn foo(const STR_LEN: i32) -> type {
        let entries: [(type, type); STR_LEN] =
            [(i32, i32); STR_LEN];
        i32
    }

    fn main() -> i32 {
        let set: foo(3) = 42;
        set
    }
    `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(42);
});

test("const fn with only const parameters can return type array", async () => {
    const wasm = await compileWithAstCompiler(`
    const fn foo(const STR_LEN: i32) -> [type; STR_LEN] {
        let entries: [type; STR_LEN] =
            [i32; STR_LEN];
        entries
    }

    const BAR: [type; 3] = foo(3);

    fn main() -> i32 {
        42
    }
    `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(42);
});
