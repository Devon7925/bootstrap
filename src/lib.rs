pub mod ast;
pub mod codegen;
pub mod error;
pub mod hir;
pub mod lexer;
pub mod parser;
pub mod span;
pub mod typeck;

use crate::error::CompileError;
use wasmi::{Engine, Linker, Memory, Module, Store, TypedFunc};

const STAGE2_WASM: &[u8] = include_bytes!("../stage2.wasm");
const INSTR_OFFSET_PTR_OFFSET: usize = 4_096;
const FUNCTIONS_COUNT_PTR_OFFSET: usize = 851_960;
const FUNCTIONS_BASE_OFFSET: usize = 851_968;
const FUNCTION_ENTRY_SIZE: usize = 32;
const STAGE1_MAX_FUNCTIONS: usize = 512;

pub struct Compilation {
    wasm: Vec<u8>,
}

impl Compilation {
    pub fn wasm(&self) -> &[u8] {
        &self.wasm
    }

    pub fn to_wasm(&self) -> Result<Vec<u8>, CompileError> {
        Ok(self.wasm.clone())
    }

    pub fn into_wasm(self) -> Result<Vec<u8>, CompileError> {
        Ok(self.wasm)
    }
}

pub fn compile(source: &str) -> Result<Compilation, CompileError> {
    if source.is_empty() {
        return Err(CompileError::new("source must not be empty"));
    }

    let engine = Engine::default();
    let module = Module::new(&engine, STAGE2_WASM)
        .map_err(|err| CompileError::new(format!("failed to load stage2 module: {err}")))?;
    let mut store = Store::new(&engine, ());
    let linker = Linker::new(&engine);
    let instance = linker
        .instantiate(&mut store, &module)
        .and_then(|inst| inst.start(&mut store))
        .map_err(|err| {
            CompileError::new(format!("failed to instantiate stage2 compiler: {err}"))
        })?;

    let memory: Memory = instance
        .get_memory(&mut store, "memory")
        .ok_or_else(|| CompileError::new("stage2 compiler must export memory"))?;

    let compile: TypedFunc<(i32, i32, i32), i32> =
        instance
            .get_typed_func(&mut store, "compile")
            .map_err(|_| CompileError::new("stage2 compiler missing compile export"))?;

    let memory_size = memory
        .current_pages(&store)
        .to_bytes()
        .ok_or_else(|| CompileError::new("stage2 memory size overflowed"))?
        as usize;
    let reserved = FUNCTIONS_BASE_OFFSET + STAGE1_MAX_FUNCTIONS * FUNCTION_ENTRY_SIZE;
    if memory_size <= reserved {
        return Err(CompileError::new(
            "stage2 compiler memory layout does not leave space for output buffer",
        ));
    }

    let output_ptr = (memory_size - reserved) as i32;
    if source.len() >= output_ptr as usize {
        return Err(CompileError::new(
            "source is too large to fit in stage2 compiler memory",
        ));
    }

    memory
        .write(&mut store, 0, source.as_bytes())
        .map_err(|err| {
            CompileError::new(format!("failed to write source into stage2 memory: {err}"))
        })?;

    let produced_len = compile
        .call(&mut store, (0, source.len() as i32, output_ptr))
        .map_err(|err| CompileError::new(format!("stage2 compilation trapped: {err}")))?;

    if produced_len <= 0 {
        let failure = read_stage2_failure(&memory, &store, output_ptr, produced_len);
        return Err(CompileError::new(failure));
    }

    let mut wasm = vec![0u8; produced_len as usize];
    memory
        .read(&store, output_ptr as usize, &mut wasm)
        .map_err(|err| CompileError::new(format!("failed to read stage2 output: {err}")))?;

    Ok(Compilation { wasm })
}

fn read_stage2_failure(
    memory: &Memory,
    store: &Store<()>,
    output_ptr: i32,
    produced_len: i32,
) -> String {
    let mut functions_buf = [0u8; 4];
    let mut instr_buf = [0u8; 4];
    let functions = memory
        .read(
            store,
            output_ptr as usize + FUNCTIONS_COUNT_PTR_OFFSET,
            &mut functions_buf,
        )
        .map(|_| i32::from_le_bytes(functions_buf))
        .unwrap_or(-1);
    let instr_offset = memory
        .read(
            store,
            output_ptr as usize + INSTR_OFFSET_PTR_OFFSET,
            &mut instr_buf,
        )
        .map(|_| i32::from_le_bytes(instr_buf))
        .unwrap_or(-1);
    let mut compiled_functions = 0;
    if functions > 0 {
        for index in 0..functions {
            let entry =
                output_ptr as usize + FUNCTIONS_BASE_OFFSET + index as usize * FUNCTION_ENTRY_SIZE;
            let mut len_buf = [0u8; 4];
            if memory.read(store, entry + 16, &mut len_buf).is_err() {
                break;
            }
            let code_len = i32::from_le_bytes(len_buf);
            if code_len > 0 {
                compiled_functions += 1;
            } else {
                break;
            }
        }
    }

    format!(
        "stage2 compilation failed (status {produced_len}, functions={functions}, instr_offset={instr_offset}, compiled_functions={compiled_functions})"
    )
}

pub fn compile_to_wasm(source: &str) -> Result<Vec<u8>, CompileError> {
    compile(source)?.into_wasm()
}
