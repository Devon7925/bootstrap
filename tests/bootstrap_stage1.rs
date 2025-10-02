use std::fs;

use bootstrap::compile;

#[path = "wasm_harness.rs"]
mod wasm_harness;

use wasm_harness::{run_wasm_main, CompilerInstance};

#[test]
fn stage1_constant_compiler_emits_wasm() {
    let source =
        fs::read_to_string("examples/stage1_minimal.bp").expect("failed to load stage1 source");

    let stage1_compilation = compile(&source).expect("failed to compile stage1 source");
    let stage1_wasm = stage1_compilation
        .to_wasm()
        .expect("failed to encode stage1 wasm");

    let mut compiler = CompilerInstance::new(stage1_wasm.as_slice());
    assert_eq!(compiler.memory_size_bytes(), 262144);

    let mut input_cursor = 0usize;
    let mut output_cursor = 1024i32;

    let output = compiler
        .compile_with_layout(
            &mut input_cursor,
            &mut output_cursor,
            "fn main() -> i32 { return 7; }",
        )
        .expect("stage1 should compile constant return");
    assert!(output.starts_with(&[0x00, 0x61, 0x73, 0x6d]));
    assert_eq!(run_wasm_main(compiler.engine(), &output), 7);

    let output_two = compiler
        .compile_with_layout(
            &mut input_cursor,
            &mut output_cursor,
            "fn main() -> i32 {\n    -9\n}\n",
        )
        .expect("stage1 should compile negative literal");
    assert_eq!(run_wasm_main(compiler.engine(), &output_two), -9);

    let output_three = compiler
        .compile_with_layout(
            &mut input_cursor,
            &mut output_cursor,
            "fn main() -> i32 {\n    return 5 + 3 - 2 + -4;\n}\n",
        )
        .expect("stage1 should compile arithmetic chain");
    assert_eq!(run_wasm_main(compiler.engine(), &output_three), 2);

    let output_four = compiler
        .compile_with_layout(
            &mut input_cursor,
            &mut output_cursor,
            "fn main() -> i32 {\n    return -(1 + 2) + (3 - (4 - 5));\n}\n",
        )
        .expect("stage1 should compile nested expressions");
    assert_eq!(run_wasm_main(compiler.engine(), &output_four), 1);

    let output_five = compiler
        .compile_with_layout(
            &mut input_cursor,
            &mut output_cursor,
            "fn main() -> i32 {\n    return 6 * 7;\n}\n",
        )
        .expect("stage1 should compile multiplication");
    assert_eq!(run_wasm_main(compiler.engine(), &output_five), 42);

    let output_six = compiler
        .compile_with_layout(
            &mut input_cursor,
            &mut output_cursor,
            "fn main() -> i32 {\n    return 30 / 2 + 4 * 3;\n}\n",
        )
        .expect("stage1 should compile mixed ops");
    assert_eq!(run_wasm_main(compiler.engine(), &output_six), 27);

    let output_seven = compiler
        .compile_with_layout(
            &mut input_cursor,
            &mut output_cursor,
            "fn main() -> i32 {\n    let x: i32 = 2 + 3;\n    let y: i32 = x * 10;\n    y / 2\n}\n",
        )
        .expect("stage1 should compile let binding");
    assert_eq!(run_wasm_main(compiler.engine(), &output_seven), 25);

    let output_eight = compiler
        .compile_with_layout(
            &mut input_cursor,
            &mut output_cursor,
            "fn main() -> i32 {\n    let mut total: i32 = 1;\n    total = total + 5;\n    total\n}\n",
        )
        .expect("stage1 should compile mut assignment");
    assert_eq!(run_wasm_main(compiler.engine(), &output_eight), 6);

    let output_nine = compiler
        .compile_with_layout(
            &mut input_cursor,
            &mut output_cursor,
            "fn main() -> i32 {\n    let mut value: i32 = 1;\n    let cond: bool = 2 + 2 == 4 && !(3 < 2);\n    if cond {\n        value = value + 4;\n    };\n    if 10 <= 5 || cond {\n        value = value + 8;\n    };\n    value\n}\n",
        )
        .expect("stage1 should compile boolean logic");
    assert_eq!(run_wasm_main(compiler.engine(), &output_nine), 13);

    let output_ten = compiler
        .compile_with_layout(
            &mut input_cursor,
            &mut output_cursor,
            "fn main() -> i32 {\n    let cond: bool = 3 > 5;\n    let computed: i32 = if cond {\n        1\n    } else {\n        let base: i32 = 2;\n        base * 5\n    };\n    let chained: i32 = if cond {\n        10\n    } else if true {\n        20\n    } else {\n        30\n    };\n    computed + chained\n}\n",
        )
        .expect("stage1 should compile nested if expressions");
    assert_eq!(run_wasm_main(compiler.engine(), &output_ten), 30);

    let output_eleven = compiler
        .compile_with_layout(
            &mut input_cursor,
            &mut output_cursor,
            "fn main() -> i32 {\n    let mut acc: i32 = 0;\n    let mut i: i32 = 0;\n    loop {\n        if i == 5 {\n            break;\n        };\n        acc = acc + i;\n        i = i + 1;\n    };\n    acc\n}\n",
        )
        .expect("stage1 should compile loop with break");
    assert_eq!(run_wasm_main(compiler.engine(), &output_eleven), 10);

    let output_twelve = compiler
        .compile_with_layout(
            &mut input_cursor,
            &mut output_cursor,
            "fn main() -> i32 {\n    let mut total: i32 = 0;\n    let mut i: i32 = 0;\n    loop {\n        i = i + 1;\n        if i > 6 {\n            break;\n        };\n        let parity: i32 = i - (i / 2) * 2;\n        if parity == 1 {\n            continue;\n        };\n        total = total + i;\n    };\n    total\n}\n",
        )
        .expect("stage1 should compile loop with continue");
    assert_eq!(run_wasm_main(compiler.engine(), &output_twelve), 12);

    let output_thirteen = compiler
        .compile_with_layout(
            &mut input_cursor,
            &mut output_cursor,
            "fn helper() -> i32 {\n    return 5;\n}\n\nfn main() -> i32 {\n    helper()\n}\n",
        )
        .expect("stage1 should compile simple call");
    assert_eq!(run_wasm_main(compiler.engine(), &output_thirteen), 5);

    let output_fourteen = compiler
        .compile_with_layout(
            &mut input_cursor,
            &mut output_cursor,
            "fn increment(value: i32) -> i32 {\n    value + 1\n}\n\nfn is_even(value: i32) -> bool {\n    return value - (value / 2) * 2 == 0;\n}\n\nfn pick(flag: bool, left: i32, right: i32) -> i32 {\n    if flag {\n        left\n    } else {\n        right\n    }\n}\n\nfn main() -> i32 {\n    let base: i32 = increment(7);\n    pick(is_even(base), base, 5)\n}\n",
        )
        .expect("stage1 should compile multi-function program");
    assert_eq!(run_wasm_main(compiler.engine(), &output_fourteen), 8);
}
