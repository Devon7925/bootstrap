import { describe, expect, test } from "bun:test";

import { compileWithAstCompiler, runWasmMainWithGc } from "./helpers";

describe("struct intrinsic with const type values", () => {
    test("constructs a static pair with dot and bracket access", async () => {
        const wasm = await compileWithAstCompiler(`
        const Pair = struct(6, 2, [
            ("first\\0", i32),
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

    test("use constants for struct type integers definition", async () => {
        const wasm = await compileWithAstCompiler(`
        const STR_LEN: i32 = 6;
        const PROP_COUNT: i32 = 2;
        const Pair = struct(STR_LEN, PROP_COUNT, [
            ("first\\0", i32),
            ("second", i32),
        ]);

        fn main() -> i32 {
            let pair: Pair = Pair {
                first: 1,
                second: 2,
            };
            pair.first + pair.second
        }
      `);
        const result = await runWasmMainWithGc(wasm);
        expect(result).toBe(3);
    });

    test("use constants for struct props definition", async () => {
        const wasm = await compileWithAstCompiler(`
        const PROP1: [u8; 6] = "first\\0";
        const Pair = struct(6, 2, [
            (PROP1, i32),
            ("second", i32),
        ]);

        fn main() -> i32 {
            let pair: Pair = Pair {
                first: 1,
                second: 2,
            };
            pair.first + pair.second
        }
      `);
        const result = await runWasmMainWithGc(wasm);
        expect(result).toBe(3);
    });

    test("use constants for struct array definition", async () => {
        const wasm = await compileWithAstCompiler(`
        const PairData: [([u8;6], type); 2] = [
            ("first\\0", i32),
            ("second", i32),
        ];
        const Pair = struct(6, 2, PairData);

        fn main() -> i32 {
            let pair: Pair = Pair {
                first: 1,
                second: 2,
            };
            pair.first + pair.second
        }
      `);
        const result = await runWasmMainWithGc(wasm);
        expect(result).toBe(3);
    });

    test("rejects struct literal labels that do not match canonical names", async () => {
        await expect(
            compileWithAstCompiler(`
            const SECOND: [u8; 6] = "second";
            const Pair = struct(6, 2, [
                ("first\\0", i32),
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
        ).rejects.toThrow("/entry.bp:10:21: struct literal field name does not match canonical field first");
    });

    test("struct literals reject missing fields", async () => {
        await expect(
            compileWithAstCompiler(`
            const Pair = struct(6, 2, [
                ("first\\0", i32),
                ("second", i32),
            ]);

            fn main() -> i32 {
                let pair: Pair = Pair {
                    first: 1,
                };
                pair.first
            }
          `),
        ).rejects.toThrow("/entry.bp:8:39: struct literal missing field second");
    });

    test("const fn can return struct data", async () => {
        const wasm = await compileWithAstCompiler(`
        const fn struct_data() -> [([u8;3], type); 12] {
            [
                ("k0\\0", i32),
                ("k1\\0", i32),
                ("k2\\0", i32),
                ("k3\\0", i32),
                ("k4\\0", i32),
                ("k5\\0", i32),
                ("k6\\0", i32),
                ("k7\\0", i32),
                ("k8\\0", i32),
                ("k9\\0", i32),
                ("k10", i32),
                ("k11", i32),
            ]
        }

        const TwelveKeys = struct(3, 12, struct_data());

        fn main() -> i32 {
            let set: TwelveKeys = TwelveKeys {
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

    test("const fn can return struct", async () => {
        const wasm = await compileWithAstCompiler(`
        const fn dynamic_struct() -> type {
            struct(3, 12, [
                ("k0\\0", i32),
                ("k1\\0", i32),
                ("k2\\0", i32),
                ("k3\\0", i32),
                ("k4\\0", i32),
                ("k5\\0", i32),
                ("k6\\0", i32),
                ("k7\\0", i32),
                ("k8\\0", i32),
                ("k9\\0", i32),
                ("k10", i32),
                ("k11", i32),
            ])
        }

        const TwelveKeys = dynamic_struct();

        fn main() -> i32 {
            let set: TwelveKeys = TwelveKeys {
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

    test("dynamic struct factories build property names programmatically", async () => {
        const wasm = await compileWithAstCompiler(`
        const KEY_COUNT: i32 = 12;
        const KEY_NAME_CAP: i32 = 4;

        const fn dynamic_struct(const COUNT: i32) -> type {
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
                }
                let ones = idx - tens * 10;
                entries[idx].0[place] = (48 + ones) as u8;
                idx = idx + 1;
            }
            struct(KEY_NAME_CAP, COUNT, entries)
        }

        const TwelveKeys = dynamic_struct(KEY_COUNT);

        fn main() -> i32 {
            let set: TwelveKeys = TwelveKeys {
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

    test("struct values can be stored inside arrays", async () => {
        const wasm = await compileWithAstCompiler(`
        const Pair = struct(6, 2, [
            ("first\\0", i32),
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
        expect(result).toBe(5);
    });

    test("struct types can contain other struct fields", async () => {
        const wasm = await compileWithAstCompiler(`
        const Pair = struct(6, 2, [
            ("first\\0", i32),
            ("second", i32),
        ]);

        const Wrapper = struct(6, 1, [
            ("inner\\0", Pair),
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

    test("incorrect parameter specialization raises diagnostic", async () => {
        await expect(
            compileWithAstCompiler(`
            const Pair = struct(4, 1, [
                ("first\\0", i32),
            ]);

            fn main() -> i32 { 0 }
          `),
        ).rejects.toThrow("/entry.bp:2:26: const parameter template expected type mismatch");
    });

    test("duplicate property names raise diagnostics", async () => {
        await expect(
            compileWithAstCompiler(`
            const Pair = struct(6, 2, [
                ("first\\0", i32),
                ("first\\0", i32),
            ]);

            fn main() -> i32 { 0 }
          `),
        ).rejects.toThrow("/entry.bp:1:1: duplicate struct field first");
    });

    test("struct intrinsic rejects non-array properties", async () => {
        await expect(
            compileWithAstCompiler(`
            const Pair = struct(6, 2, 42);

            fn main() -> i32 { 0 }
          `),
        ).rejects.toThrow(
            "/entry.bp:1:1: struct intrinsic properties must be an array of field tuples",
        );
    });

    test("const parameter functions can forward struct types", async () => {
        const wasm = await compileWithAstCompiler(`
        const fn identity(const T: type) -> type { T }

        const Pair = struct(6, 2, [
            ("first\\0", i32),
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

    test("passes struct value to runtime function", async () => {
        const wasm = await compileWithAstCompiler(`
        const Pair = struct(6, 2, [
            ("first\\0", i32),
            ("second", i32),
        ]);

        fn host_sum_pair(p: Pair) -> i32 {
            p.first + p.second
        }

        fn main() -> i32 {
            let pair: Pair = Pair { first: 20, second: 22 };
            host_sum_pair(pair)
        }
      `);
        const result = await runWasmMainWithGc(wasm);
        expect(result).toBe(42);
    });
});
