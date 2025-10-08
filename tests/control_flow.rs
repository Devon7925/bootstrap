#[path = "ast_compiler_helpers.rs"]
mod ast_compiler_helpers;

#[path = "wasm_harness.rs"]
mod wasm_harness;

use ast_compiler_helpers::{compile_with_ast_compiler, try_compile_with_ast_compiler};
use wasm_harness::run_wasm_main;
use wasmi::{Engine, Linker, Module, Store, TypedFunc};

#[test]
fn loops_and_break_execute() {
    let source = r#"
fn loop_sum(limit: i32) -> i32 {
    let mut acc: i32 = 0;
    let mut i: i32 = 0;
    loop {
        if i == limit {
            break;
        };
        acc = acc + i;
        i = i + 1;
    }
    acc
}

fn main() -> i32 {
    let mut count: i32 = 0;
    let mut total: i32 = 0;
    loop {
        if count >= 5 {
            break;
        };
        total = total + loop_sum(count);
        count = count + 1;
    }
    total
}
"#;

    let wasm = compile_with_ast_compiler(source);

    let engine = Engine::default();
    let mut wasm_reader = wasm.as_slice();
    let module = Module::new(&engine, &mut wasm_reader).expect("failed to create module");
    let mut store = Store::new(&engine, ());
    let linker = Linker::new(&engine);
    let instance = linker
        .instantiate(&mut store, &module)
        .expect("failed to instantiate module")
        .start(&mut store)
        .expect("failed to start module");

    let main: TypedFunc<(), i32> = instance
        .get_typed_func(&mut store, "main")
        .expect("expected exported main");

    let result = main.call(&mut store, ()).expect("failed to execute main");

    assert_eq!(result, 10);
}

#[test]
fn continue_skips_iterations() {
    let source = r#"
fn sum_even(limit: i32) -> i32 {
    let mut acc: i32 = 0;
    let mut i: i32 = 0;
    loop {
        if i >= limit {
            break;
        };
        i = i + 1;
        let remainder: i32 = i - (i / 2) * 2;
        if remainder == 1 {
            continue;
        };
        acc = acc + i;
    }
    acc
}

fn loop_skip() -> i32 {
    let mut total: i32 = 0;
    let mut i: i32 = 0;
    loop {
        i = i + 1;
        if i > 5 {
            break;
        };
        if i == 3 {
            continue;
        };
        total = total + i;
    }
    total
}

fn main() -> i32 {
    sum_even(6) + loop_skip()
}
"#;

    let wasm = compile_with_ast_compiler(source);

    let engine = Engine::default();
    let mut wasm_reader = wasm.as_slice();
    let module = Module::new(&engine, &mut wasm_reader).expect("failed to create module");
    let mut store = Store::new(&engine, ());
    let linker = Linker::new(&engine);
    let instance = linker
        .instantiate(&mut store, &module)
        .expect("failed to instantiate module")
        .start(&mut store)
        .expect("failed to start module");

    let main: TypedFunc<(), i32> = instance
        .get_typed_func(&mut store, "main")
        .expect("expected exported main");

    let result = main.call(&mut store, ()).expect("failed to execute main");

    assert_eq!(result, 24);
}

#[test]
fn loop_can_break_with_values() {
    let source = r#"
fn find_first_even(limit: i32) -> i32 {
    let mut candidate: i32 = 0;
    let mut result: i32 = -1;
    loop {
        candidate = candidate + 1;
        if candidate >= limit {
            break;
        };
        candidate = candidate + 1;
        if candidate >= limit {
            break;
        };
        result = candidate;
        break;
    }
    result
}

fn main() -> i32 {
    find_first_even(10)
}
"#;

    let wasm = compile_with_ast_compiler(source);

    let engine = Engine::default();
    let mut wasm_reader = wasm.as_slice();
    let module = Module::new(&engine, &mut wasm_reader).expect("failed to create module");
    let mut store = Store::new(&engine, ());
    let linker = Linker::new(&engine);
    let instance = linker
        .instantiate(&mut store, &module)
        .expect("failed to instantiate module")
        .start(&mut store)
        .expect("failed to start module");

    let main: TypedFunc<(), i32> = instance
        .get_typed_func(&mut store, "main")
        .expect("expected exported main");

    let result = main.call(&mut store, ()).expect("failed to execute main");
    assert_eq!(result, 2);
}

