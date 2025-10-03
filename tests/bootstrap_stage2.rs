use std::fs;

use bootstrap::compile;

#[path = "wasm_harness.rs"]
mod wasm_harness;

use wasm_harness::{CompilerInstance, run_wasm_main};

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
    let reserved = STAGE1_FUNCTIONS_BASE_OFFSET + STAGE1_MAX_FUNCTIONS * STAGE1_FUNCTION_ENTRY_SIZE;
    assert!(
        memory_size > reserved,
        "stage1 memory must exceed reserved layout"
    );
    (memory_size - reserved) as i32
}

#[test]
fn stage1_compiler_bootstraps_stage2() {
    let (mut stage1, stage1_source) = prepare_stage1_compiler();
    let stage1_output_address = stage1_output_ptr(&stage1);
    assert!(
        stage1_source.len() < stage1_output_address as usize,
        "stage1 source must not overlap output buffer",
    );

    // Compile the stage1 source with the stage1 compiler itself to produce stage2.
    let stage2_wasm = stage1
        .compile_at(0, stage1_output_address, &stage1_source)
        .expect("stage1 should compile itself and produce stage2");

    // Ensure stage2 can be instantiated and compile the stage1 source again.
    let mut stage2 = CompilerInstance::new(&stage2_wasm);
    let stage2_output_address = stage1_output_ptr(&stage2);
    assert!(
        stage1_source.len() < stage2_output_address as usize,
        "stage2 output buffer must accommodate stage1 source",
    );
    let stage3_wasm = stage2
        .compile_at(0, stage2_output_address, &stage1_source)
        .expect("stage2 should compile the stage1 source");

    assert_eq!(
        &stage2_wasm, &stage3_wasm,
        "stage2 and stage3 wasm outputs should be identical",
    );

    // Stage3 should be a fully functional compiler capable of compiling user
    // programs. Compile a small program and execute it to verify the output.
    let mut stage3 = CompilerInstance::new(&stage3_wasm);
    let stage3_output_address = stage1_output_ptr(&stage3);
    let sample_program = r#"
fn main() -> i32 {
    let mut total: i32 = 0;
    let mut idx: i32 = 0;
    loop {
        if idx >= 5 {
            break;
        };
        total = total + idx;
        idx = idx + 1;
    };
    total
}
"#;
    assert!(
        sample_program.len() < stage3_output_address as usize,
        "sample program must fit in stage3 output buffer",
    );
    let sample_wasm = stage3
        .compile_at(0, stage3_output_address, sample_program)
        .expect("stage3 should compile sample program");

    assert_eq!(
        run_wasm_main(stage3.engine(), &sample_wasm),
        10,
        "compiled sample program should execute correctly",
    );
}

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

    compile(source).expect("host compiler should accept function calls in inequality conditions");

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

    compile(source).expect("host compiler should accept expression arguments in function calls");

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

    compile(source).expect("host compiler should accept if expression blocks with tail values");

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

    stage1.compile_at(0, output_ptr, &source).expect(
        "stage1 should compile large function bodies without exhausting its instruction buffer",
    );
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
fn stage1_shadowed_name_len_repro() {
    let (mut stage1, _) = prepare_stage1_compiler();
    let output_ptr = stage1_output_ptr(&stage1);

    // Stage1 previously rejected this program because it confused the inner `name_len`
    // binding with the outer one. This regression test ensures the bug remains fixed.
    let source = r#"
fn main() -> i32 {
    let mut name_len: i32 = 0;
    loop {
        let mut name_len: i32 = 0;
        name_len = name_len + 1;
        break;
    };

    name_len
}
"#;

    compile(source).expect("host compiler should accept name_len shadowing repro");

    stage1
        .compile_at(0, output_ptr, source)
        .expect("stage1 should accept name_len shadowing repro");
}
