use std::fs;

use bootstrap::compile;
use bootstrap::lexer::Lexer;
use bootstrap::parser::Parser;

#[path = "wasm_harness.rs"]
mod wasm_harness;

use wasm_harness::{CompileFailure, CompilerInstance};

fn prepare_stage1_compiler() -> (CompilerInstance, String) {
    let stage1_source =
        fs::read_to_string("examples/stage1_minimal.bp").expect("failed to load stage1 source");

    let stage1_compilation = compile(&stage1_source).expect("failed to compile stage1 source");
    let stage1_wasm = stage1_compilation
        .to_wasm()
        .expect("failed to encode stage1 wasm");

    (CompilerInstance::new(stage1_wasm.as_slice()), stage1_source)
}

const STAGE1_FUNCTION_ENTRY_SIZE: usize = 32;
const STAGE1_FUNCTIONS_BASE_OFFSET: usize = 851968;
const STAGE1_MAX_FUNCTIONS: usize = 512;

fn stage1_output_ptr(compiler: &CompilerInstance) -> i32 {
    let memory_size = compiler.memory_size_bytes();
    let reserved = STAGE1_FUNCTIONS_BASE_OFFSET
        + STAGE1_MAX_FUNCTIONS * STAGE1_FUNCTION_ENTRY_SIZE;
    assert!(
        memory_size > reserved,
        "stage1 memory must exceed reserved layout"
    );
    (memory_size - reserved) as i32
}

#[test]
fn stage1_compiler_identifies_remaining_bootstrap_blocker() {
    let (mut stage1, stage1_source) = prepare_stage1_compiler();
    let output_ptr = stage1_output_ptr(&stage1);
    assert!(
        stage1_source.len() < output_ptr as usize,
        "stage1 source must not overlap output buffer"
    );

    // Compile the stage1 source with the stage1 compiler itself to produce stage2.
    let result = stage1.compile_at(0, output_ptr, &stage1_source);
    match result {
        Ok(_) => {
            panic!("stage1 unexpectedly compiled itself without encountering bootstrap blockers")
        }
        Err(CompileFailure {
            produced_len,
            functions,
            instr_offset,
            compiled_functions,
        }) => {
            eprintln!(
                "stage2 blocker debug: produced_len={produced_len} functions={functions} instr_offset={instr_offset} compiled_functions={compiled_functions}"
            );
            assert_eq!(produced_len, -1);
            assert!(
                instr_offset > 0,
                "stage1 should advance code generation before failing"
            );
            assert_eq!(
                compiled_functions, 117,
                "stage1 currently stops compiling at function index 117"
            );

            let tokens = Lexer::new(&stage1_source)
                .collect::<Result<Vec<_>, _>>()
                .expect("lex stage1 source");
            let mut parser = Parser::new(&tokens, &stage1_source);
            let program = parser.parse_program().expect("parse stage1 source");
            let total_functions = program.functions.len() as i32;

            assert!(functions > 0, "expected to register at least one function");
            assert_eq!(
                functions, total_functions,
                "expected to register all functions"
            );

            let failing_function = &program.functions[compiled_functions as usize];
            assert_eq!(
                failing_function.name, "register_function_signatures",
                "stage1 now fails while compiling register_function_signatures (function responsible for collecting all function signatures)"
            );
        }
    }
}

// When stage1 fails we report the first function that still needs code generation
// support in order to reach full stage2 bootstrapping.

#[test]
fn stage1_compiler_accepts_break_with_value_statements() {
    let (mut stage1, _) = prepare_stage1_compiler();
    let output_ptr = stage1_output_ptr(&stage1);

    let source = r#"
fn main() -> i32 {
    let mut counter: i32 = 0;
    loop {
        if counter > 3 {
            break counter;
        };
        counter = counter + 1;
    };
    0
}
"#;

    compile(source).expect("host compiler should accept break-with-value");

    stage1
        .compile_at(0, output_ptr, source)
        .expect("stage1 should accept break-with-value");
}