#[test]
fn loop_break_types_must_match() {
    let source = r#"
fn bad() -> i32 {
    loop {
        if true {
            return 5;
        };
        break;
    }
    0
}

fn main() -> i32 {
    bad()
}
"#;
    let wasm = compile_with_ast_compiler(source);

    let engine = Engine::default();
    let mut wasm_reader = wasm.as_slice();
    let module = Module::new(&engine, &mut wasm_reader).expect("failed to create module");
    let mut store = Store::new(&engine, ());
    let linker = Linker::new(&engine);
    let instance = linker
        .instantiate(&mut store, &module)
        .expect("failed to instantiate module")
        .start(&mut store)
        .expect("failed to start module");

    let main: TypedFunc<(), i32> = instance
        .get_typed_func(&mut store, "main")
        .expect("expected exported main");

    let result = main.call(&mut store, ()).expect("failed to execute main");
    assert_eq!(result, 5);
}

#[test]
fn else_if_chains_execute() {
    let source = r#"
fn describe(value: i32) -> i32 {
    if value < 0 {
        -1
    } else if value == 0 {
        0
    } else if value == 1 {
        1
    } else {
        2
    }
}

fn main() -> i32 {
    describe(-3) + describe(0) * 10 + describe(1) * 100 + describe(5) * 1000
}
"#;

    let wasm = compile_with_ast_compiler(source);

    let engine = Engine::default();
    let mut wasm_reader = wasm.as_slice();
    let module = Module::new(&engine, &mut wasm_reader).expect("failed to create module");
    let mut store = Store::new(&engine, ());
    let linker = Linker::new(&engine);
    let instance = linker
        .instantiate(&mut store, &module)
        .expect("failed to instantiate module")
        .start(&mut store)
        .expect("failed to start module");

    let main: TypedFunc<(), i32> = instance
        .get_typed_func(&mut store, "main")
        .expect("expected exported main");

    let result = main.call(&mut store, ()).expect("failed to execute main");
    assert_eq!(result, 2099);
}

#[test]
fn loop_allows_final_if_without_semicolon() {
    let source = r#"
fn loop_with_final_if(limit: i32) -> i32 {
    let mut value: i32 = 0;
    loop {
        if value >= limit {
            break;
        };
        value = value + 1;
        if value == limit {
            break;
        }
    }
    value
}

fn main() -> i32 {
    loop_with_final_if(4)
}
"#;

    let wasm = compile_with_ast_compiler(source);

    let engine = Engine::default();
    let mut wasm_reader = wasm.as_slice();
    let module = Module::new(&engine, &mut wasm_reader).expect("failed to create module");
    let mut store = Store::new(&engine, ());
    let linker = Linker::new(&engine);
    let instance = linker
        .instantiate(&mut store, &module)
        .expect("failed to instantiate module")
        .start(&mut store)
        .expect("failed to start module");

    let main: TypedFunc<(), i32> = instance
        .get_typed_func(&mut store, "main")
        .expect("expected exported main");

    let result = main.call(&mut store, ()).expect("failed to execute main");

    assert_eq!(result, 4);
}

#[test]
fn while_break_cannot_carry_values() {
    let source = r#"
fn bad() {
    while (false) {
        break 1;
    }
}

fn main() -> i32 {
    0
}
"#;

    let error = try_compile_with_ast_compiler(source).expect_err("expected break value error");
    assert!(error.produced_len <= 0);
}

