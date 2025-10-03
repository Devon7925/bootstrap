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

#[test]
fn stage1_compiler_identifies_remaining_bootstrap_blocker() {
    let (mut stage1, stage1_source) = prepare_stage1_compiler();

    // Compile the stage1 source with the stage1 compiler itself to produce stage2.
    let result = stage1.compile_at(0, 131072, &stage1_source);
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
                compiled_functions, 84,
                "stage1 currently stops compiling at function index 84"
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
                failing_function.name, "write_type_section",
                "stage1 now fails while compiling write_type_section (first function containing an else-if chain)"
            );
        }
    }
}

// When stage1 fails we report the first function that still needs code generation
// support in order to reach full stage2 bootstrapping.

#[test]
fn stage1_compiler_accepts_break_with_value_statements() {
    let (mut stage1, _) = prepare_stage1_compiler();

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
        .compile_at(0, 131072, source)
        .expect("stage1 should accept break-with-value");
}

#[test]
fn stage1_compiler_accepts_loop_expression_results() {
    let (mut stage1, _) = prepare_stage1_compiler();

    let source = r#"
fn main() -> i32 {
    loop {
        break 7;
    }
}
"#;

    compile(source).expect("host compiler should accept loop expressions with values");

    stage1
        .compile_at(0, 131072, source)
        .expect("stage1 should accept loop expression result");
}

#[test]
fn stage1_compiler_accepts_else_if_chains() {
    let (mut stage1, _) = prepare_stage1_compiler();

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
        .compile_at(0, 131072, source)
        .expect("stage1 should accept else-if chains");
}

#[test]
fn stage1_compiler_accepts_else_if_inside_loops() {
    let (mut stage1, _) = prepare_stage1_compiler();

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
        .compile_at(0, 131072, source)
        .expect("stage1 should accept else-if chains inside loops");
}

#[test]
fn stage1_compiler_accepts_else_if_with_followup_else() {
    let (mut stage1, _) = prepare_stage1_compiler();

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
        .compile_at(0, 131072, source)
        .expect("stage1 should accept else-if chains followed by else statements");
}

#[test]
fn stage1_compiler_accepts_unit_returns() {
    let (mut stage1, _) = prepare_stage1_compiler();

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
        .compile_at(0, 131072, source)
        .expect("stage1 should accept implicit unit return");
}

#[test]
fn stage1_compiler_accepts_line_comments() {
    let (mut stage1, _) = prepare_stage1_compiler();

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
        .compile_at(0, 131072, source)
        .expect("stage1 should accept line comments");
}

#[test]
fn stage1_compiler_accepts_not_equal_comparisons() {
    let (mut stage1, _) = prepare_stage1_compiler();

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
        .compile_at(0, 131072, source)
        .expect("stage1 should accept not-equal comparisons");
}

#[test]
fn stage1_compiler_accepts_greater_equal_comparisons() {
    let (mut stage1, _) = prepare_stage1_compiler();

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
        .compile_at(0, 131072, source)
        .expect("stage1 should accept greater-equal comparisons");
}

#[test]
fn stage1_compiler_accepts_bitwise_and_or_operations() {
    let (mut stage1, _) = prepare_stage1_compiler();

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
        .compile_at(0, 131072, source)
        .expect("stage1 should accept bitwise and/or");
}

#[test]
fn stage1_compiler_accepts_nested_loops() {
    let (mut stage1, _) = prepare_stage1_compiler();

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
        .compile_at(0, 131072, source)
        .expect("stage1 should accept nested loops");
}

#[test]
fn stage1_compiler_accepts_return_without_value() {
    let (mut stage1, _) = prepare_stage1_compiler();

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
        .compile_at(0, 131072, source)
        .expect("stage1 should accept explicit `return;` statements");
}

#[test]
fn stage1_compiler_accepts_if_expression_results() {
    let (mut stage1, _) = prepare_stage1_compiler();

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
        .compile_at(0, 131072, source)
        .expect("stage1 should accept if expressions with values");
}

#[test]
fn stage1_compiler_accepts_if_expression_blocks_with_values() {
    let (mut stage1, _) = prepare_stage1_compiler();

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
        .compile_at(0, 131072, source)
        .expect("stage1 should accept if expression blocks with tail values");
}

#[test]
fn stage1_compiler_accepts_mutable_bool_locals() {
    let (mut stage1, _) = prepare_stage1_compiler();

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
        .compile_at(0, 131072, source)
        .expect("stage1 should accept mutable bool locals");
}

#[test]
fn stage1_compiler_accepts_bit_shifts() {
    let (mut stage1, _) = prepare_stage1_compiler();

    let source = r#"
fn main() -> i32 {
    let value: i32 = 8 >> 1;
    value
}
"#;

    compile(source).expect("host compiler should accept shift operators");

    stage1
        .compile_at(0, 131072, source)
        .expect("stage1 should accept shift operators");
}
