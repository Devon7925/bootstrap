use bootstrap::compile;
use wasmi::{Engine, Func, Linker, Module, Store, TypedFunc, Value};

#[test]
fn numeric_suffixes_emit_correct_types() {
    let source = r#"
fn add_wide(a: i64, b: i64) -> i64 {
    a + b + 1i64
}

fn sum_f32() -> f32 {
    let mut total: f32 = 1.5;
    total = total + 2.25;
    total
}

fn sum_f64() -> f64 {
    let mut total: f64 = 1.0f64;
    total = total + 2.5f64;
    total
}

fn main() -> i64 {
    add_wide(10i64, 5i64)
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

    let main_func: TypedFunc<(), i64> = instance
        .get_typed_func(&mut store, "main")
        .expect("expected exported main");
    let sum_f32_func: Func = instance
        .get_func(&mut store, "sum_f32")
        .expect("expected exported sum_f32");
    let sum_f64_func: Func = instance
        .get_func(&mut store, "sum_f64")
        .expect("expected exported sum_f64");

    let main_result = main_func
        .call(&mut store, ())
        .expect("failed to execute main");
    assert_eq!(main_result, 16);

    let mut f32_results = [Value::F32(0.0f32.into())];
    sum_f32_func
        .call(&mut store, &[], &mut f32_results)
        .expect("failed to execute sum_f32");
    let f32_value = f32_results[0].f32().expect("expected f32 result").to_bits();
    let f32_result = f32::from_bits(f32_value);
    assert!((f32_result - 3.75).abs() < f32::EPSILON);

    let mut f64_results = [Value::F64(0.0f64.into())];
    sum_f64_func
        .call(&mut store, &[], &mut f64_results)
        .expect("failed to execute sum_f64");
    let f64_value = f64_results[0].f64().expect("expected f64 result").to_bits();
    let f64_result = f64::from_bits(f64_value);
    assert!((f64_result - 3.5).abs() < 1e-9);
}