#[test]
fn stage1_compiler_accepts_loop_expression_results() {
    let (mut stage1, _) = prepare_stage1_compiler();
    let output_ptr = stage1_output_ptr(&stage1);

    let source = r#"
fn main() -> i32 {
    loop {
        break 7;
    }
}
"#;

    compile(source).expect("host compiler should accept loop expressions with values");

    stage1
        .compile_at(0, output_ptr, source)
        .expect("stage1 should accept loop expression result");
}

#[test]
fn stage1_compiler_accepts_else_if_chains() {
    let (mut stage1, _) = prepare_stage1_compiler();
    let output_ptr = stage1_output_ptr(&stage1);

    let source = r#"
fn main() -> i32 {
    let mut value: i32 = 3;
    if value == 3 {
        value = 1;
    } else if value == 4 {
        value = 2;
    };
    value
}
"#;

    compile(source).expect("host compiler should accept else-if chains");

    stage1
        .compile_at(0, output_ptr, source)
        .expect("stage1 should accept else-if chains");
}

#[test]
fn stage1_compiler_accepts_else_if_inside_loops() {
    let (mut stage1, _) = prepare_stage1_compiler();
    let output_ptr = stage1_output_ptr(&stage1);

    let source = r#"
fn main() -> i32 {
    let mut total: i32 = 0;
    let mut idx: i32 = 0;
    loop {
        if idx >= 3 {
            break;
        };
        if idx == 0 {
            total = total + 1;
        } else if idx == 1 {
            total = total + 2;
        };
        idx = idx + 1;
    };
    total
}
"#;

    compile(source).expect("host compiler should accept else-if chains inside loops");

    stage1
        .compile_at(0, output_ptr, source)
        .expect("stage1 should accept else-if chains inside loops");
}

#[test]
fn stage1_compiler_accepts_else_if_with_followup_else() {
    let (mut stage1, _) = prepare_stage1_compiler();
    let output_ptr = stage1_output_ptr(&stage1);

    let source = r#"
fn main() -> i32 {
    let func_count: i32 = 2;
    let mut idx: i32 = 0;
    loop {
        if idx >= func_count {
            break;
        };
        let return_type: i32 = idx;
        let mut wasm_return: i32 = 127;
        if return_type == 1 {
            wasm_return = 127;
        } else if return_type == 2 {
            wasm_return = 127;
        };
        if return_type == 0 {
            wasm_return = wasm_return + 1;
        } else {
            wasm_return = wasm_return + 2;
            wasm_return = wasm_return + 3;
        };
        idx = idx + 1;
    };
    idx
}
"#;

    compile(source)
        .expect("host compiler should accept else-if chains followed by else statements");

    stage1
        .compile_at(0, output_ptr, source)
        .expect("stage1 should accept else-if chains followed by else statements");
}

#[test]
fn stage1_compiler_accepts_unit_returns() {
    let (mut stage1, _) = prepare_stage1_compiler();
    let output_ptr = stage1_output_ptr(&stage1);

    let source = r#"
fn helper() {
    store_i32(0, 1);
}

fn main() -> i32 {
    helper();
    0
}
"#;

    compile(source).expect("host compiler should accept implicit unit return");

    stage1
        .compile_at(0, output_ptr, source)
        .expect("stage1 should accept implicit unit return");
}

#[test]
fn stage1_compiler_accepts_function_calls_in_equality_conditions() {
    let (mut stage1, _) = prepare_stage1_compiler();
    let output_ptr = stage1_output_ptr(&stage1);

    let source = r#"
fn type_code_i32() -> i32 { 1 }

fn type_code_bool() -> i32 { 2 }

fn get_type(flag: bool) -> i32 {
    if flag { 1 } else { 2 }
}

fn main() -> i32 {
    let mut result: i32 = 0;
    if get_type(true) == type_code_i32() {
        result = result + 10;
    } else if get_type(false) == type_code_bool() {
        result = result + 20;
    };
    result
}
"#;

    compile(source).expect("host compiler should accept function calls in equality conditions");

    stage1
        .compile_at(0, output_ptr, source)
        .expect("stage1 should accept function calls in equality conditions");
}

