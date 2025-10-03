#![allow(dead_code)]

use wasmi::{Engine, Linker, Memory, Module, Store, TypedFunc};

pub const DEFAULT_INPUT_STRIDE: usize = 256;
pub const DEFAULT_OUTPUT_STRIDE: i32 = 4096;

pub struct CompilerInstance {
    engine: Engine,
    store: Store<()>,
    memory: Memory,
    compile: TypedFunc<(i32, i32, i32), i32>,
}

const INSTR_OFFSET_PTR_OFFSET: usize = 4096;
const FUNCTIONS_COUNT_PTR_OFFSET: usize = 851960;
const FUNCTIONS_BASE_OFFSET: usize = 851968;
const TYPES_COUNT_PTR_OFFSET: usize = 819196;
const TYPES_BASE_OFFSET: usize = 819200;
const TYPE_ENTRY_SIZE: usize = 16;

#[derive(Debug, Clone, Copy)]
pub struct CompileFailure {
    pub produced_len: i32,
    pub functions: i32,
    pub instr_offset: i32,
    pub compiled_functions: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeEntry {
    pub name_start: i32,
    pub name_len: i32,
    pub value_start: i32,
    pub value_len: i32,
}

impl CompilerInstance {
    pub fn new(wasm: &[u8]) -> Self {
        let engine = Engine::default();
        Self::from_engine(engine, wasm)
    }

    fn from_engine(engine: Engine, wasm: &[u8]) -> Self {
        let module = Module::new(&engine, wasm).expect("failed to create module");
        let mut store = Store::new(&engine, ());
        let linker = Linker::new(&engine);
        let instance = linker
            .instantiate(&mut store, &module)
            .expect("failed to instantiate module")
            .start(&mut store)
            .expect("failed to start module");
        let memory = instance
            .get_memory(&mut store, "memory")
            .expect("compiler module must export memory");
        let compile = instance
            .get_typed_func(&mut store, "compile")
            .expect("expected exported compile function");

        Self {
            engine,
            store,
            memory,
            compile,
        }
    }

    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    pub fn memory_size_bytes(&self) -> usize {
        self.memory
            .current_pages(&self.store)
            .to_bytes()
            .expect("memory pages to bytes") as usize
    }

    pub fn compile_at(
        &mut self,
        input_ptr: usize,
        output_ptr: i32,
        source: &str,
    ) -> Result<Vec<u8>, CompileFailure> {
        assert!(output_ptr >= 0, "output pointer must be non-negative");
        assert!(
            input_ptr <= i32::MAX as usize,
            "input pointer must fit in i32"
        );
        assert!(
            source.len() <= i32::MAX as usize,
            "source length must fit in i32"
        );

        self.memory
            .write(&mut self.store, input_ptr, source.as_bytes())
            .expect("failed to write source into compiler memory");

        let produced_len = match self.compile.call(
            &mut self.store,
            (input_ptr as i32, source.len() as i32, output_ptr),
        ) {
            Ok(len) => len,
            Err(_) => {
                return Err(self.read_failure(output_ptr, -1));
            }
        };
        if produced_len <= 0 {
            return Err(self.read_failure(output_ptr, produced_len));
        }

        let mut output = vec![0u8; produced_len as usize];
        self.memory
            .read(&self.store, output_ptr as usize, &mut output)
            .expect("failed to read compiler output");
        Ok(output)
    }

    pub fn compile_with_stride(
        &mut self,
        input_cursor: &mut usize,
        output_cursor: &mut i32,
        input_stride: usize,
        output_stride: i32,
        source: &str,
    ) -> Result<Vec<u8>, CompileFailure> {
        let output = self.compile_at(*input_cursor, *output_cursor, source)?;
        *input_cursor += input_stride;
        *output_cursor += output_stride;
        Ok(output)
    }

    pub fn compile_with_layout(
        &mut self,
        input_cursor: &mut usize,
        output_cursor: &mut i32,
        source: &str,
    ) -> Result<Vec<u8>, CompileFailure> {
        self.compile_with_stride(
            input_cursor,
            output_cursor,
            DEFAULT_INPUT_STRIDE,
            DEFAULT_OUTPUT_STRIDE,
            source,
        )
    }

    pub fn read_types_count(&self, output_ptr: i32) -> i32 {
        self.read_i32(output_ptr + TYPES_COUNT_PTR_OFFSET as i32)
    }

    pub fn read_type_entry(&self, output_ptr: i32, index: usize) -> TypeEntry {
        let entry_ptr = output_ptr as usize + TYPES_BASE_OFFSET + index * TYPE_ENTRY_SIZE;
        let mut buf = [0u8; TYPE_ENTRY_SIZE];
        self.memory
            .read(&self.store, entry_ptr, &mut buf)
            .expect("failed to read type entry from compiler memory");
        TypeEntry {
            name_start: i32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]),
            name_len: i32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]),
            value_start: i32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]),
            value_len: i32::from_le_bytes([buf[12], buf[13], buf[14], buf[15]]),
        }
    }

    pub fn read_i32(&self, ptr: i32) -> i32 {
        let mut buf = [0u8; 4];
        let _ = self
            .memory
            .read(&self.store, ptr as usize, &mut buf)
            .expect("read_i32 should succeed");
        i32::from_le_bytes(buf)
    }
}

impl CompilerInstance {
    fn read_failure(&mut self, output_ptr: i32, produced_len: i32) -> CompileFailure {
        let mut func_buf = [0u8; 4];
        let mut instr_buf = [0u8; 4];
        let func_ptr = output_ptr as usize + FUNCTIONS_COUNT_PTR_OFFSET;
        let instr_ptr = output_ptr as usize + INSTR_OFFSET_PTR_OFFSET;
        let _ = self.memory.read(&self.store, func_ptr, &mut func_buf);
        let _ = self.memory.read(&self.store, instr_ptr, &mut instr_buf);
        let functions = i32::from_le_bytes(func_buf);
        let instr_offset = i32::from_le_bytes(instr_buf);
        let mut compiled_functions = 0;
        for index in 0..functions {
            let entry = output_ptr as usize + FUNCTIONS_BASE_OFFSET + index as usize * 32;
            let mut len_buf = [0u8; 4];
            if self
                .memory
                .read(&self.store, entry + 16, &mut len_buf)
                .is_err()
            {
                break;
            }
            let code_len = i32::from_le_bytes(len_buf);
            if code_len > 0 {
                compiled_functions += 1;
            } else {
                break;
            }
        }
        CompileFailure {
            produced_len,
            functions,
            instr_offset,
            compiled_functions,
        }
    }
}

pub fn run_wasm_main(engine: &Engine, wasm: &[u8]) -> i32 {
    let module = Module::new(engine, wasm).expect("failed to create target module");
    let mut store = Store::new(engine, ());
    let linker = Linker::new(engine);
    let instance = linker
        .instantiate(&mut store, &module)
        .expect("failed to instantiate target module")
        .start(&mut store)
        .expect("failed to start target module");

    let memory: Memory = instance
        .get_memory(&mut store, "memory")
        .expect("compiled module should export memory");
    assert_eq!(
        memory
            .current_pages(&store)
            .to_bytes()
            .expect("memory pages to bytes"),
        1048576
    );

    let main_fn: TypedFunc<(), i32> = instance
        .get_typed_func(&mut store, "main")
        .expect("compiled module should export main");
    main_fn
        .call(&mut store, ())
        .expect("failed to execute compiled main")
}
