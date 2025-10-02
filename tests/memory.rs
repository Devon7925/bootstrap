use bootstrap::compile;
use wasmi::{Engine, Linker, Memory, Module, Store, TypedFunc};

#[test]
fn exports_single_page_memory() {
    let source = r#"
fn main(ptr: i32, len: i32) -> i32 {
    len
}
"#;

    let compilation = compile(source).expect("failed to compile source");
    let wasm = compilation
        .to_wasm()
        .expect("failed to encode module to wasm bytes");

    let engine = Engine::default();
    let module = Module::new(&engine, wasm.as_slice()).expect("failed to build module");
    let mut store = Store::new(&engine, ());
    let linker = Linker::new(&engine);
    let instance = linker
        .instantiate(&mut store, &module)
        .expect("failed to instantiate")
        .start(&mut store)
        .expect("failed to start module");

    let memory: Memory = instance
        .get_memory(&mut store, "memory")
        .expect("expected exported linear memory");
    let memory_bytes = memory
        .current_pages(&store)
        .to_bytes()
        .expect("memory size to fit into usize");
    assert_eq!(memory_bytes, 65536);

    let main: TypedFunc<(i32, i32), i32> = instance
        .get_typed_func(&mut store, "main")
        .expect("expected exported main function");
    let result = main
        .call(&mut store, (0, 42))
        .expect("failed to execute main");
    assert_eq!(result, 42);
}
