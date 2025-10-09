import { expect, test } from "bun:test";

import { expectCompileFailure } from "./helpers";

test("program requires main", async () => {
  const failure = await expectCompileFailure(`
    fn helper() -> i32 {
        1
    }
  `);
  expect(failure.failure.producedLength).toBeLessThanOrEqual(0);
});

test("main cannot accept parameters", async () => {
  const failure = await expectCompileFailure(`
    fn main(value: i32) -> i32 {
        value
    }
  `);
  expect(failure.failure.producedLength).toBeLessThanOrEqual(0);
});

test.skip("main must return i32", async () => {
  const failure = await expectCompileFailure(`
    fn main() -> bool {
        true
    }
  `);
  expect(failure.failure.producedLength).toBeLessThanOrEqual(0);
});

test("main function name must be unique", async () => {
  const failure = await expectCompileFailure(`
    fn main() -> i32 {
        1
    }

    fn helper() -> i32 {
        2
    }

    fn main() -> i32 {
        3
    }
  `);
  expect(failure.failure.producedLength).toBeLessThanOrEqual(0);
});