#[test]
fn stage1_compiler_accepts_function_calls_in_inequality_conditions() {
    let (mut stage1, _) = prepare_stage1_compiler();
    let output_ptr = stage1_output_ptr(&stage1);

    let source = r#"
fn type_code_unit() -> i32 { 0 }

fn type_code_i32() -> i32 { 1 }

fn main() -> i32 {
    let mut payload: i32 = 4;
    if type_code_i32() != type_code_unit() {
        payload = payload + 1;
    };
    payload
}
"#;

    compile(source)
        .expect("host compiler should accept function calls in inequality conditions");

    stage1
        .compile_at(0, output_ptr, source)
        .expect("stage1 should accept function calls in inequality conditions");
}

#[test]
fn stage1_compiler_accepts_expression_arguments_in_function_calls() {
    let (mut stage1, _) = prepare_stage1_compiler();
    let output_ptr = stage1_output_ptr(&stage1);

    let source = r#"
fn read(offset: i32) -> i32 {
    offset
}

fn main() -> i32 {
    let base: i32 = 8;
    read(base + 4)
}
"#;

    compile(source)
        .expect("host compiler should accept expression arguments in function calls");

    stage1
        .compile_at(0, output_ptr, source)
        .expect("stage1 should accept expression arguments in function calls");
}

#[test]
fn stage1_compiler_accepts_line_comments() {
    let (mut stage1, _) = prepare_stage1_compiler();
    let output_ptr = stage1_output_ptr(&stage1);

    let source = r#"
fn main() -> i32 {
    // ensure comment handling survives stage2
    let value: i32 = 5;
    if value == 5 {
        0
    } else {
        1
    }
}
"#;

    compile(source).expect("host compiler should accept line comments");

    stage1
        .compile_at(0, output_ptr, source)
        .expect("stage1 should accept line comments");
}

#[test]
fn stage1_compiler_accepts_not_equal_comparisons() {
    let (mut stage1, _) = prepare_stage1_compiler();
    let output_ptr = stage1_output_ptr(&stage1);

    let source = r#"
fn main() -> i32 {
    let mut result: i32 = 0;
    if 3 != 4 {
        result = result + 1;
    };
    result
}
"#;

    compile(source).expect("host compiler should accept not-equal comparisons");

    stage1
        .compile_at(0, output_ptr, source)
        .expect("stage1 should accept not-equal comparisons");
}

#[test]
fn stage1_compiler_accepts_greater_equal_comparisons() {
    let (mut stage1, _) = prepare_stage1_compiler();
    let output_ptr = stage1_output_ptr(&stage1);

    let source = r#"
fn main() -> i32 {
    let threshold: i32 = 7;
    if threshold >= 7 {
        1
    } else {
        0
    }
}
"#;

    compile(source).expect("host compiler should accept greater-equal comparisons");

    stage1
        .compile_at(0, output_ptr, source)
        .expect("stage1 should accept greater-equal comparisons");
}

#[test]
fn stage1_compiler_accepts_bitwise_and_or_operations() {
    let (mut stage1, _) = prepare_stage1_compiler();
    let output_ptr = stage1_output_ptr(&stage1);

    let source = r#"
fn mask(value: i32) -> i32 {
    (value & 255) | 8
}

fn main() -> i32 {
    mask(260)
}
"#;

    compile(source).expect("host compiler should accept bitwise and/or");

    stage1
        .compile_at(0, output_ptr, source)
        .expect("stage1 should accept bitwise and/or");
}

