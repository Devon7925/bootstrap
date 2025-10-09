#[path = "ast_compiler_helpers.rs"]
mod ast_compiler_helpers;

#[path = "wasm_harness.rs"]
mod wasm_harness;

use ast_compiler_helpers::compile_with_ast_compiler;
use wasm_harness::{instantiate_module, wasmtime_engine_with_gc};
use wasmtime::{Memory, TypedFunc};

#[test]
fn exports_multi_page_memory() {
    let source = r#"
fn slice_len(_ptr: i32, len: i32) -> i32 {
    len
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
        .expect("expected exported linear memory");
    let memory_bytes = memory.data_size(&store);
    assert!(memory_bytes >= 1048576);

    let slice_len: TypedFunc<(i32, i32), i32> = instance
        .get_typed_func(&mut store, "slice_len")
        .expect("expected exported slice_len function");
    let result = slice_len
        .call(&mut store, (0, 42))
        .expect("failed to execute main");
    assert_eq!(result, 42);
}

#[test]
fn reads_last_byte_from_input_slice() {
    let source = r#"
fn last_byte(ptr: i32, len: i32) -> i32 {
    if len == 0 {
        return -1;
    };

    let last: i32 = len - 1;
    load_u8(ptr + last)
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
        .expect("expected exported linear memory");

    let input = b"bootstrap";
    let offset = 32usize;
    memory
        .write(&mut store, offset, input)
        .expect("failed to write input into linear memory");

    let last_byte: TypedFunc<(i32, i32), i32> = instance
        .get_typed_func(&mut store, "last_byte")
        .expect("expected exported last_byte function");
    let result = last_byte
        .call(&mut store, (offset as i32, input.len() as i32))
        .expect("failed to execute main");

    assert_eq!(result as u8, *input.last().expect("non-empty slice"));
}

#[test]
fn writes_byte_into_memory() {
    let source = r#"
fn write_then_read(ptr: i32, value: i32) -> i32 {
    store_u8(ptr, value);
    load_u8(ptr)
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
        .expect("expected exported linear memory");

    let write_then_read: TypedFunc<(i32, i32), i32> = instance
        .get_typed_func(&mut store, "write_then_read")
        .expect("expected exported write_then_read function");

    let offset = 128i32;
    let value = 173i32;
    let result = write_then_read
        .call(&mut store, (offset, value))
        .expect("failed to execute main");

    assert_eq!(result as u8, value as u8);

    let mut buffer = [0u8; 1];
    memory
        .read(&store, offset as usize, &mut buffer)
        .expect("failed to read written byte");
    assert_eq!(buffer[0], value as u8);
}

#[test]
fn stores_and_loads_word_values() {
    let source = r#"
fn roundtrip_i32(ptr: i32, value: i32) -> i32 {
    store_i32(ptr, value);
    load_i32(ptr)
}

fn main() -> i32 {
    0
}
"#;

    let wasm = compile_with_ast_compiler(source);

    let engine = wasmtime_engine_with_gc();
    let (mut store, instance) = instantiate_module(&engine, &wasm);

    let roundtrip_i32: TypedFunc<(i32, i32), i32> = instance
        .get_typed_func(&mut store, "roundtrip_i32")
        .expect("expected exported roundtrip_i32 function");
    let i32_result = roundtrip_i32
        .call(&mut store, (256, 0x7fff_ff12))
        .expect("failed to execute roundtrip_i32");
    assert_eq!(i32_result, 0x7fff_ff12);
}

#[test]
fn stores_and_loads_halfword_values() {
    let source = r#"
fn roundtrip_u16(ptr: i32, value: i32) -> i32 {
    store_u16(ptr, value);
    load_u16(ptr)
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
        .expect("expected exported linear memory");

    let roundtrip_u16: TypedFunc<(i32, i32), i32> = instance
        .get_typed_func(&mut store, "roundtrip_u16")
        .expect("expected exported roundtrip_u16 function");

    let offset = 512i32;
    let value = 0xFE12i32;
    let result = roundtrip_u16
        .call(&mut store, (offset, value))
        .expect("failed to execute roundtrip_u16");
    assert_eq!(result, value & 0xffff);

    let mut buffer = [0u8; 2];
    memory
        .read(&store, offset as usize, &mut buffer)
        .expect("failed to read written halfword");
    assert_eq!(buffer[0], (value & 0xff) as u8);
    assert_eq!(buffer[1], ((value >> 8) & 0xff) as u8);
}
