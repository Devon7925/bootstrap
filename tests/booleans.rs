use bootstrap::compile;
use wasmi::{Engine, Linker, Module, Store, TypedFunc};

#[test]
fn boolean_logic_and_loops_execute() {
    let source = r#"
fn choose(flag: bool, a: i32, b: i32) -> i32 {
    if flag {
        a
    } else {
        b
    }
}

fn logic_chain(a: bool, b: bool, c: bool) -> bool {
    if a && (b || c) {
        true
    } else {
        false
    }
}

fn find_even(limit: i32) -> bool {
    let mut i: i32 = 0;
    loop {
        if i == limit {
            break false;
        };
        if i % 2 == 0 && i != 0 {
            break true;
        };
        i = i + 1;
    }
}

fn main() -> i32 {
    let first: i32 = choose(true, 10, 20);
    let second: i32 = choose(false, 1, 2);
    if logic_chain(true, false, true) && find_even(5) {
        first + second
    } else {
        0
    }
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

    let choose: TypedFunc<(i32, i32, i32), i32> = instance
        .get_typed_func(&mut store, "choose")
        .expect("expected exported choose");
    let logic_chain: TypedFunc<(i32, i32, i32), i32> = instance
        .get_typed_func(&mut store, "logic_chain")
        .expect("expected exported logic_chain");
    let find_even: TypedFunc<i32, i32> = instance
        .get_typed_func(&mut store, "find_even")
        .expect("expected exported find_even");
    let main: TypedFunc<(), i32> = instance
        .get_typed_func(&mut store, "main")
        .expect("expected exported main");

    let choose_true = choose
        .call(&mut store, (1, 7, 3))
        .expect("failed to call choose");
    assert_eq!(choose_true, 7);
    let choose_false = choose
        .call(&mut store, (0, 7, 3))
        .expect("failed to call choose");
    assert_eq!(choose_false, 3);

    let logic_true = logic_chain
        .call(&mut store, (1, 0, 1))
        .expect("failed to call logic_chain");
    assert_eq!(logic_true, 1);
    let logic_false = logic_chain
        .call(&mut store, (0, 1, 1))
        .expect("failed to call logic_chain");
    assert_eq!(logic_false, 0);

    let find_even_result = find_even
        .call(&mut store, 6)
        .expect("failed to call find_even");
    assert_eq!(find_even_result, 1);
    let find_even_miss = find_even
        .call(&mut store, 1)
        .expect("failed to call find_even");
    assert_eq!(find_even_miss, 0);

    let main_result = main.call(&mut store, ()).expect("failed to execute main");
    assert_eq!(main_result, 12);
}
