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
    let mut linker = Linker::new(&engine);
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
