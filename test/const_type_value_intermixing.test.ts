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

test("const fn with only const parameters can return usable type array", async () => {
    const wasm = await compileWithAstCompiler(`
    const fn foo(const STR_LEN: i32) -> [type; STR_LEN] {
        let entries: [type; STR_LEN] =
            [i32; STR_LEN];
        entries
    }

    const BAR: [type; 3] = foo(3);

    fn main() -> BAR[0] {
        42
    }
    `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(42);
});

test("const fn with only const parameters can return composed tuple-array partial type", async () => {
    const wasm = await compileWithAstCompiler(`
    const fn foo(const STR_LEN: i32) -> ([u8; STR_LEN], type) {
        let entries: ([u8; STR_LEN], type) =
            ([0 as u8; STR_LEN], i32);
        entries
    }

    const BAR: ([u8; 3], type) = foo(3);

    fn main() -> i32 {
        42
    }
    `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(42);
});

test("const fn with only const parameters can return usable composed tuple-array partial type", async () => {
    const wasm = await compileWithAstCompiler(`
    const fn foo(const STR_LEN: i32) -> ([u8; STR_LEN], type) {
        let entries: ([u8; STR_LEN], type) =
            ([0 as u8; STR_LEN], i32);
        entries
    }

    const BAR: ([u8; 3], type) = foo(3);

    fn main() -> BAR.1 {
        42
    }
    `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(42);
});

test("const fn with only const parameters can return usable value from composed tuple-array partial type", async () => {
    const wasm = await compileWithAstCompiler(`
    const fn foo(const STR_LEN: i32) -> ([i32; STR_LEN], type) {
        let entries: ([i32; STR_LEN], type) =
            ([42; STR_LEN], i32);
        entries
    }

    const BAR: ([i32; 3], type) = foo(3);

    fn main() -> i32 {
        BAR.0[0]
    }
    `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(42);
});

test("const fn with only const parameters can return composed array-tuple partial type", async () => {
    const wasm = await compileWithAstCompiler(`
    const KEY_COUNT: i32 = 12;

    const fn foo(const COUNT: i32) -> [(type, type); COUNT] {
        let entries: [(type, type); COUNT] =
            [(u32, i32); COUNT];
        entries
    }

    const BAR: [(type, type); KEY_COUNT] = foo(KEY_COUNT);

    fn main() -> i32 {
        42
    }
    `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(42);
});

test("const fn with only const parameters can return composed array-tuple-array partial type", async () => {
    const wasm = await compileWithAstCompiler(`
    const KEY_COUNT: i32 = 12;
    const KEY_NAME_CAP: i32 = 4;

    const fn foo(const COUNT: i32) -> [([u8; KEY_NAME_CAP], type); COUNT] {
        let entries: [([u8; KEY_NAME_CAP], type); COUNT] =
            [([0 as u8; KEY_NAME_CAP], i32); COUNT];
        return entries;
    }

    const BAR: [([u8; KEY_NAME_CAP], type); KEY_COUNT] = foo(KEY_COUNT);

    fn main() -> i32 {
        42
    }
    `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(42);
});

test("const fn with only const parameters can return spaced composed array-tuple-array partial type", async () => {
    const wasm = await compileWithAstCompiler(`
    const KEY_COUNT: i32 = 12;
    const KEY_NAME_CAP: i32 = 4;

    const fn foo(const COUNT: i32) -> [([u8; KEY_NAME_CAP], type); COUNT] {
        let entries: [([u8; KEY_NAME_CAP], type); COUNT] =
            [([0 as u8; KEY_NAME_CAP], i32); COUNT];
        let idx = 0;
        return entries;
    }

    const BAR: [([u8; KEY_NAME_CAP], type); KEY_COUNT] = foo(KEY_COUNT);

    fn main() -> i32 {
        42
    }
    `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(42);
});

test("const fn with only const parameters can return usable composed array-tuple-array partial type from spaced binding", async () => {
    const wasm = await compileWithAstCompiler(`
    const KEY_COUNT: i32 = 12;
    const KEY_NAME_CAP: i32 = 4;

    const fn foo(const COUNT: i32) -> [([i32; KEY_NAME_CAP], type); COUNT] {
        let mut entries: [([i32; KEY_NAME_CAP], type); COUNT] =
            [([42; KEY_NAME_CAP], i32); COUNT];
        let mut idx = 0;
        entries
    }

    const BAR: [([i32; KEY_NAME_CAP], type); KEY_COUNT] = foo(KEY_COUNT);

    fn main() -> i32 {
        BAR[0].0[0] as i32
    }
    `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(42);
});

test("const fn with only const parameters can return usable composed array-tuple-array u8 partial type from spaced binding", async () => {
    const wasm = await compileWithAstCompiler(`
    const KEY_COUNT: i32 = 12;
    const KEY_NAME_CAP: i32 = 4;

    const fn foo(const COUNT: i32) -> [([u8; KEY_NAME_CAP], type); COUNT] {
        let mut entries: [([u8; KEY_NAME_CAP], type); COUNT] =
            [("abcd", i32); COUNT];
        let mut idx = 0;
        return entries;
    }

    const BAR: [([i32; KEY_NAME_CAP], type); KEY_COUNT] = foo(KEY_COUNT);

    fn main() -> i32 {
        BAR[0].0[0] as i32
    }
    `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(97);
});

test("const fn with only const parameters can return mutated usable composed array-tuple-array partial type", async () => {
    const wasm = await compileWithAstCompiler(`
    const KEY_COUNT: i32 = 12;
    const KEY_NAME_CAP: i32 = 4;

    const fn foo(const COUNT: i32) -> [([u8; KEY_NAME_CAP], type); COUNT] {
        let mut entries: [([u8; KEY_NAME_CAP], type); COUNT] =
            [("abcd", i32); COUNT];
        let mut idx = 0;
        entries[0].0[0] = 'e' as u8;
        entries
    }

    const BAR: [([u8; KEY_NAME_CAP], type); KEY_COUNT] = foo(KEY_COUNT);

    fn main() -> i32 {
        BAR[0].0[0] as i32
    }
    `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(101);
});

test("const fn with only const parameters can simply process composed array-tuple-array partial type", async () => {
    const wasm = await compileWithAstCompiler(`
    const KEY_COUNT: i32 = 12;
    const KEY_NAME_CAP: i32 = 4;

    const fn foo(const COUNT: i32) -> [([u8; KEY_NAME_CAP], type); COUNT] {
        let mut entries: [([u8; KEY_NAME_CAP], type); COUNT] =
            [([0 as u8; KEY_NAME_CAP], i32); COUNT];
        let mut idx = 0;
        while idx < COUNT {
            entries[idx].0[0] = ('k' as u8);
            idx = idx + 1;
        }
        entries
    }

    const BAR: [([u8; KEY_NAME_CAP], type); KEY_COUNT] = foo(KEY_COUNT);

    fn main() -> i32 {
        BAR[0].0[0] as i32
    }
    `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(107);
});

test("const fn with only const parameters can divide to produce composed type", async () => {
    const wasm = await compileWithAstCompiler(`
    const KEY_COUNT: i32 = 12;

    const fn foo(const COUNT: i32) -> [(i32, type); COUNT] {
        let divided: i32 = COUNT / 10;
        return [(42, i32); COUNT];
    }

    const BAR: [(i32, type); KEY_COUNT] = foo(KEY_COUNT);

    fn main() -> i32 {
        BAR[0].0 as i32
    }
    `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(42);
});

test("const fn with only const parameters can fully process composed array-tuple-array partial type", async () => {
    const wasm = await compileWithAstCompiler(`
    const KEY_COUNT: i32 = 12;
    const KEY_NAME_CAP: i32 = 4;

    const fn foo(const COUNT: i32) -> [([u8; KEY_NAME_CAP], type); COUNT] {
        let mut entries: [([u8; KEY_NAME_CAP], type); COUNT] =
            [([0 as u8; KEY_NAME_CAP], i32); COUNT];
        let mut idx = 0;
        while idx < COUNT {
            entries[idx].0[0] = ('k' as u8);
            let mut place = 1;
            let tens = idx / 10;
            if tens > 0 {
                entries[idx].0[place] = (48 + tens) as u8;
                place = place + 1;
            };
            let ones = idx - tens * 10;
            entries[idx].0[place] = (48 + ones) as u8;
            idx = idx + 1;
        }
        entries
    }

    const BAR: [([u8; KEY_NAME_CAP], type); KEY_COUNT] = foo(KEY_COUNT);

    fn main() -> i32 {
        BAR[0].0[0] as i32
    }
    `);
    const result = await runWasmMainWithGc(wasm);
    expect(result).toBe(107);
});