#[test]
fn stage1_compiler_accepts_nested_loops() {
    let (mut stage1, _) = prepare_stage1_compiler();
    let output_ptr = stage1_output_ptr(&stage1);

    let source = r#"
fn main() -> i32 {
    let mut outer: i32 = 0;
    let mut inner_sum: i32 = 0;
    loop {
        if outer >= 2 {
            break;
        };
        let mut inner: i32 = 0;
        loop {
            if inner >= 3 {
                break;
            };
            inner_sum = inner_sum + inner;
            inner = inner + 1;
        };
        outer = outer + 1;
    };
    inner_sum
}
"#;

    compile(source).expect("host compiler should accept nested loops");

    stage1
        .compile_at(0, output_ptr, source)
        .expect("stage1 should accept nested loops");
}

#[test]
fn stage1_compiler_accepts_return_without_value() {
    let (mut stage1, _) = prepare_stage1_compiler();
    let output_ptr = stage1_output_ptr(&stage1);

    let source = r#"
fn helper() {
    if true {
        return;
    };
}

fn main() -> i32 {
    helper();
    0
}
"#;

    compile(source).expect("host compiler should accept explicit unit returns");

    stage1
        .compile_at(0, output_ptr, source)
        .expect("stage1 should accept explicit `return;` statements");
}

#[test]
fn stage1_compiler_accepts_if_expression_results() {
    let (mut stage1, _) = prepare_stage1_compiler();
    let output_ptr = stage1_output_ptr(&stage1);

    let source = r#"
fn select(flag: bool) -> i32 {
    let value: i32 = if flag { 1 } else { 2 };
    value
}

fn main() -> i32 {
    select(true)
}
"#;

    compile(source).expect("host compiler should accept if expressions with values");

    stage1
        .compile_at(0, output_ptr, source)
        .expect("stage1 should accept if expressions with values");
}

#[test]
fn stage1_compiler_accepts_loop_local_redeclaration() {
    let (mut stage1, _) = prepare_stage1_compiler();
    let output_ptr = stage1_output_ptr(&stage1);

    let source = r#"
fn functions_entry(base: i32, index: i32) -> i32 {
    base + index
}

fn write_type_section(func_count: i32) -> i32 {
    let mut idx: i32 = 0;
    loop {
        if idx >= func_count {
            break;
        };
        let entry: i32 = functions_entry(0, idx);
        if entry == 99 {
            return 1;
        };
        idx = idx + 1;
    };

    let mut write_idx: i32 = 0;
    loop {
        if write_idx >= func_count {
            break;
        };
        let entry: i32 = functions_entry(0, write_idx);
        if entry == -1 {
            return 2;
        };
        write_idx = write_idx + 1;
    };

    0
}

fn main() -> i32 {
    write_type_section(2)
}
"#;

    compile(source).expect("host compiler should accept debug program");

    stage1
        .compile_at(0, output_ptr, source)
        .expect("stage1 should accept loop-local redeclarations");
}

#[test]
fn stage1_compiler_accepts_if_expression_blocks_with_values() {
    let (mut stage1, _) = prepare_stage1_compiler();
    let output_ptr = stage1_output_ptr(&stage1);

    let source = r#"
fn choose(flag: bool) -> i32 {
    let result: i32 = if flag {
        5
    } else {
        let base: i32 = 2;
        base + 3
    };
    result
}

fn main() -> i32 {
    choose(false)
}
"#;

    compile(source)
        .expect("host compiler should accept if expression blocks with tail values");

    stage1
        .compile_at(0, output_ptr, source)
        .expect("stage1 should accept if expression blocks with tail values");
}

#[test]
fn stage1_compiler_accepts_mutable_bool_locals() {
    let (mut stage1, _) = prepare_stage1_compiler();
    let output_ptr = stage1_output_ptr(&stage1);

    let source = r#"
fn main() -> i32 {
    let mut matched: bool = false;
    let mut idx: i32 = 0;
    loop {
        if idx >= 4 {
            break;
        };
        if idx == 2 {
            matched = true;
        };
        idx = idx + 1;
    };
    if matched { 1 } else { 0 }
}
"#;

    compile(source).expect("host compiler should accept mutable bool locals");

    stage1
        .compile_at(0, output_ptr, source)
        .expect("stage1 should accept mutable bool locals");
}

