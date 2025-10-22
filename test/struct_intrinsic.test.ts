import { describe, expect, test } from "bun:test";

import { compileWithAstCompiler, expectCompileFailure, runWasmMainWithGc } from "./helpers";

describe("struct intrinsic with const type values", () => {
    test("registers struct types when field names are canonical", async () => {
        const wasm = await compileWithAstCompiler(`
        const FIRST: [u8; 6] = [102, 105, 114, 115, 116, 0];
        const SECOND: [u8; 6] = [115, 101, 99, 111, 110, 0];

        const Pair = struct(6, 2, [
            (FIRST, i32),
            (SECOND, i32),
        ]);

        fn main() -> i32 { 0 }
      `);
        expect(wasm.byteLength).toBeGreaterThan(0);
    });

    test("struct intrinsic rejects field names without null terminators", async () => {
        const failure = await expectCompileFailure(`
        const FIRST_BAD: [u8; 6] = [102, 105, 114, 115, 116, 33];

        const Pair = struct(6, 1, [
            (FIRST_BAD, i32),
        ]);

        fn main() -> i32 { 0 }
      `);
        expect(failure.failure.detail).toContain("struct field names must be null terminated");
    });

    test.todo("constructs a static pair with dot and bracket access", async () => {
        const wasm = await compileWithAstCompiler(`
        const Pair = struct(6, 2, [
            ("first\0", i32),
            ("second", i32),
        ]);

        fn main() -> i32 {
            let pair: Pair = Pair {
                first: 1,
                second: 2,
            };
            pair.first + pair.second + pair["second"]
        }
      `);
        const result = await runWasmMainWithGc(wasm);
        expect(result).toBe(5);
    });

    test.todo("rejects struct literal labels that do not match canonical names", async () => {
        await expect(
            compileWithAstCompiler(`
            const SECOND: [u8; 6] = "second";
            const Pair = struct(6, 2, [
                ("first\0", i32),
                (SECOND, i32),
            ]);

            fn main() -> i32 {
                let pair: Pair = Pair {
                    ["fir"]: 1,
                    second: 2,
                };
                pair.first
            }
          `),
        ).rejects.toThrow("/entry.bp:8:21: struct literal field name does not match canonical field first");
    });

    test.todo("struct literals reject missing fields", async () => {
        await expect(
            compileWithAstCompiler(`
            const Pair = struct(6, 2, [
                ("first\0", i32),
                ("second", i32),
            ]);

            fn main() -> i32 {
                let pair: Pair = Pair {
                    first: 1,
                };
                pair.first
            }
          `),
        ).rejects.toThrow("/entry.bp:7:17: struct literal missing field second");
    });

    test.todo("dynamic struct factories build property names programmatically", async () => {
        const wasm = await compileWithAstCompiler(`
        const fn digits(num: i32) -> i32 {
            let mut digits = 0;
            while num > 0 {
                num = num / 10;
                digits = digits + 1;
            }
            digits
        }

        const fn dynamic_struct(const KEY_COUNT: i32) -> type {
            let mut entries: [([u8; digits(KEY_COUNT) + 1], type); KEY_COUNT] =
                [([0; digits(KEY_COUNT) + 1], i32); KEY_COUNT];
            let mut idx = 0;
            while idx < KEY_COUNT {
                entries[idx].0[0] = 'k';
                let mut digit = digits(KEY_COUNT) - 1;
                while digit >= 0 {
                    let mut digit_val = idx;
                    let mut digit_idx = 0;
                    while digit_idx <= digit {
                        digit_val = digit_val / 10;
                        digit_idx = digit_idx + 1;
                    }
                    digit_val = digit_val - digit_val / 10 * 10;
                    entries[idx].0[digit + 1] = 48 + digit_val;
                    digit = digit - 1;
                }
                idx = idx + 1;
            }
            struct(digits(KEY_COUNT) + 1, KEY_COUNT, entries)
        }

        const ElevenKeys = dynamic_struct(11);

        fn main() -> i32 {
            let set: ElevenKeys = ElevenKeys {
                k0: 0,
                k1: 1,
                k2: 2,
                k3: 3,
                k4: 4,
                k5: 5,
                k6: 6,
                k7: 7,
                k8: 8,
                k9: 9,
                k10: 10,
                k11: 11,
            };
            set.k1 + set.k11 + 3 * set.k10
        }
      `);
        const result = await runWasmMainWithGc(wasm);
        expect(result).toBe(42);
    });

    test.todo("struct values can be stored inside arrays", async () => {
        const wasm = await compileWithAstCompiler(`
        const Pair = struct(6, 2, [
            ("first\0", i32),
            ("second", i32),
        ]);

        fn main() -> i32 {
            let values: [Pair; 2] = [
                Pair { first: 1, second: 2 },
                Pair { first: 3, second: 4 },
            ];
            values[0].first + values[1].second
        }
      `);
        const result = await runWasmMainWithGc(wasm);
        expect(result).toBe(6);
    });

    test.todo("struct types can contain other struct fields", async () => {
        const wasm = await compileWithAstCompiler(`
        const Pair = struct(6, 2, [
            ("first\0", i32),
            ("second", i32),
        ]);

        const Wrapper = struct(6, 1, [
            ("inner\0", Pair),
        ]);

        fn main() -> i32 {
            let wrapper: Wrapper = Wrapper {
                inner: Pair { first: 12, second: 30 },
            };
            wrapper.inner.first + wrapper.inner.second
        }
      `);
        const result = await runWasmMainWithGc(wasm);
        expect(result).toBe(42);
    });

    test.todo("duplicate property names raise diagnostics", async () => {
        await expect(
            compileWithAstCompiler(`
            const Pair = struct(6, 2, [
                ("first\0", i32),
                ("first\0", i32),
            ]);

            fn main() -> i32 { 0 }
          `),
        ).rejects.toThrow("/entry.bp:4:17: duplicate struct field first");
    });

    test.todo("const parameter functions can forward struct types", async () => {
        const wasm = await compileWithAstCompiler(`
        const fn identity(const T: type) -> type { T }

        const Pair = struct(6, 2, [
            ("first\0", i32),
            ("second", i32),
        ]);

        const PairAlias = identity(Pair);

        fn main() -> i32 {
            let pair: PairAlias = Pair { first: 21, second: 21 };
            pair.first + pair.second
        }
      `);
        const result = await runWasmMainWithGc(wasm);
        expect(result).toBe(42);
    });
});
