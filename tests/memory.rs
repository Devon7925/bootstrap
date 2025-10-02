use bootstrap::compile;
use wasmi::{Engine, Linker, Memory, Module, Store, TypedFunc};

#[test]
fn exports_single_page_memory() {
    let source = r#"
fn slice_len(_ptr: i32, len: i32) -> i32 {
    len
}

fn main() -> i32 {
    0
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

    let slice_len: TypedFunc<(i32, i32), i32> = instance
        .get_typed_func(&mut store, "slice_len")
        .expect("expected exported slice_len function");
    let result = slice_len
        .call(&mut store, (0, 42))
        .expect("failed to execute main");
    assert_eq!(result, 42);
}

#[test]
fn reads_last_byte_from_input_slice() {
    let source = r#"
fn last_byte(ptr: i32, len: i32) -> i32 {
    if len == 0 {
        return -1;
    };

    let last: i32 = len - 1;
    load_u8(ptr + last)
}

fn main() -> i32 {
    0
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

    let input = b"bootstrap";
    let offset = 32usize;
    memory
        .write(&mut store, offset, input)
        .expect("failed to write input into linear memory");

    let last_byte: TypedFunc<(i32, i32), i32> = instance
        .get_typed_func(&mut store, "last_byte")
        .expect("expected exported last_byte function");
    let result = last_byte
        .call(&mut store, (offset as i32, input.len() as i32))
        .expect("failed to execute main");

    assert_eq!(result as u8, *input.last().expect("non-empty slice"));
}

#[test]
fn writes_byte_into_memory() {
    let source = r#"
fn write_then_read(ptr: i32, value: i32) -> i32 {
    store_u8(ptr, value);
    load_u8(ptr)
}

fn main() -> i32 {
    0
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

    let write_then_read: TypedFunc<(i32, i32), i32> = instance
        .get_typed_func(&mut store, "write_then_read")
        .expect("expected exported write_then_read function");

    let offset = 128i32;
    let value = 173i32;
    let result = write_then_read
        .call(&mut store, (offset, value))
        .expect("failed to execute main");

    assert_eq!(result as u8, value as u8);

    let mut buffer = [0u8; 1];
    memory
        .read(&store, offset as usize, &mut buffer)
        .expect("failed to read written byte");
    assert_eq!(buffer[0], value as u8);
}