#[test]
fn stage1_compiler_minimal_write_type_section_repro() {
    let (mut stage1, _) = prepare_stage1_compiler();
    let output_ptr = stage1_output_ptr(&stage1);

    // stage1 now reserves 64 KiB for the instruction buffer (see `compile` in stage1).
    // Large function bodies should compile successfully within this capacity.
    let iteration_count = 3000;
    let mut source = String::from("fn main() -> i32 {\n    let mut value: i32 = 0;\n");
    for _ in 0..iteration_count {
        source.push_str("    value = value + 1;\n");
    }
    source.push_str("    value\n}\n");

    compile(&source).expect("host compiler should accept large function bodies");

    stage1
        .compile_at(0, output_ptr, &source)
        .expect("stage1 should compile large function bodies without exhausting its instruction buffer");
}

#[test]
fn stage1_compiler_accepts_bit_shifts() {
    let (mut stage1, _) = prepare_stage1_compiler();
    let output_ptr = stage1_output_ptr(&stage1);

    let source = r#"
fn main() -> i32 {
    let value: i32 = 8 >> 1;
    value
}
"#;

    compile(source).expect("host compiler should accept shift operators");

    stage1
        .compile_at(0, output_ptr, source)
        .expect("stage1 should accept shift operators");
}

#[test]
fn stage1_compiler_accepts_less_equal_comparisons() {
    let (mut stage1, _) = prepare_stage1_compiler();
    let output_ptr = stage1_output_ptr(&stage1);

    let source = r#"
fn within_bounds(index: i32, len: i32) -> i32 {
    if index + 4 <= len {
        1
    } else {
        0
    }
}

fn main() -> i32 {
    within_bounds(3, 10)
}
"#;

    compile(source).expect("host compiler should accept less-equal comparisons");

    stage1
        .compile_at(0, output_ptr, source)
        .expect("stage1 should accept less-equal comparisons");
}

#[test]
fn stage1_compiler_accepts_logical_and_chains() {
    let (mut stage1, _) = prepare_stage1_compiler();
    let output_ptr = stage1_output_ptr(&stage1);

    let source = r#"
fn matches_sequence(a: i32, b: i32, c: i32) -> i32 {
    if a == 1 && b == 2 && c == 3 {
        1
    } else {
        0
    }
}

fn main() -> i32 {
    matches_sequence(1, 2, 4)
}
"#;

    compile(source).expect("host compiler should accept logical and chains");

    stage1
        .compile_at(0, output_ptr, source)
        .expect("stage1 should accept logical and chains");
}

#[test]
fn stage1_compiler_accepts_logical_or_with_negation() {
    let (mut stage1, _) = prepare_stage1_compiler();
    let output_ptr = stage1_output_ptr(&stage1);

    let source = r#"
fn is_boundary(idx: i32, len: i32) -> bool {
    idx == 0 || idx == len
}

fn main() -> i32 {
    if is_boundary(0, 8) || !is_boundary(4, 8) {
        1
    } else {
        0
    }
}
"#;

    compile(source).expect("host compiler should accept logical or expressions");

    stage1
        .compile_at(0, output_ptr, source)
        .expect("stage1 should accept logical or expressions");
}