#[test]
fn loop_and_break_support_truthy_conditions() {
    let source = r#"
fn sum_up_to(limit: i32) -> i32 {
    let mut total: i32 = 0;
    let mut count: i32 = 0;
    let mut remaining: i32 = limit;
    loop {
        if remaining {
            total = total + count;
            count = count + 1;
            remaining = remaining - 1;
            0
        } else {
            break;
            0
        };
    }
    total
}

fn main() -> i32 {
    sum_up_to(5)
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let engine = wasmi::Engine::default();
    let result = run_wasm_main(&engine, &wasm);
    assert_eq!(result, 10);
}

#[test]
fn predicate_calls_in_loops_execute() {
    let source = r#"
fn predicate(value: i32) -> bool {
    if value >= 3 {
        true
    } else {
        false
    }
}

fn main() -> i32 {
    let mut value: i32 = 0;
    loop {
        if predicate(value) {
            break;
        };
        value = value + 1;
    }
    value
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let engine = wasmi::Engine::default();
    let result = run_wasm_main(&engine, &wasm);
    assert_eq!(result, 3);
}

#[test]
fn if_statements_inside_blocks_execute() {
    let source = r#"
fn adjust(input: i32) -> i32 {
    let mut value: i32 = input;
    if value > 0 {
        value = value - 1;
    };
    if value < 0 {
        value = 0;
    };
    value
}

fn main() -> i32 {
    adjust(2) + adjust(-1)
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let engine = wasmi::Engine::default();
    let result = run_wasm_main(&engine, &wasm);
    assert_eq!(result, 1);
}

#[test]
fn loop_breaks_can_return_values() {
    let source = r#"
fn choose() -> i32 {
    loop {
        break 42;
    }
}

fn main() -> i32 {
    choose()
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let engine = wasmi::Engine::default();
    let result = run_wasm_main(&engine, &wasm);
    assert_eq!(result, 42);
}

#[test]
fn continue_outside_loop_is_rejected() {
    let source = r#"
fn main() -> i32 {
    continue;
    0
}
"#;

    let error = try_compile_with_ast_compiler(source)
        .expect_err("continue outside loops should be rejected");
    assert!(error.produced_len <= 0);
}

#[test]
fn nested_loops_can_break_with_values() {
    let source = r#"
fn nested(limit: i32) -> i32 {
    let mut outer: i32 = limit;
    let mut total: i32 = 0;
    loop {
        if outer {
            let mut inner: i32 = outer;
            loop {
                if inner {
                    total = total + outer;
                    inner = inner - 1;
                    0
                } else {
                    break;
                    0
                };
            }
            outer = outer - 1;
            0
        } else {
            break total;
            0
        };
    }
}

fn main() -> i32 {
    nested(3)
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let engine = wasmi::Engine::default();
    let result = run_wasm_main(&engine, &wasm);
    assert_eq!(result, 14);
}

#[test]
fn diverging_if_tail_statements_are_allowed() {
    let source = r#"
fn branch(flag: bool) -> i32 {
    if flag {
        return 10;
    } else {
        return 20;
    };
}

fn main() -> i32 {
    branch(true)
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let engine = wasmi::Engine::default();
    let result = run_wasm_main(&engine, &wasm);
    assert_eq!(result, 10);
}

#[test]
fn break_outside_loop_is_rejected() {
    let source = r#"
fn main() -> i32 {
    break;
    0
}
"#;

    let error = try_compile_with_ast_compiler(source)
        .expect_err("break outside loop should be rejected");
    assert!(error.produced_len <= 0);
}

#[test]
fn if_with_literal_condition_executes() {
    let source = r#"
fn main() -> i32 {
    if 1 {
        42
    } else {
        0
    }
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let engine = wasmi::Engine::default();
    let result = run_wasm_main(&engine, &wasm);
    assert_eq!(result, 42);
}

#[test]
fn if_else_with_parameter_condition_executes() {
    let source = r#"
fn choose(flag: i32) -> i32 {
    if flag {
        10
    } else {
        20
    }
}

fn main() -> i32 {
    choose(0)
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let engine = wasmi::Engine::default();
    let result = run_wasm_main(&engine, &wasm);
    assert_eq!(result, 20);
}

#[test]
fn nested_if_expressions_execute() {
    let source = r#"
fn pick(a: i32, b: i32) -> i32 {
    if a {
        if b {
            1
        } else {
            2
        }
    } else {
        if b {
            3
        } else {
            4
        }
    }
}

fn main() -> i32 {
    pick(0, 1) + pick(1, 0) * 10
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let engine = wasmi::Engine::default();
    let result = run_wasm_main(&engine, &wasm);
    assert_eq!(result, 23);
}

#[test]
fn else_if_chains_cover_all_branches() {
    let source = r#"
fn classify(value: i32) -> i32 {
    if value < 0 {
        1
    } else if value == 0 {
        2
    } else {
        3
    }
}

fn main() -> i32 {
    classify(-2) + classify(0) * 10 + classify(5) * 100
}
"#;

    let wasm = compile_with_ast_compiler(source);
    let engine = wasmi::Engine::default();
    let result = run_wasm_main(&engine, &wasm);
    assert_eq!(result, 321);
}
