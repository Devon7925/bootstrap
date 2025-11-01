import { expect, test } from "bun:test";

import {
  compileWithAstCompiler,
  expectCompileFailure,
  runWasmMainWithGc,
} from "./helpers";

test("anonymous function literals cannot initialize constants yet", async () => {
  const failure = await expectCompileFailure(`
    const HANDLER: fn(i32) -> i32 = fn(x: i32) -> i32 { x };
  `);
  expect(failure.failure.detail).toBe("type metadata resolution failed");
});

test("anonymous function metadata tracks parameter diagnostics", async () => {
  const failure = await expectCompileFailure(`
    const HANDLER: fn(i32, i32) -> i32 = fn(x: i32, x: i32) -> i32 { x };
  `);
  expect(failure.failure.detail).toBe("/entry.bp:2:53: duplicate parameter name");
});

test("evaluates inline anonymous function via const parameter", async () => {
  const wasm = await compileWithAstCompiler(`
    fn map_pair(const F: fn(i32) -> i32, lhs: i32, rhs: i32) -> (i32, i32) {
        (F(lhs), F(rhs))
    }

    fn main() -> i32 {
        let pair = map_pair(fn(x: i32) -> i32 { x + x }, 4, 7);
        pair.0 + pair.1
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(22);
});

test.todo("const fn factories can return anonymous functions with const parameters", async () => {
  const wasm = await compileWithAstCompiler(`
    const fn make_incrementer() -> fn(const i32) -> i32 {
        fn(const x: i32) -> i32 { x + 1 }
    }

    fn apply(const F: fn(const i32) -> i32, const value: i32) -> i32 {
        F(value)
    }

    fn main() -> i32 {
        apply(make_incrementer(), 41)
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(42);
});

test.todo("const fn factories can return anonymous functions without const parameters", async () => {
  const wasm = await compileWithAstCompiler(`
    const fn make_incrementer() -> fn(i32) -> i32 {
        fn(const x: i32) -> i32 { x + 1 }
    }

    fn apply(const F: fn(i32) -> i32, value: i32) -> i32 {
        F(value)
    }

    fn main() -> i32 {
        apply(make_incrementer(), 41)
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(42);
});

test("non-const parameters cannot accept anonymous functions", async () => {
  const failure = await expectCompileFailure(`
    fn apply(handler: fn(i32) -> i32, value: i32) -> i32 {
        handler(value)
    }

    fn main() -> i32 {
        apply(fn(x: i32) -> i32 { x + 1 }, 5)
    }
  `);
  expect(failure.failure.detail).toContain("function values are only permitted in const contexts");
});

test("returning anonymous functions enforces const signatures", async () => {
  const failure = await expectCompileFailure(`
    const fn make_identity() -> fn(x: i32) -> i32 {
        fn(x: i32) -> i32 { x }
    }
  `);
  expect(failure.failure.detail).toContain(
    "anonymous functions returned from const fn must accept only const parameters",
  );
});

test("anonymous functions remain const-only values", async () => {
  const failure = await expectCompileFailure(`
    fn main() -> i32 {
        let runtime = fn(x: i32) -> i32 { x };
        runtime(3)
    }
  `);
  expect(failure.failure.detail).toContain(
    "function values are only permitted in const contexts",
  );
});

test("anonymous functions may not capture non-const locals", async () => {
  const failure = await expectCompileFailure(`
    fn main() -> i32 {
        let delta = 1;
        const HANDLER: fn(i32) -> i32 = fn(x: i32) -> i32 { x + delta };
        HANDLER(5)
    }
  `);
  expect(failure.failure.detail).toContain("parsing source failed");
});

test.todo("const arrays can store anonymous function literals", async () => {
  const wasm = await compileWithAstCompiler(`
    const HANDLERS: [fn(i32) -> i32; 2] = [
        fn(x: i32) -> i32 { x + 1 },
        fn(x: i32) -> i32 { x - 1 },
    ];

    fn main() -> i32 {
        HANDLERS[0](10) + HANDLERS[1](10)
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(20);
});
