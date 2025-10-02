use std::fs;

use bootstrap::compile;
use wasmi::{Engine, Linker, Memory, Module, Store, TypedFunc};

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

    let input = b"fn main() -> i32 { return 7; }";
    let input_offset = 0usize;
    memory
        .write(&mut store, input_offset, input)
        .expect("failed to write input for stage1");

    let output_offset = 1024i32;
    let compile_func: TypedFunc<(i32, i32, i32), i32> = instance
        .get_typed_func(&mut store, "compile")
        .expect("expected exported compile function");
    let produced_len = compile_func
        .call(
            &mut store,
            (input_offset as i32, input.len() as i32, output_offset),
        )
        .expect("stage1 compile invocation failed");
    assert!(produced_len > 0, "stage1 compiler returned no bytes");

    let mut output = vec![0u8; produced_len as usize];
    memory
        .read(&store, output_offset as usize, &mut output)
        .expect("failed to read stage1 output");

    assert!(output.starts_with(&[0x00, 0x61, 0x73, 0x6d]));

    let target_module =
        Module::new(&engine, output.as_slice()).expect("failed to create target module");
    let mut target_store = Store::new(&engine, ());
    let target_linker = Linker::new(&engine);
    let target_instance = target_linker
        .instantiate(&mut target_store, &target_module)
        .expect("failed to instantiate target module")
        .start(&mut target_store)
        .expect("failed to start target module");

    let target_memory: Memory = target_instance
        .get_memory(&mut target_store, "memory")
        .expect("target module should export memory");
    assert_eq!(
        target_memory
            .current_pages(&target_store)
            .to_bytes()
            .unwrap(),
        65536
    );

    let main_fn: TypedFunc<(), i32> = target_instance
        .get_typed_func(&mut target_store, "main")
        .expect("compiled module should export main");
    let main_result = main_fn
        .call(&mut target_store, ())
        .expect("failed to execute compiled main");
    assert_eq!(main_result, 7);

    let input_two = b"fn main() -> i32 {\n    -9\n}\n";
    let input_offset_two = input_offset + 256usize;
    memory
        .write(&mut store, input_offset_two, input_two)
        .expect("failed to write second input for stage1");

    let output_offset_two = output_offset + 2048;
    let produced_len_two = compile_func
        .call(
            &mut store,
            (
                input_offset_two as i32,
                input_two.len() as i32,
                output_offset_two,
            ),
        )
        .expect("stage1 compile invocation failed for second program");
    assert!(
        produced_len_two > 0,
        "stage1 compiler returned no bytes for second program"
    );

    let mut output_two = vec![0u8; produced_len_two as usize];
    memory
        .read(&store, output_offset_two as usize, &mut output_two)
        .expect("failed to read stage1 second output");

    let target_module_two =
        Module::new(&engine, output_two.as_slice()).expect("failed to create second target module");
    let mut target_store_two = Store::new(&engine, ());
    let target_linker_two = Linker::new(&engine);
    let target_instance_two = target_linker_two
        .instantiate(&mut target_store_two, &target_module_two)
        .expect("failed to instantiate second target module")
        .start(&mut target_store_two)
        .expect("failed to start second target module");

    let main_fn_two: TypedFunc<(), i32> = target_instance_two
        .get_typed_func(&mut target_store_two, "main")
        .expect("second compiled module should export main");
    let main_result_two = main_fn_two
        .call(&mut target_store_two, ())
        .expect("failed to execute second compiled main");
    assert_eq!(main_result_two, -9);

    let input_three = b"fn main() -> i32 {\n    return 5 + 3 - 2 + -4;\n}\n";
    let input_offset_three = input_offset_two + 256usize;
    memory
        .write(&mut store, input_offset_three, input_three)
        .expect("failed to write third input for stage1");

    let output_offset_three = output_offset_two + 4096;
    let produced_len_three = compile_func
        .call(
            &mut store,
            (
                input_offset_three as i32,
                input_three.len() as i32,
                output_offset_three,
            ),
        )
        .expect("stage1 compile invocation failed for third program");
    assert!(
        produced_len_three > 0,
        "stage1 compiler returned no bytes for third program"
    );

    let mut output_three = vec![0u8; produced_len_three as usize];
    memory
        .read(&store, output_offset_three as usize, &mut output_three)
        .expect("failed to read stage1 third output");

    let target_module_three = Module::new(&engine, output_three.as_slice())
        .expect("failed to create third target module");
    let mut target_store_three = Store::new(&engine, ());
    let target_linker_three = Linker::new(&engine);
    let target_instance_three = target_linker_three
        .instantiate(&mut target_store_three, &target_module_three)
        .expect("failed to instantiate third target module")
        .start(&mut target_store_three)
        .expect("failed to start third target module");

    let main_fn_three: TypedFunc<(), i32> = target_instance_three
        .get_typed_func(&mut target_store_three, "main")
        .expect("third compiled module should export main");
    let main_result_three = main_fn_three
        .call(&mut target_store_three, ())
        .expect("failed to execute third compiled main");
    assert_eq!(main_result_three, 2);
}