#[test]
#[ignore]
fn stage1_register_function_signatures_repro() {
    let (mut stage1, _) = prepare_stage1_compiler();
    let output_ptr = stage1_output_ptr(&stage1);

    // Minimal program that nests loops so `param_parse_idx` is mutated in the inner loop,
    // then reused by the outer loop—stage1 still restores the pre-loop locals instead of
    // merging the updates, so it cannot compile this structure yet.
    let source = r#"
fn stub_skip_whitespace(_base: i32, _len: i32, offset: i32) -> i32 { offset }

fn stub_expect_char(_base: i32, _len: i32, offset: i32, _ch: i32) -> i32 { offset }

fn stub_is_identifier_start(_byte: i32) -> bool { true }

fn stub_is_identifier_continue(_byte: i32) -> bool { false }

fn stub_peek_byte(_base: i32, _len: i32, _offset: i32) -> i32 { 0 }

fn register_function_signatures(
    input_ptr: i32,
    input_len: i32,
) -> i32 {
    let mut offset: i32 = 0;

    loop {
        offset = stub_skip_whitespace(input_ptr, input_len, offset);
        if offset >= input_len {
            break;
        };

        offset = offset + 1;

        if offset >= input_len {
            return -1;
        };
        stub_peek_byte(input_ptr, input_len, offset);
        offset = stub_skip_whitespace(input_ptr, input_len, offset);
        if offset >= input_len {
            return -1;
        };

        let mut name_len: i32 = 0;
        loop {
            if offset >= input_len {
                break;
            };
            let ch: i32 = stub_peek_byte(input_ptr, input_len, offset);
            if name_len == 0 {
                if !stub_is_identifier_start(ch) {
                    break;
                };
            } else if !stub_is_identifier_continue(ch) {
                break;
            };
            name_len = name_len + 1;
            offset = offset + 1;
        };
        if name_len == 0 {
            return -1;
        };

        offset = stub_skip_whitespace(input_ptr, input_len, offset);
        offset = stub_expect_char(input_ptr, input_len, offset, 40);
        if offset < 0 {
            return -1;
        };

        let mut param_parse_idx: i32 = stub_skip_whitespace(input_ptr, input_len, offset);
        loop {
            if param_parse_idx >= input_len {
                return -1;
            };
            let next_byte: i32 = stub_peek_byte(input_ptr, input_len, param_parse_idx);
            if next_byte == 41 {
                param_parse_idx = param_parse_idx + 1;
                break;
            };

            let mut name_len: i32 = 0;
            loop {
                if param_parse_idx >= input_len {
                    break;
                };
                let ch: i32 = stub_peek_byte(input_ptr, input_len, param_parse_idx);
                if name_len == 0 {
                    if !stub_is_identifier_start(ch) {
                        break;
                    };
                } else if !stub_is_identifier_continue(ch) {
                    break;
                };
                name_len = name_len + 1;
                param_parse_idx = param_parse_idx + 1;
            };
            if name_len == 0 {
                return -1;
            };

            param_parse_idx = stub_skip_whitespace(input_ptr, input_len, param_parse_idx);
            param_parse_idx = stub_expect_char(input_ptr, input_len, param_parse_idx, 58);
            if param_parse_idx < 0 {
                return -1;
            };

            param_parse_idx = stub_skip_whitespace(input_ptr, input_len, param_parse_idx);
            if param_parse_idx >= input_len {
                return -1;
            };

            if param_parse_idx < 0 {
                return -1;
            };

            param_parse_idx = stub_skip_whitespace(input_ptr, input_len, param_parse_idx);
            if param_parse_idx >= input_len {
                return -1;
            };
            let delimiter: i32 = stub_peek_byte(input_ptr, input_len, param_parse_idx);
            if delimiter == 44 {
                param_parse_idx = stub_skip_whitespace(input_ptr, input_len, param_parse_idx + 1);
                continue;
            };
            if delimiter == 41 {
                param_parse_idx = param_parse_idx + 1;
                break;
            };
            return -1;
        };

        offset = param_parse_idx;
    };

    offset
}

fn main() -> i32 { register_function_signatures(0, 0) }
"#;

    compile(source).expect("host compiler should accept register_function_signatures repro");

    stage1
        .compile_at(0, output_ptr, source)
        .expect_err("stage1 should fail when compiling register_function_signatures repro");
}

