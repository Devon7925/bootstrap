#[path = "ast_compiler_helpers.rs"]
mod ast_compiler_helpers;

#[path = "wasm_harness.rs"]
mod wasm_harness;

use ast_compiler_helpers::{compile_with_ast_compiler, try_compile_with_ast_compiler};
use wasm_harness::{instantiate_module, wasmtime_engine_with_gc};
use wasmtime::{Memory, TypedFunc};

#[test]
fn integer_width_programs_execute() {
    let source = r#"
fn add_i8(a: i8, b: i8) -> i8 {
    let mut total: i8 = a;
    total = total + b;
    total
}

fn less_than_i8(a: i8, b: i8) -> bool {
    a < b
}

fn add_i16(a: i16, b: i16) -> i16 {
    let mut total: i16 = a;
    total = total + b;
    total
}

fn less_than_i16(a: i16, b: i16) -> bool {
    a < b
}

fn add_i64(a: i64, b: i64) -> i64 {
    a + b
}

fn less_than_i64(a: i64, b: i64) -> bool {
    a < b
}

fn add_u8(a: u8, b: u8) -> u8 {
    let mut total: u8 = a;
    total = total + b;
    total
}

fn max_u8(a: u8, b: u8) -> u8 {
    if a > b { a } else { b }
}

fn roundtrip_u8(ptr: i32, value: u8) -> u8 {
    store_u8(ptr, value);
    load_u8(ptr)
}

fn add_u16(a: u16, b: u16) -> u16 {
    let mut total: u16 = a;
    total = total + b;
    total
}

fn roundtrip_u16(ptr: i32, value: u16) -> u16 {
    store_u16(ptr, value);
    load_u16(ptr)
}

fn add_u32(a: u32, b: u32) -> u32 {
    a + b
}

fn add_u64(a: u64, b: u64) -> u64 {
    a + b
}

fn less_than_u64(a: u64, b: u64) -> bool {
    a < b
}

fn mix_call(a: i8, b: i16, c: u32, d: u64) -> u64 {
    let doubled_small: i8 = add_i8(a, a);
    let doubled_mid: i16 = add_i16(b, b);
    let doubled_mid_unsigned: u32 = add_u32(c, c);
    let doubled_large: u64 = add_u64(d, d);
    let mut result: u64 = d;

    if less_than_i16(doubled_mid, b) {
        result = add_u64(d, d);
    } else {
        if doubled_small < a {
            result = doubled_large;
        } else {
            if doubled_mid_unsigned > c {
                result = doubled_large;
            } else {
                result = d;
            };
        };
    };

    result
}

fn main() -> i32 {
    0
}
"#;

    let wasm = compile_with_ast_compiler(source);

    let engine = wasmtime_engine_with_gc();
    let (mut store, instance) = instantiate_module(&engine, &wasm);

    let memory: Memory = instance
        .get_memory(&mut store, "memory")
        .expect("expected exported memory");

    let add_i8_func: TypedFunc<(i32, i32), i32> = instance
        .get_typed_func(&mut store, "add_i8")
        .expect("expected add_i8 export");
    let less_than_i8_func: TypedFunc<(i32, i32), i32> = instance
        .get_typed_func(&mut store, "less_than_i8")
        .expect("expected less_than_i8 export");
    let add_i16_func: TypedFunc<(i32, i32), i32> = instance
        .get_typed_func(&mut store, "add_i16")
        .expect("expected add_i16 export");
    let less_than_i16_func: TypedFunc<(i32, i32), i32> = instance
        .get_typed_func(&mut store, "less_than_i16")
        .expect("expected less_than_i16 export");
    let add_i64_func: TypedFunc<(i64, i64), i64> = instance
        .get_typed_func(&mut store, "add_i64")
        .expect("expected add_i64 export");
    let less_than_i64_func: TypedFunc<(i64, i64), i32> = instance
        .get_typed_func(&mut store, "less_than_i64")
        .expect("expected less_than_i64 export");
    let add_u8_func: TypedFunc<(i32, i32), i32> = instance
        .get_typed_func(&mut store, "add_u8")
        .expect("expected add_u8 export");
    let max_u8_func: TypedFunc<(i32, i32), i32> = instance
        .get_typed_func(&mut store, "max_u8")
        .expect("expected max_u8 export");
    let roundtrip_u8_func: TypedFunc<(i32, i32), i32> = instance
        .get_typed_func(&mut store, "roundtrip_u8")
        .expect("expected roundtrip_u8 export");
    let add_u16_func: TypedFunc<(i32, i32), i32> = instance
        .get_typed_func(&mut store, "add_u16")
        .expect("expected add_u16 export");
    let roundtrip_u16_func: TypedFunc<(i32, i32), i32> = instance
        .get_typed_func(&mut store, "roundtrip_u16")
        .expect("expected roundtrip_u16 export");
    let add_u32_func: TypedFunc<(i32, i32), i32> = instance
        .get_typed_func(&mut store, "add_u32")
        .expect("expected add_u32 export");
    let add_u64_func: TypedFunc<(i64, i64), i64> = instance
        .get_typed_func(&mut store, "add_u64")
        .expect("expected add_u64 export");
    let less_than_u64_func: TypedFunc<(i64, i64), i32> = instance
        .get_typed_func(&mut store, "less_than_u64")
        .expect("expected less_than_u64 export");
    let mix_call_func: TypedFunc<(i32, i32, i32, i64), i64> = instance
        .get_typed_func(&mut store, "mix_call")
        .expect("expected mix_call export");

    let add_i8_result = add_i8_func
        .call(&mut store, (120, 5))
        .expect("failed to execute add_i8");
    assert_eq!(add_i8_result as i8, 125);

    let less_than_i8_result = less_than_i8_func
        .call(&mut store, (5, 9))
        .expect("failed to execute less_than_i8");
    assert_eq!(less_than_i8_result, 1);

    let add_i16_result = add_i16_func
        .call(&mut store, (3000, 1234))
        .expect("failed to execute add_i16");
    assert_eq!(add_i16_result as i16, 4234);

    let less_than_i16_result = less_than_i16_func
        .call(&mut store, (4000, 1999))
        .expect("failed to execute less_than_i16");
    assert_eq!(less_than_i16_result, 0);

    let add_i64_result = add_i64_func
        .call(&mut store, (1_000_000_000, 2_000_000_000))
        .expect("failed to execute add_i64");
    assert_eq!(add_i64_result, 3_000_000_000);

    let less_than_i64_result = less_than_i64_func
        .call(&mut store, (9_000_000_000, 1_000_000_000))
        .expect("failed to execute less_than_i64");
    assert_eq!(less_than_i64_result, 0);

    let add_u8_result = add_u8_func
        .call(&mut store, (200, 50))
        .expect("failed to execute add_u8");
    assert_eq!(add_u8_result as u8, 250);

    let max_u8_result = max_u8_func
        .call(&mut store, (17, 42))
        .expect("failed to execute max_u8");
    assert_eq!(max_u8_result as u8, 42);

    let u8_offset = 128;
    let u8_value = 0xABi32;
    let roundtrip_u8_result = roundtrip_u8_func
        .call(&mut store, (u8_offset, u8_value))
        .expect("failed to execute roundtrip_u8");
    assert_eq!(roundtrip_u8_result as u8, u8_value as u8);
    let mut u8_buffer = [0u8; 1];
    memory
        .read(&store, u8_offset as usize, &mut u8_buffer)
        .expect("failed to read stored u8");
    assert_eq!(u8_buffer[0], u8_value as u8);

    let add_u16_result = add_u16_func
        .call(&mut store, (1000, 2300))
        .expect("failed to execute add_u16");
    assert_eq!(add_u16_result as u16, 3300);

    let u16_offset = 256;
    let u16_value = 0xBEEF_i32;
    let roundtrip_u16_result = roundtrip_u16_func
        .call(&mut store, (u16_offset, u16_value))
        .expect("failed to execute roundtrip_u16");
    assert_eq!(roundtrip_u16_result as u16, u16_value as u16);
    let mut u16_buffer = [0u8; 2];
    memory
        .read(&store, u16_offset as usize, &mut u16_buffer)
        .expect("failed to read stored u16");
    assert_eq!(u16_buffer[0], (u16_value & 0xff) as u8);
    assert_eq!(u16_buffer[1], ((u16_value >> 8) & 0xff) as u8);

    let add_u32_result = add_u32_func
        .call(&mut store, (1_000_000, 2_000_000))
        .expect("failed to execute add_u32");
    assert_eq!(add_u32_result as u32, 3_000_000);

    let add_u64_result = add_u64_func
        .call(&mut store, (5, 7))
        .expect("failed to execute add_u64");
    assert_eq!(add_u64_result, 12);

    let less_than_u64_result = less_than_u64_func
        .call(&mut store, (99, 42))
        .expect("failed to execute less_than_u64");
    assert_eq!(less_than_u64_result, 0);

    let mix_call_result = mix_call_func
        .call(&mut store, (3, 10, 7, 9))
        .expect("failed to execute mix_call");
    assert_eq!(mix_call_result, 18);
}

#[test]
fn mixed_integer_widths_are_rejected_without_casts() {
    let source = r#"
fn main() -> i32 {
    let lhs: i16 = 12;
    let rhs: i64 = 34;
    if lhs < rhs { 0 } else { 1 }
}
"#;

    let error = try_compile_with_ast_compiler(source)
        .expect_err("integer width mismatches should be rejected");
    assert!(error.produced_len <= 0);
}

#[test]
fn signed_and_unsigned_mixes_require_casts() {
    let source = r#"
fn difference(a: u16, b: i16) -> u16 {
    a - b
}

fn main() -> i32 {
    0
}
"#;

    let error = try_compile_with_ast_compiler(source)
        .expect_err("signed/unsigned mixes without casts should be rejected");
    assert!(error.produced_len <= 0);
}

#[test]
fn integer_arguments_must_match_parameter_widths() {
    let source = r#"
fn take_i8(value: i8) -> i8 {
    value
}

fn main() -> i32 {
    let sample: i16 = 10;
    let _ = take_i8(sample);
    0
}
"#;

    let error = try_compile_with_ast_compiler(source)
        .expect_err("calls with mismatched integer widths should be rejected");
    assert!(error.produced_len <= 0);
}
