pub mod error;

use std::fmt;

use crate::error::CompileError;
use wasmtime::{Config, Engine, Extern, Instance, Memory, Module, Store, TypedFunc};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Wasm,
    Wgsl,
}

impl Target {
    pub const DEFAULT: Target = Target::Wasm;
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Target::Wasm => f.write_str("wasm"),
            Target::Wgsl => f.write_str("wgsl"),
        }
    }
}

const COMPILER_WASM: &[u8] = include_bytes!("../compiler.wasm");
const INSTR_OFFSET_PTR_OFFSET: usize = 4_096;
const FUNCTIONS_COUNT_PTR_OFFSET: usize = 851_960;
pub const FUNCTIONS_BASE_OFFSET: usize = 851_968;
pub const FUNCTION_ENTRY_SIZE: usize = 32;
pub const STAGE1_MAX_FUNCTIONS: usize = 512;

pub struct Compilation {
    target: Target,
    wasm: Vec<u8>,
}

impl Compilation {
    pub fn target(&self) -> Target {
        self.target
    }

    pub fn wasm(&self) -> &[u8] {
        &self.wasm
    }

    pub fn to_wasm(&self) -> Result<Vec<u8>, CompileError> {
        if self.target != Target::Wasm {
            return Err(CompileError::new(format!(
                "target '{}' cannot be emitted as Wasm",
                self.target
            )));
        }
        Ok(self.wasm.clone())
    }

    pub fn into_wasm(self) -> Result<Vec<u8>, CompileError> {
        if self.target != Target::Wasm {
            return Err(CompileError::new(format!(
                "target '{}' cannot be emitted as Wasm",
                self.target
            )));
        }
        Ok(self.wasm)
    }
}

pub fn compile(source: &str, target: Target) -> Result<Compilation, CompileError> {
    if source.is_empty() {
        return Err(CompileError::new("source must not be empty"));
    }

    if target != Target::Wasm {
        return Err(CompileError::new(format!(
            "target '{}' is not supported yet",
            target
        )));
    }

    let mut config = Config::new();
    config.wasm_reference_types(true);
    config.wasm_function_references(true);
    config.wasm_gc(true);
    let engine = Engine::new(&config)
        .map_err(|err| CompileError::new(format!("failed to configure wasmtime engine: {err}")))?;
    let module = Module::from_binary(&engine, COMPILER_WASM)
        .map_err(|err| CompileError::new(format!("failed to load stage2 module: {err}")))?;
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[] as &[Extern]).map_err(|err| {
        CompileError::new(format!("failed to instantiate stage2 compiler: {err}"))
    })?;

    let memory: Memory = instance
        .get_memory(&mut store, "memory")
        .ok_or_else(|| CompileError::new("stage2 compiler must export memory"))?;

    let compile: TypedFunc<(i32, i32, i32), i32> =
        instance
            .get_typed_func(&mut store, "compile")
            .map_err(|_| CompileError::new("stage2 compiler missing compile export"))?;

    let memory_size = memory.data_size(&store);
    let reserved = FUNCTIONS_BASE_OFFSET + STAGE1_MAX_FUNCTIONS * FUNCTION_ENTRY_SIZE;
    if memory_size <= reserved {
        return Err(CompileError::new(
            "stage2 compiler memory layout does not leave space for output buffer",
        ));
    }

    let output_ptr = source.len() as i32;

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

    Ok(Compilation { target, wasm })
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

    let mut detail_buf = vec![0u8; 256];
    let mut detail = String::new();
    if memory
        .read(store, output_ptr as usize, &mut detail_buf)
        .is_ok()
    {
        if let Some(end) = detail_buf.iter().position(|&b| b == 0) {
            detail_buf.truncate(end);
        }
        if let Ok(text) = String::from_utf8(detail_buf.clone()) {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                detail = format!(", detail=\"{}\"", trimmed);
            }
        }
    }

    format!(
        "stage2 compilation failed (status {produced_len}, functions={functions}, instr_offset={instr_offset}, compiled_functions={compiled_functions}{detail})"
    )
}

pub fn compile_to_wasm(source: &str) -> Result<Vec<u8>, CompileError> {
    compile(source, Target::Wasm)?.into_wasm()
}
