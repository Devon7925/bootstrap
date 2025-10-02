use bootstrap::compile;
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
    while (count < 5) {
        total = total + loop_sum(count);
        count = count + 1;
    }
    total
}
"#;

    let compilation = compile(source).expect("failed to compile source");
    let wasm = compilation.to_wasm().expect("failed to encode wasm");

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
    while (i < limit) {
        i = i + 1;
        if i % 2 == 1 {
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

    let compilation = compile(source).expect("failed to compile source");
    let wasm = compilation.to_wasm().expect("failed to encode wasm");

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
    let mut i: i32 = 0;
    loop {
        if i == limit {
            break -1;
        };
        if i % 2 == 0 && i != 0 {
            break i;
        };
        i = i + 1;
    }
}

fn main() -> i32 {
    find_first_even(10)
}
"#;

    let compilation = compile(source).expect("failed to compile source");
    let wasm = compilation.to_wasm().expect("failed to encode wasm");

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
            break 5;
        };
        break;
    }
}
"#;

    let error = match compile(source) {
        Ok(_) => panic!("expected type mismatch error"),
        Err(err) => err,
    };
    assert!(
        error.message.contains("break value type mismatch"),
        "unexpected error message: {}",
        error.message
    );
}

#[test]
fn while_break_cannot_carry_values() {
    let source = r#"
fn bad() {
    while (false) {
        break 1;
    }
}
"#;

    let error = match compile(source) {
        Ok(_) => panic!("expected break value error"),
        Err(err) => err,
    };
    assert!(
        error
            .message
            .contains("only allowed inside `loop` expressions"),
        "unexpected error message: {}",
        error.message
    );
}
