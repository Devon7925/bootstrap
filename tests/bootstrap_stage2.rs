#[path = "wasm_harness.rs"]
mod wasm_harness;

use wasm_harness::{CompilerInstance, run_wasm_main};

#[path = "stage1_helpers.rs"]
mod stage1_helpers;

use stage1_helpers::{stage1_source, stage1_wasm};

fn prepare_stage1_compiler() -> (CompilerInstance, &'static str) {
    (CompilerInstance::new(stage1_wasm()), stage1_source())
}

const STAGE1_FUNCTION_ENTRY_SIZE: usize = 32;
const STAGE1_FUNCTIONS_BASE_OFFSET: usize = 851_968;
const STAGE1_MAX_FUNCTIONS: usize = 320;

fn stage1_output_ptr(compiler: &CompilerInstance) -> i32 {
    let memory_size = compiler.memory_size_bytes();
    let reserved = STAGE1_FUNCTIONS_BASE_OFFSET + STAGE1_MAX_FUNCTIONS * STAGE1_FUNCTION_ENTRY_SIZE;
    assert!(
        memory_size > reserved,
        "stage1 memory must exceed reserved layout"
    );
    (memory_size - reserved) as i32
}

#[test]
fn stage1_compiler_bootstraps_stage2() {
    let (mut stage1, stage1_source) = prepare_stage1_compiler();
    let stage1_output_address = stage1_output_ptr(&stage1);
    assert!(
        stage1_source.len() < stage1_output_address as usize,
        "stage1 source must not overlap output buffer",
    );

    // Compile the stage1 source with the stage1 compiler itself to produce stage2.
    let stage2_wasm = stage1
        .compile_at(0, stage1_output_address, &stage1_source)
        .expect("stage1 should compile itself and produce stage2");

    // Ensure stage2 can be instantiated and compile the stage1 source again.
    let mut stage2 = CompilerInstance::new(&stage2_wasm);
    let stage2_output_address = stage1_output_ptr(&stage2);
    assert!(
        stage1_source.len() < stage2_output_address as usize,
        "stage2 output buffer must accommodate stage1 source",
    );
    let stage3_wasm = stage2
        .compile_at(0, stage2_output_address, &stage1_source)
        .expect("stage2 should compile the stage1 source");

    assert_eq!(
        &stage2_wasm, &stage3_wasm,
        "stage2 and stage3 wasm outputs should be identical",
    );

    // Stage3 should be a fully functional compiler capable of compiling user
    // programs. Compile a small program and execute it to verify the output.
    let mut stage3 = CompilerInstance::new(&stage3_wasm);
    let stage3_output_address = stage1_output_ptr(&stage3);
    let sample_program = r#"
fn main() -> i32 {
    let mut total: i32 = 0;
    let mut idx: i32 = 0;
    loop {
        if idx >= 5 {
            break;
        };
        total = total + idx;
        idx = idx + 1;
    };
    total
}
"#;
    assert!(
        sample_program.len() < stage3_output_address as usize,
        "sample program must fit in stage3 output buffer",
    );
    let sample_wasm = stage3
        .compile_at(0, stage3_output_address, sample_program)
        .expect("stage3 should compile sample program");

    assert_eq!(
        run_wasm_main(stage3.engine(), &sample_wasm),
        10,
        "compiled sample program should execute correctly",
    );
}
