import { expect, test } from "bun:test";

import { compileWithAstCompiler, runWasmMainWithGc } from "./helpers";

test("integer casts execute", async () => {
  const wasm = await compileWithAstCompiler(`
    fn main() -> i32 {
        let mut score: i32 = 0;

        let neg_i8: i8 = -1;
        let neg_as_u8: u8 = neg_i8 as u8;
        if neg_as_u8 as i32 == 255 {
            score = score + 1;
            0
        } else {
            0
        };

        let byte_value: u8 = 255;
        let byte_as_i16: i16 = byte_value as i16;
        if byte_as_i16 as i32 == 255 {
            score = score + 1;
            0
        } else {
            0
        };

        let neg_i16: i16 = -300;
        let neg_as_u16: u16 = neg_i16 as u16;
        if neg_as_u16 as i32 == 65236 {
            score = score + 1;
            0
        } else {
            0
        };

        let roundtrip_i16: i16 = neg_as_u16 as i16;
        if roundtrip_i16 as i32 == -300 {
            score = score + 1;
            0
        } else {
            0
        };

        let neg_i64: i64 = (-1 as i64);
        let neg_i64_as_u8: u8 = neg_i64 as u8;
        if neg_i64_as_u8 as i32 == 255 {
            score = score + 1;
            0
        } else {
            0
        };

        let small_u8: u8 = 128;
        let small_as_i64: i64 = small_u8 as i64;
        if small_as_i64 as i32 == 128 {
            score = score + 1;
            0
        } else {
            0
        };

        let neg_small_i16: i16 = -1234;
        let neg_small_i64: i64 = neg_small_i16 as i64;
        if neg_small_i64 as i32 == -1234 {
            score = score + 1;
            0
        } else {
            0
        };

        let trunc_i64: i64 = (512 as i64);
        let trunc_as_u8: u8 = trunc_i64 as u8;
        if trunc_as_u8 as i32 == 0 {
            score = score + 1;
            0
        } else {
            0
        };

        let small_i32: i32 = -1025;
        let small_as_u16: u16 = small_i32 as u16;
        if small_as_u16 as i32 == 64511 {
            score = score + 1;
            0
        } else {
            0
        };

        score
    }
  `);

  const result = await runWasmMainWithGc(wasm);
  expect(result).toBe(9);
});
