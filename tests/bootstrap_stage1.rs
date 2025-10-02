use std::fs;

use bootstrap::compile;
use wasmi::{Engine, Linker, Memory, Module, Store, TypedFunc};

fn stage1_compile_program(
    store: &mut Store<()>,
    memory: &Memory,
    compile_func: &TypedFunc<(i32, i32, i32), i32>,
    input_cursor: &mut usize,
    output_cursor: &mut i32,
    source: &str,
) -> Vec<u8> {
    memory
        .write(&mut *store, *input_cursor, source.as_bytes())
        .expect("failed to write source for stage1");

    let produced_len = compile_func
        .call(
            &mut *store,
            (*input_cursor as i32, source.len() as i32, *output_cursor),
        )
        .expect("stage1 compile invocation failed");
    assert!(produced_len > 0, "stage1 compiler returned no bytes");

    let mut output = vec![0u8; produced_len as usize];
    memory
        .read(&*store, *output_cursor as usize, &mut output)
        .expect("failed to read stage1 output");

    *input_cursor += 256;
    *output_cursor += 4096;

    output
}

fn run_stage1_output(engine: &Engine, wasm: &[u8]) -> i32 {
    let target_module = Module::new(engine, wasm).expect("failed to create target module");
    let mut target_store = Store::new(engine, ());
    let target_linker = Linker::new(engine);
    let target_instance = target_linker
        .instantiate(&mut target_store, &target_module)
        .expect("failed to instantiate target module")
        .start(&mut target_store)
        .expect("failed to start target module");

    let target_memory: Memory = target_instance
        .get_memory(&mut target_store, "memory")
        .expect("compiled module should export memory");
    assert_eq!(
        target_memory
            .current_pages(&target_store)
            .to_bytes()
            .expect("memory pages to bytes"),
        65536
    );

    let main_fn: TypedFunc<(), i32> = target_instance
        .get_typed_func(&mut target_store, "main")
        .expect("compiled module should export main");
    main_fn
        .call(&mut target_store, ())
        .expect("failed to execute compiled main")
}

#[test]
fn stage1_constant_compiler_emits_wasm() {
    let source =
        fs::read_to_string("examples/stage1_minimal.bp").expect("failed to load stage1 source");

    let stage1_compilation = compile(&source).expect("failed to compile stage1 source");
    let stage1_wasm = stage1_compilation
        .to_wasm()
        .expect("failed to encode stage1 wasm");

    let engine = Engine::default();
    let module =
        Module::new(&engine, stage1_wasm.as_slice()).expect("failed to create stage1 module");
    let mut store = Store::new(&engine, ());
    let linker = Linker::new(&engine);
    let instance = linker
        .instantiate(&mut store, &module)
        .expect("failed to instantiate stage1 module")
        .start(&mut store)
        .expect("failed to start stage1 module");

    let memory: Memory = instance
        .get_memory(&mut store, "memory")
        .expect("stage1 module must export memory");

    let mut input_cursor = 0usize;
    let mut output_cursor = 1024i32;

    let compile_func: TypedFunc<(i32, i32, i32), i32> = instance
        .get_typed_func(&mut store, "compile")
        .expect("expected exported compile function");

    let output = stage1_compile_program(
        &mut store,
        &memory,
        &compile_func,
        &mut input_cursor,
        &mut output_cursor,
        "fn main() -> i32 { return 7; }",
    );
    assert!(output.starts_with(&[0x00, 0x61, 0x73, 0x6d]));
    assert_eq!(run_stage1_output(&engine, &output), 7);

    let output_two = stage1_compile_program(
        &mut store,
        &memory,
        &compile_func,
        &mut input_cursor,
        &mut output_cursor,
        "fn main() -> i32 {\n    -9\n}\n",
    );
    assert_eq!(run_stage1_output(&engine, &output_two), -9);

    let output_three = stage1_compile_program(
        &mut store,
        &memory,
        &compile_func,
        &mut input_cursor,
        &mut output_cursor,
        "fn main() -> i32 {\n    return 5 + 3 - 2 + -4;\n}\n",
    );
    assert_eq!(run_stage1_output(&engine, &output_three), 2);

    let output_four = stage1_compile_program(
        &mut store,
        &memory,
        &compile_func,
        &mut input_cursor,
        &mut output_cursor,
        "fn main() -> i32 {\n    return -(1 + 2) + (3 - (4 - 5));\n}\n",
    );
    assert_eq!(run_stage1_output(&engine, &output_four), 1);

    let output_five = stage1_compile_program(
        &mut store,
        &memory,
        &compile_func,
        &mut input_cursor,
        &mut output_cursor,
        "fn main() -> i32 {\n    return 6 * 7;\n}\n",
    );
    assert_eq!(run_stage1_output(&engine, &output_five), 42);

    let output_six = stage1_compile_program(
        &mut store,
        &memory,
        &compile_func,
        &mut input_cursor,
        &mut output_cursor,
        "fn main() -> i32 {\n    return 30 / 2 + 4 * 3;\n}\n",
    );
    assert_eq!(run_stage1_output(&engine, &output_six), 27);

    let output_seven = stage1_compile_program(
        &mut store,
        &memory,
        &compile_func,
        &mut input_cursor,
        &mut output_cursor,
        "fn main() -> i32 {\n    let x: i32 = 2 + 3;\n    let y: i32 = x * 10;\n    y / 2\n}\n",
    );
    assert_eq!(run_stage1_output(&engine, &output_seven), 25);

    let output_eight = stage1_compile_program(
        &mut store,
        &memory,
        &compile_func,
        &mut input_cursor,
        &mut output_cursor,
        "fn main() -> i32 {\n    let mut total: i32 = 1;\n    total = total + 5;\n    total\n}\n",
    );
    assert_eq!(run_stage1_output(&engine, &output_eight), 6);
}
