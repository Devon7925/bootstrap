import { expect, test } from "bun:test";

import { compileWithAstCompiler, expectCompileFailure, runWasmMainWithGc } from "./helpers";

test("loops and break execute", async () => {
  const wasm = await compileWithAstCompiler(`
    fn loop_sum(limit: i32) -> i32 {
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

    fn main() -> i32 {
        let mut count: i32 = 0;
        let mut total: i32 = 0;
        loop {
            if count >= 5 {
                break;
            };
            total = total + loop_sum(count);
            count = count + 1;
        }
        total
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(10);
});

test("continue skips iterations", async () => {
  const wasm = await compileWithAstCompiler(`
    fn sum_even(limit: i32) -> i32 {
        let mut acc: i32 = 0;
        let mut i: i32 = 0;
        loop {
            if i >= limit {
                break;
            };
            i = i + 1;
            let remainder: i32 = i - (i / 2) * 2;
            if remainder == 1 {
                continue;
            };
            acc = acc + i;
        }
        acc
    }

    fn loop_skip() -> i32 {
        let mut total: i32 = 0;
        let mut i: i32 = 0;
        loop {
            i = i + 1;
            if i > 5 {
                break;
            };
            if i == 3 {
                continue;
            };
            total = total + i;
        }
        total
    }

    fn main() -> i32 {
        sum_even(6) + loop_skip()
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(24);
});

test("loop can break with values", async () => {
  const wasm = await compileWithAstCompiler(`
    fn find_first_even(limit: i32) -> i32 {
        let mut candidate: i32 = 0;
        let mut result: i32 = -1;
        loop {
            candidate = candidate + 1;
            if candidate >= limit {
                break;
            };
            candidate = candidate + 1;
            if candidate >= limit {
                break;
            };
            result = candidate;
            break;
        }
        result
    }

    fn main() -> i32 {
        find_first_even(10)
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(2);
});

test("loop break types must match", async () => {
  const wasm = await compileWithAstCompiler(`
    fn bad() -> i32 {
        loop {
            if true {
                return 5;
            };
            break;
        }
        0
    }

    fn main() -> i32 {
        bad()
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(5);
});

test("else if chains execute", async () => {
  const wasm = await compileWithAstCompiler(`
    fn describe(value: i32) -> i32 {
        if value < 0 {
            -1
        } else if value == 0 {
            0
        } else if value == 1 {
            1
        } else {
            2
        }
    }

    fn main() -> i32 {
        describe(-3) + describe(0) * 10 + describe(1) * 100 + describe(5) * 1000
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(2099);
});

test.todo("if branches with mismatched types report precise diagnostics", async () => {
  const failure = await expectCompileFailure(`
    fn mismatched(flag: bool) -> i32 {
        if flag {
            1
        } else {
            false
        }
    }

    fn main() -> i32 {
        mismatched(true)
    }
  `);
  expect(failure.failure.detail).toMatch(/if (branch|branches) type mismatch/);
});

test("loop allows final if without semicolon", async () => {
  const wasm = await compileWithAstCompiler(`
    fn loop_with_final_if(limit: i32) -> i32 {
        let mut value: i32 = 0;
        loop {
            if value >= limit {
                break;
            };
            value = value + 1;
            if value == limit {
                break;
            }
        }
        value
    }

    fn main() -> i32 {
        loop_with_final_if(4)
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(4);
});

test("loop expressions can initialize locals", async () => {
  const wasm = await compileWithAstCompiler(`
    fn main() -> i32 {
        let value: i32 = loop {
            break 5;
        };
        value
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(5);
});

test("while break cannot carry values", async () => {
  const failure = await expectCompileFailure(`
    fn break_with_value() {
        while (false) {
            break 1;
        }
    }
  `);
  expect(failure.failure.detail).toBe(
    "/entry.bp:4:13: while loops cannot break with values",
  );
});

test("loop and break support truthy conditions", async () => {
  const wasm = await compileWithAstCompiler(`
    fn sum_up_to(limit: i32) -> i32 {
        let mut total: i32 = 0;
        let mut count: i32 = 0;
        let mut remaining: i32 = limit;
        loop {
            if remaining {
                total = total + count;
                count = count + 1;
                remaining = remaining - 1;
                0
            } else {
                break;
                0
            };
        }
        total
    }

    fn main() -> i32 {
        sum_up_to(5)
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(10);
});

test("predicate calls in loops execute", async () => {
  const wasm = await compileWithAstCompiler(`
    fn predicate(value: i32) -> bool {
        if value >= 3 {
            true
        } else {
            false
        }
    }

    fn main() -> i32 {
        let mut value: i32 = 0;
        loop {
            if predicate(value) {
                break;
            };
            value = value + 1;
        }
        value
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(3);
});

test("if statements inside blocks execute", async () => {
  const wasm = await compileWithAstCompiler(`
    fn adjust(input: i32) -> i32 {
        let mut value: i32 = input;
        if value > 0 {
            value = value - 1;
        }
        if value < 0 {
            value = 0;
        }
        value
    }

    fn main() -> i32 {
        adjust(2) + adjust(-1)
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(1);
});

test("if statements with else branches do not require semicolons", async () => {
  const wasm = await compileWithAstCompiler(`
    fn branch(flag: bool) -> i32 {
        let mut result: i32 = 0;
        if flag {
            result = result + 1;
        } else {
            result = result + 2;
        }
        result
    }

    fn main() -> i32 {
        branch(true) + branch(false)
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(3);
});

test("loop breaks can return values", async () => {
  const wasm = await compileWithAstCompiler(`
    fn choose() -> i32 {
        loop {
            break 42;
        }
    }

    fn main() -> i32 {
        choose()
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(42);
});

test("continue outside loop is rejected", async () => {
  const failure = await expectCompileFailure(`
    fn continue_outside_loop() -> i32 {
        continue;
        0
    }
  `);
  expect(failure.failure.detail).toBe(
    "/entry.bp:3:9: continue statements must be inside loops",
  );
});

test("nested loops can break with values", async () => {
  const wasm = await compileWithAstCompiler(`
    fn nested(limit: i32) -> i32 {
        let mut outer: i32 = limit;
        let mut total: i32 = 0;
        loop {
            if outer {
                let mut inner: i32 = outer;
                loop {
                    if inner {
                        total = total + outer;
                        inner = inner - 1;
                        0
                    } else {
                        break;
                        0
                    };
                }
                outer = outer - 1;
                0
            } else {
                break total;
                0
            };
        }
    }

    fn main() -> i32 {
        nested(3)
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(14);
});

test("diverging if tail statements are allowed", async () => {
  const wasm = await compileWithAstCompiler(`
    fn branch(flag: bool) -> i32 {
        if flag {
            return 10;
        } else {
            return 20;
        };
    }

    fn main() -> i32 {
        branch(true)
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(10);
});

test("break outside loop is rejected", async () => {
  const failure = await expectCompileFailure(`
    fn break_outside_loop() -> i32 {
        break;
        0
    }
  `);
  expect(failure.failure.detail).toBe(
    "/entry.bp:3:9: break statements must be inside loop",
  );
});

test("if with literal condition executes", async () => {
  const wasm = await compileWithAstCompiler(`
    fn main() -> i32 {
        if 1 {
            42
        } else {
            0
        }
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(42);
});

test("if else with parameter condition executes", async () => {
  const wasm = await compileWithAstCompiler(`
    fn choose(flag: i32) -> i32 {
        if flag {
            10
        } else {
            20
        }
    }

    fn main() -> i32 {
        choose(0)
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(20);
});

test("while loops execute", async () => {
  const wasm = await compileWithAstCompiler(`
    fn main() -> i32 {
        let mut total: i32 = 0;
        let mut value: i32 = 0;
        while value < 4 {
            total = total + value;
            value = value + 1;
        }
        total
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(6);
});

test("while loops support continue", async () => {
  const wasm = await compileWithAstCompiler(`
    fn main() -> i32 {
        let mut total: i32 = 0;
        let mut value: i32 = 0;
        while value < 6 {
            value = value + 1;
            if value == 3 {
                continue;
            };
            total = total + value;
        }
        total
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(18);
});

test("while loops reject break values", async () => {
  const failure = await expectCompileFailure(`
    fn attempt() {
        while true {
            break 1;
        }
    }
  `);
  expect(failure.failure.detail).toBe(
    "/entry.bp:4:13: while loops cannot break with values",
  );
});

test("nested if expressions execute", async () => {
  const wasm = await compileWithAstCompiler(`
    fn pick(a: i32, b: i32) -> i32 {
        if a {
            if b {
                1
            } else {
                2
            }
        } else {
            if b {
                3
            } else {
                4
            }
        }
    }

    fn main() -> i32 {
        pick(0, 1) + pick(1, 0) * 10
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(23);
});

test("else if chains cover all branches", async () => {
  const wasm = await compileWithAstCompiler(`
    fn classify(value: i32) -> i32 {
        if value < 0 {
            1
        } else if value == 0 {
            2
        } else {
            3
        }
    }

    fn main() -> i32 {
        classify(-2) + classify(0) * 10 + classify(5) * 100
    }
  `);
  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(321);
});

test("parser reports detail for incomplete if condition", async () => {
  const failure = await expectCompileFailure(`
    fn main() -> i32 {
        if (
    }
  `);
  expect(failure.failure.detail).toBe(
    "/entry.bp:3:12: if expression condition parse failed",
  );
});
