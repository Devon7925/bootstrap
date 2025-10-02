use bootstrap::compile;
use wasmi::{Engine, Func, Linker, Memory, Module, Store, TypedFunc, Value};

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
    assert_eq!(memory_bytes, 131072);

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

#[test]
fn stores_and_loads_word_values() {
    let source = r#"
fn roundtrip_i32(ptr: i32, value: i32) -> i32 {
    store_i32(ptr, value);
    load_i32(ptr)
}

fn roundtrip_i64(ptr: i32, value: i64) -> i64 {
    store_i64(ptr, value);
    load_i64(ptr)
}

fn roundtrip_f32(ptr: i32, value: f32) -> f32 {
    store_f32(ptr, value);
    load_f32(ptr)
}

fn roundtrip_f64(ptr: i32, value: f64) -> f64 {
    store_f64(ptr, value);
    load_f64(ptr)
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

    let roundtrip_i32: TypedFunc<(i32, i32), i32> = instance
        .get_typed_func(&mut store, "roundtrip_i32")
        .expect("expected exported roundtrip_i32 function");
    let roundtrip_i64: TypedFunc<(i32, i64), i64> = instance
        .get_typed_func(&mut store, "roundtrip_i64")
        .expect("expected exported roundtrip_i64 function");
    let roundtrip_f32: Func = instance
        .get_func(&mut store, "roundtrip_f32")
        .expect("expected exported roundtrip_f32 function");
    let roundtrip_f64: Func = instance
        .get_func(&mut store, "roundtrip_f64")
        .expect("expected exported roundtrip_f64 function");

    let i32_result = roundtrip_i32
        .call(&mut store, (256, 0x7fff_ff12))
        .expect("failed to execute roundtrip_i32");
    assert_eq!(i32_result, 0x7fff_ff12);

    let i64_value: i64 = 0x7fff_ffff_ffff_ff13;
    let i64_result = roundtrip_i64
        .call(&mut store, (512, i64_value))
        .expect("failed to execute roundtrip_i64");
    assert_eq!(i64_result, i64_value);

    let f32_value: f32 = 123.5;
    let mut f32_results = [Value::F32(0.0f32.into())];
    roundtrip_f32
        .call(
            &mut store,
            &[Value::I32(768), Value::F32(f32_value.into())],
            &mut f32_results,
        )
        .expect("failed to execute roundtrip_f32");
    let f32_bits = f32_results[0].f32().expect("expected f32 result").to_bits();
    let f32_result = f32::from_bits(f32_bits);
    assert!((f32_result - f32_value).abs() < f32::EPSILON);

    let f64_value: f64 = 98765.4321;
    let mut f64_results = [Value::F64(0.0f64.into())];
    roundtrip_f64
        .call(
            &mut store,
            &[Value::I32(1024), Value::F64(f64_value.into())],
            &mut f64_results,
        )
        .expect("failed to execute roundtrip_f64");
    let f64_bits = f64_results[0].f64().expect("expected f64 result").to_bits();
    let f64_result = f64::from_bits(f64_bits);
    assert!((f64_result - f64_value).abs() < f64::EPSILON);
}
