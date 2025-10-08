#[path = "ast_compiler_helpers.rs"]
mod ast_compiler_helpers;

#[path = "wasm_harness.rs"]
mod wasm_harness;

use ast_compiler_helpers::ast_compiler_wasm;
use wasm_harness::CompilerInstance;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValueType {
    I32,
    I64,
    Ref { nullable: bool, heap_type: i32 },
    Other(i32),
}

fn read_u32_leb(bytes: &[u8], idx: &mut usize) -> u32 {
    let mut result: u32 = 0;
    let mut shift: u32 = 0;
    loop {
        let byte = bytes[*idx];
        *idx += 1;
        result |= u32::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            break;
        }
        shift += 7;
    }
    result
}

fn read_i32_leb(bytes: &[u8], idx: &mut usize) -> i32 {
    let mut result: i32 = 0;
    let mut shift: i32 = 0;
    let mut byte: u8;
    loop {
        byte = bytes[*idx];
        *idx += 1;
        result |= ((byte & 0x7f) as i32) << shift;
        shift += 7;
        if byte & 0x80 == 0 {
            break;
        }
    }
    if shift < 32 && (byte & 0x40) != 0 {
        result |= !0 << shift;
    }
    result
}

fn read_value_type(bytes: &[u8], idx: &mut usize) -> ValueType {
    let code = read_i32_leb(bytes, idx);
    match code {
        0x7f | -0x01 => ValueType::I32,
        0x7e | -0x02 => ValueType::I64,
        -0x1c => {
            let heap_type = read_i32_leb(bytes, idx);
            ValueType::Ref {
                nullable: false,
                heap_type,
            }
        }
        -0x1d => {
            let heap_type = read_i32_leb(bytes, idx);
            ValueType::Ref {
                nullable: true,
                heap_type,
            }
        }
        other => ValueType::Other(other),
    }
}

fn find_section<'a>(bytes: &'a [u8], target_id: u8) -> Option<&'a [u8]> {
    if bytes.len() < 8 {
        return None;
    }
    let mut idx = 8; // skip magic and version
    while idx < bytes.len() {
        let section_id = bytes[idx];
        idx += 1;
        let payload_len = read_u32_leb(bytes, &mut idx) as usize;
        if idx + payload_len > bytes.len() {
            return None;
        }
        if section_id == target_id {
            let start = idx;
            let end = idx + payload_len;
            return Some(&bytes[start..end]);
        }
        idx += payload_len;
    }
    None
}

#[test]
fn array_types_emit_gc_entries() {
    let source = r#"
        fn accepts(arg: [i32; 4]) -> i32 {
            0
        }

        fn main() -> i32 {
            0
        }
    "#;

    let mut compiler = CompilerInstance::new(ast_compiler_wasm());
    let mut input_cursor = 0usize;
    let mut output_cursor = 1024i32;
    let wasm = compiler
        .compile_with_layout(&mut input_cursor, &mut output_cursor, source)
        .expect("program should compile with array types");
    assert!(wasm.len() > 8, "wasm output should include header");

    let type_section = find_section(&wasm, 1).expect("type section");
    let mut idx = 0usize;
    let type_count = read_u32_leb(type_section, &mut idx);
    assert_eq!(type_count, 3, "expected one array type plus two functions");

    let array_tag = read_i32_leb(type_section, &mut idx);
    assert_eq!(array_tag, -0x22, "array type should use gc composite opcode");
    let element_type = read_value_type(type_section, &mut idx);
    assert_eq!(element_type, ValueType::I32, "array element should be i32");
    let mutability = type_section[idx];
    idx += 1;
    assert_eq!(mutability, 1, "array fields should be mutable");

    let func0_tag = read_i32_leb(type_section, &mut idx);
    assert_eq!(func0_tag, -0x20, "function type opcode");
    let func0_params = read_u32_leb(type_section, &mut idx);
    assert_eq!(func0_params, 1, "accepts should take one parameter");
    let func0_param_type = read_value_type(type_section, &mut idx);
    match func0_param_type {
        ValueType::Ref { nullable, heap_type } => {
            assert!(!nullable, "array parameters should be non-null refs");
            assert_eq!(heap_type, 0, "array heap type index should be 0");
        }
        other => panic!("unexpected parameter type: {other:?}"),
    }
    let func0_results = read_u32_leb(type_section, &mut idx);
    assert_eq!(func0_results, 1, "function should return a value");
    let func0_result_type = read_value_type(type_section, &mut idx);
    assert_eq!(func0_result_type, ValueType::I32, "result should be i32");

    let func1_tag = read_i32_leb(type_section, &mut idx);
    assert_eq!(func1_tag, -0x20, "main should use function type encoding");
    let func1_params = read_u32_leb(type_section, &mut idx);
    assert_eq!(func1_params, 0, "main should have no parameters");
    let func1_results = read_u32_leb(type_section, &mut idx);
    assert_eq!(func1_results, 1, "main should return i32");
    let func1_result_type = read_value_type(type_section, &mut idx);
    assert_eq!(func1_result_type, ValueType::I32, "main result should be i32");
    assert_eq!(idx, type_section.len(), "type section should be fully consumed");

    let function_section = find_section(&wasm, 3).expect("function section");
    let mut fidx = 0usize;
    let func_decl_count = read_u32_leb(function_section, &mut fidx);
    assert_eq!(func_decl_count, 2, "expected two function declarations");
    let accepts_type_index = read_u32_leb(function_section, &mut fidx);
    assert_eq!(accepts_type_index, 1, "first function should follow array type");
    let main_type_index = read_u32_leb(function_section, &mut fidx);
    assert_eq!(main_type_index, 2, "second function type index should follow");
    assert_eq!(fidx, function_section.len(), "function section fully consumed");
}
