import { expect, test } from "bun:test";

import { compileWithAstCompiler, expectCompileFailure, runWasmMainWithGc } from "./helpers";

test("constant main returns literal value", async () => {
  const wasm = await compileWithAstCompiler(`
    fn main() -> i32 {
        42
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(42);
});

test("global constants can be referenced from main", async () => {
  const wasm = await compileWithAstCompiler(`
    const ANSWER: i32 = 42;

    fn main() -> i32 {
        ANSWER
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(42);
});

test("constants can reference other constants", async () => {
  const wasm = await compileWithAstCompiler(`
    const BASE: i32 = 40;
    const VALUE: i32 = BASE;

    fn main() -> i32 {
        VALUE + 2
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(42);
});

test("duplicate constants are rejected", async () => {
  const failure = await expectCompileFailure(`
    const VALUE: i32 = 1;
    const VALUE: i32 = 2;
  `);
  expect(failure.failure.detail).toBe(
    "/entry.bp:3:11: duplicate constant declaration",
  );
});

test("non literal constant initializers are evaluated", async () => {
  const wasm = await compileWithAstCompiler(`
    const VALUE: i32 = (1 + 2) * 3 - 5;

    fn main() -> i32 {
        VALUE
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(4);
});

test("non-const function calls in constant initializers are rejected", async () => {
  const failure = await expectCompileFailure(`
    const VALUE: i32 = helper();

    fn helper() -> i32 {
        42
    }
  `);
  expect(failure.failure.detail).toBe(
    "/entry.bp:2:11: const initializer must be compile-time evaluable",
  );
});

test("const modulo expressions support signed operands", async () => {
  const wasm = await compileWithAstCompiler(`
    const POSITIVE: i32 = 42 % 5;
    const NEGATIVE: i32 = -42 % 5;
    const MIXED_SIGN: i32 = 42 % -5;

    fn main() -> i32 {
        let mut score: i32 = 0;
        if POSITIVE == 2 {
            score = score + 1;
        };
        if NEGATIVE == -2 {
            score = score + 10;
        };
        if MIXED_SIGN == 2 {
            score = score + 100;
        };
        score
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(111);
});

test("const modulo expressions support unsigned operands", async () => {
  const wasm = await compileWithAstCompiler(`
    const LARGE: u32 = 4_000_000_005;
    const STEP: u32 = 3_000_000_002;
    const REM: u32 = LARGE % STEP;
    const TARGET: u32 = 1_000_000_003;

    fn main() -> i32 {
        if REM == TARGET { 1 } else { 0 }
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(1);
});

test("const modulo by zero is rejected", async () => {
  const failure = await expectCompileFailure(`
    const VALUE: i32 = 10 % 0;
  `);
  expect(failure.failure.detail).toBe("type metadata resolution failed");
});

test("const functions can be used in constant initializers", async () => {
  const wasm = await compileWithAstCompiler(`
    const fn add(a: i32, b: i32) -> i32 {
        a + b
    }

    const VALUE: i32 = add(40, 2);

    fn main() -> i32 {
        VALUE
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(42);
});

test("const functions can use const function results as parameters", async () => {
  const wasm = await compileWithAstCompiler(`
    const fn base() -> i32 {
        40
    }

    const fn plus_two(value: i32) -> i32 {
        value + 2
    }

    const VALUE: i32 = plus_two(base());

    fn main() -> i32 {
        VALUE
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(42);
});

test("const functions can use let bindings", async () => {
  const wasm = await compileWithAstCompiler(`
    const fn base() -> i32 {
        let x: i32 = 40;
        x + 2
    }

    const VALUE: i32 = base();

    fn main() -> i32 {
        VALUE
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(42);
});

test("const functions can use let mut bindings", async () => {
  const wasm = await compileWithAstCompiler(`
    const fn base() -> i32 {
        let mut x: i32 = 30;
        x = 40;
        x + 2
    }

    const VALUE: i32 = base();

    fn main() -> i32 {
        VALUE
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(42);
});

test("const functions can conditionally assign", async () => {
  const wasm = await compileWithAstCompiler(`
    const fn base(cond: bool) -> i32 {
        let mut x: i32 = 30;
        if cond {
            x = 40;
        };
        x + 2
    }

    const VALUE: i32 = base(true);

    fn main() -> i32 {
        VALUE
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(42);
});

test("constants from imports are propagated", async () => {
  const wasm = await compileWithAstCompiler(
    `
    use "/tests/lib/value.bp";

    const OFFSET: i32 = 5;
    const TOTAL: i32 = PROVIDED_VALUE + OFFSET;

    fn main() -> i32 {
        TOTAL
    }
  `,
    {
      modules: [
        {
          path: "/tests/lib/value.bp",
          source: `
            const PROVIDED_VALUE: i32 = 37;
          `,
        },
      ],
    },
  );
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(42);
});

test("const functions can use loops", async () => {
  const wasm = await compileWithAstCompiler(`
    const fn loop_sum(limit: i32) -> i32 {
        let mut acc: i32 = 0;
        let mut i: i32 = 0;
        loop {
            if i == limit {
                break;
            };
            acc = acc + i;
            i = i + 1;
        }
        acc
    }

    const VALUE: i32 = loop_sum(5);

    fn main() -> i32 {
        VALUE
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(10);
});

test("const functions specialize simple const parameters during interpretation", async () => {
  const wasm = await compileWithAstCompiler(`
    const fn add_count(const COUNT: i32, value: i32) -> i32 {
        value + COUNT
    }

    const VALUE: i32 = add_count(2, 40);

    fn main() -> i32 {
        VALUE
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(42);
});

test("const functions returning arrays evaluate for runtime initializers", async () => {
  const wasm = await compileWithAstCompiler(`
    const fn make_values() -> [i32; 3] {
        [1, 2, 3]
    }

    fn main() -> i32 {
        let values: [i32; 3] = make_values();
        values[0] + values[1] + values[2]
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(6);
});

test("const functions with const parameters build array constants", async () => {
  const wasm = await compileWithAstCompiler(`
    const fn make_array(const COUNT: i32) -> [i32; COUNT] {
        [COUNT; COUNT]
    }

    const VALUES = make_array(3);

    fn main() -> i32 {
        VALUES[0] + VALUES[1] + VALUES[2]
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(9);
});

test("len intrinsic can be evaluated in const expressions", async () => {
  const wasm = await compileWithAstCompiler(`
    const LENGTH: i32 = len([1, 2, 3, 4]);

    fn main() -> i32 {
        LENGTH
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(4);
});

test("len intrinsic can be used from const functions", async () => {
  const wasm = await compileWithAstCompiler(`
    const fn array_len() -> i32 {
        len([7; 3])
    }

    const VALUE: i32 = array_len();

    fn main() -> i32 {
        VALUE
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(3);
});

test("const expressions can use if expressions", async () => {
  const wasm = await compileWithAstCompiler(`
    const VALUE: i32 = if true {
        40
    } else {
        0
    } + 2;

    fn main() -> i32 {
        VALUE
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(42);
});

test("constants can store types and be used in signatures", async () => {
  const wasm = await compileWithAstCompiler(`
    const Alias: type = Base;
    const Base: type = i32;

    fn identity(value: Alias) -> Base {
        let typed: Alias = value;
        typed
    }

    fn main() -> Alias {
        identity(42)
    }
  `);

  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(42);
});

test("const type aliases can use const expressions for array lengths", async () => {
  const wasm = await compileWithAstCompiler(`
    const BASE: i32 = 2;
    const LENGTH: i32 = BASE * 2;
    const Numbers: type = [i32; LENGTH];

    fn main() -> i32 {
        let values: Numbers = [1, 2, 3, 4];
        values[3]
    }
  `);

  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(4);
});

test("const type aliases can reference other const types in array elements", async () => {
  const wasm = await compileWithAstCompiler(`
    const Element: type = u8;
    const Numbers: type = [Element; 4];

    fn main() -> i32 {
        let values: Numbers = [1 as u8, 2 as u8, 3 as u8, 4 as u8];
        let second: Element = values[1];
        second as i32
    }
  `);

  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(2);
});

test("non-type constants cannot be used as type annotations", async () => {
  const failure = await expectCompileFailure(`
    const VALUE: i32 = 1;

    fn invalid(value: VALUE) -> i32 {
        value
    }
  `);
  expect(failure.failure.detail).toBe(
    "/entry.bp:4:23: type annotations require const type values",
  );
});

test("const functions can call other const functions", async () => {
  const wasm = await compileWithAstCompiler(`
    const fn base() -> i32 {
        40
    }

    const fn plus_two() -> i32 {
        base() + 2
    }

    const VALUE: i32 = plus_two();

    fn main() -> i32 {
        VALUE
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(42);
});

test("const functions cannot call non-const functions", async () => {
  const failure = await expectCompileFailure(`
    const fn call_helper() -> i32 {
        helper()
    }

    fn helper() -> i32 {
        7
    }
  `);
  expect(failure.failure.detail).toBe(
    "/entry.bp:3:9: const functions may only call const functions",
  );
});

test("function names cannot conflict with constants", async () => {
  const failure = await expectCompileFailure(`
    const helper: i32 = 1;

    fn helper() -> i32 {
        0
    }
  `);
  expect(failure.failure.detail).toBe(
    "/entry.bp:4:8: function name conflicts with constant",
  );
});

