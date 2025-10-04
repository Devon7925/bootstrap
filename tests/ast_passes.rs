#[path = "stage1_helpers.rs"]
mod stage1_helpers;

use stage1_helpers::compile_with_stage1;

use wasmparser::{ExternalKind, Operator, Parser, Payload, TypeRef};

#[test]
fn ast_constant_folding_collapses_integer_expression() {
    let source = r#"
fn constant_expr() -> i32 {
    (1 + 2) * (3 + 4)
}

fn main() -> i32 {
    constant_expr()
}
"#;

    let wasm = compile_with_stage1(source);

    let parser = Parser::new(0);
    let mut import_count: u32 = 0;
    let mut defined_index: u32 = 0;
    let mut target_func_index: Option<u32> = None;
    let mut found_body = false;

    for payload in parser.parse_all(&wasm) {
        let payload = payload.expect("failed to parse wasm payload");
        match payload {
            Payload::ImportSection(imports) => {
                for import in imports {
                    let import = import.expect("failed to parse import");
                    if let TypeRef::Func(_) = import.ty {
                        import_count += 1;
                    }
                }
            }
            Payload::ExportSection(exports) => {
                for export in exports {
                    let export = export.expect("failed to parse export");
                    if export.name == "constant_expr" {
                        if let ExternalKind::Func = export.kind {
                            target_func_index = Some(export.index);
                        }
                    }
                }
            }
            Payload::CodeSectionEntry(body) => {
                let func_index = import_count + defined_index;
                if Some(func_index) == target_func_index {
                    let mut locals_reader = body.get_locals_reader().expect("failed to read locals");
                    for _ in 0..locals_reader.get_count() {
                        locals_reader
                            .read()
                            .expect("failed to parse local declaration");
                    }
                    let mut operators = body
                        .get_operators_reader()
                        .expect("failed to read operators");
                    let mut saw_const = false;
                    let mut const_value: i32 = 0;
                    while !operators.eof() {
                        let op = operators.read().expect("failed to read operator");
                        match op {
                            Operator::I32Const { value } => {
                                assert!(
                                    !saw_const,
                                    "unexpected multiple constants in constant_expr body",
                                );
                                saw_const = true;
                                const_value = value;
                            }
                            Operator::End => break,
                            Operator::Return => continue,
                            other => panic!(
                                "unexpected operator {:?} while checking constant folding",
                                other
                            ),
                        }
                    }
                    assert!(saw_const, "expected i32.const in constant_expr body");
                    assert_eq!(const_value, 21, "expected folded constant value");
                    found_body = true;
                }
                defined_index += 1;
            }
            _ => {}
        }
    }

    assert!(found_body, "failed to locate constant_expr body");
}

#[test]
fn ast_simplifies_boolean_equality_with_constants() {
    let source = r#"
fn cond_true(flag: bool) -> i32 {
    if flag == true {
        1
    } else {
        0
    }
}

fn cond_false(flag: bool) -> i32 {
    if flag == false {
        1
    } else {
        0
    }
}

fn cond_ne_true(flag: bool) -> i32 {
    if flag != true {
        1
    } else {
        0
    }
}

fn cond_ne_false(flag: bool) -> i32 {
    if flag != false {
        1
    } else {
        0
    }
}

fn main() -> i32 {
    0
}
"#;

    let wasm = compile_with_stage1(source);

    let parser = Parser::new(0);
    let mut import_count: u32 = 0;
    let mut defined_index: u32 = 0;
    let mut function_indices = std::collections::HashMap::new();
    let mut results: std::collections::HashMap<u32, (bool, bool)> =
        std::collections::HashMap::new();

    for payload in parser.parse_all(&wasm) {
        let payload = payload.expect("failed to parse wasm payload");
        match payload {
            Payload::ImportSection(imports) => {
                for import in imports {
                    let import = import.expect("failed to parse import");
                    if let TypeRef::Func(_) = import.ty {
                        import_count += 1;
                    }
                }
            }
            Payload::ExportSection(exports) => {
                for export in exports {
                    let export = export.expect("failed to parse export");
                    if let ExternalKind::Func = export.kind {
                        function_indices.insert(export.name.to_string(), export.index);
                    }
                }
            }
            Payload::CodeSectionEntry(body) => {
                let func_index = import_count + defined_index;
                let mut locals_reader = body.get_locals_reader().expect("failed to read locals");
                for _ in 0..locals_reader.get_count() {
                    locals_reader
                        .read()
                        .expect("failed to parse local declaration");
                }

                let mut operators = body
                    .get_operators_reader()
                    .expect("failed to read operators");
                let mut saw_eq = false;
                let mut saw_eqz = false;
                while !operators.eof() {
                    match operators.read().expect("failed to read operator") {
                        Operator::I32Eq { .. } => saw_eq = true,
                        Operator::I32Eqz { .. } => saw_eqz = true,
                        Operator::End => break,
                        _ => {}
                    }
                }
                results.insert(func_index, (saw_eq, saw_eqz));
                defined_index += 1;
            }
            _ => {}
        }
    }

    let cond_true_index = *function_indices
        .get("cond_true")
        .expect("expected cond_true export");
    let cond_false_index = *function_indices
        .get("cond_false")
        .expect("expected cond_false export");
    let cond_ne_true_index = *function_indices
        .get("cond_ne_true")
        .expect("expected cond_ne_true export");
    let cond_ne_false_index = *function_indices
        .get("cond_ne_false")
        .expect("expected cond_ne_false export");

    let (true_eq, true_eqz) = results
        .get(&cond_true_index)
        .copied()
        .expect("missing cond_true body");
    assert!(!true_eq, "cond_true should not emit i32.eq");
    assert!(!true_eqz, "cond_true should not emit i32.eqz");

    let (false_eq, false_eqz) = results
        .get(&cond_false_index)
        .copied()
        .expect("missing cond_false body");
    assert!(!false_eq, "cond_false should not emit i32.eq");
    assert!(false_eqz, "cond_false should emit i32.eqz");

    let (ne_true_eq, ne_true_eqz) = results
        .get(&cond_ne_true_index)
        .copied()
        .expect("missing cond_ne_true body");
    assert!(!ne_true_eq, "cond_ne_true should not emit i32.eq");
    assert!(ne_true_eqz, "cond_ne_true should emit i32.eqz");

    let (ne_false_eq, ne_false_eqz) = results
        .get(&cond_ne_false_index)
        .copied()
        .expect("missing cond_ne_false body");
    assert!(!ne_false_eq, "cond_ne_false should not emit i32.eq");
    assert!(!ne_false_eqz, "cond_ne_false should not emit i32.eqz");
}

#[test]
fn ast_simplifies_logical_operations_with_constants() {
    let source = r#"
fn and_true(flag: bool) -> i32 {
    if flag && true {
        1
    } else {
        0
    }
}

fn and_false(flag: bool) -> i32 {
    if flag && false {
        1
    } else {
        0
    }
}

fn or_true(flag: bool) -> i32 {
    if flag || true {
        1
    } else {
        0
    }
}

fn or_false(flag: bool) -> i32 {
    if flag || false {
        1
    } else {
        0
    }
}

fn main() -> i32 {
    0
}
"#;

    let wasm = compile_with_stage1(source);

    let parser = Parser::new(0);
    let mut import_count: u32 = 0;
    let mut defined_index: u32 = 0;
    let mut function_indices = std::collections::HashMap::new();
    let mut i32_if_counts: std::collections::HashMap<u32, u32> =
        std::collections::HashMap::new();

    for payload in parser.parse_all(&wasm) {
        let payload = payload.expect("failed to parse wasm payload");
        match payload {
            Payload::ImportSection(imports) => {
                for import in imports {
                    let import = import.expect("failed to parse import");
                    if let TypeRef::Func(_) = import.ty {
                        import_count += 1;
                    }
                }
            }
            Payload::ExportSection(exports) => {
                for export in exports {
                    let export = export.expect("failed to parse export");
                    if let ExternalKind::Func = export.kind {
                        function_indices.insert(export.name.to_string(), export.index);
                    }
                }
            }
            Payload::CodeSectionEntry(body) => {
                let func_index = import_count + defined_index;
                let mut locals_reader = body.get_locals_reader().expect("failed to read locals");
                for _ in 0..locals_reader.get_count() {
                    locals_reader
                        .read()
                        .expect("failed to parse local declaration");
                }

                let mut operators = body
                    .get_operators_reader()
                    .expect("failed to read operators");
                let mut count: u32 = 0;
                while !operators.eof() {
                    match operators.read().expect("failed to read operator") {
                        Operator::If { .. } => count += 1,
                        _ => {}
                    }
                }
                i32_if_counts.insert(func_index, count);
                defined_index += 1;
            }
            _ => {}
        }
    }

    let and_true_index = *function_indices
        .get("and_true")
        .expect("expected and_true export");
    let and_false_index = *function_indices
        .get("and_false")
        .expect("expected and_false export");
    let or_true_index = *function_indices
        .get("or_true")
        .expect("expected or_true export");
    let or_false_index = *function_indices
        .get("or_false")
        .expect("expected or_false export");

    let and_true_count = *i32_if_counts
        .get(&and_true_index)
        .expect("missing and_true body");
    assert_eq!(and_true_count, 1, "flag && true should only emit the structural if");

    let and_false_count = *i32_if_counts
        .get(&and_false_index)
        .expect("missing and_false body");
    assert_eq!(and_false_count, 1, "flag && false should only emit the structural if");

    let or_true_count = *i32_if_counts
        .get(&or_true_index)
        .expect("missing or_true body");
    assert_eq!(or_true_count, 1, "flag || true should only emit the structural if");

    let or_false_count = *i32_if_counts
        .get(&or_false_index)
        .expect("missing or_false body");
    assert_eq!(or_false_count, 1, "flag || false should only emit the structural if");
}

#[test]
fn ast_simplifies_arithmetic_identities() {
    let source = r#"
fn add_zero(value: i32) -> i32 {
    value + 0
}

fn zero_add(value: i32) -> i32 {
    0 + value
}

fn mul_one(value: i32) -> i32 {
    value * 1
}

fn one_mul(value: i32) -> i32 {
    1 * value
}

fn mul_zero(value: i32) -> i32 {
    value * 0
}

fn zero_mul(value: i32) -> i32 {
    0 * value
}

fn div_one(value: i32) -> i32 {
    value / 1
}

fn main() -> i32 {
    0
}
"#;

    let wasm = compile_with_stage1(source);

    #[derive(Default)]
    struct OperationStats {
        i32_add: u32,
        i32_mul: u32,
        i32_div_s: u32,
        local_gets: u32,
        const_values: Vec<i32>,
    }

    let parser = Parser::new(0);
    let mut import_count: u32 = 0;
    let mut defined_index: u32 = 0;
    let mut function_indices = std::collections::HashMap::new();
    let mut stats: std::collections::HashMap<u32, OperationStats> = std::collections::HashMap::new();

    for payload in parser.parse_all(&wasm) {
        let payload = payload.expect("failed to parse wasm payload");
        match payload {
            Payload::ImportSection(imports) => {
                for import in imports {
                    let import = import.expect("failed to parse import");
                    if let TypeRef::Func(_) = import.ty {
                        import_count += 1;
                    }
                }
            }
            Payload::ExportSection(exports) => {
                for export in exports {
                    let export = export.expect("failed to parse export");
                    if let ExternalKind::Func = export.kind {
                        function_indices.insert(export.name.to_string(), export.index);
                    }
                }
            }
            Payload::CodeSectionEntry(body) => {
                let func_index = import_count + defined_index;
                let mut locals_reader = body.get_locals_reader().expect("failed to read locals");
                for _ in 0..locals_reader.get_count() {
                    locals_reader
                        .read()
                        .expect("failed to parse local declaration");
                }

                let mut operators = body
                    .get_operators_reader()
                    .expect("failed to read operators");
                let entry = stats.entry(func_index).or_default();
                while !operators.eof() {
                    match operators.read().expect("failed to read operator") {
                        Operator::I32Add { .. } => entry.i32_add += 1,
                        Operator::I32Mul { .. } => entry.i32_mul += 1,
                        Operator::I32DivS { .. } => entry.i32_div_s += 1,
                        Operator::LocalGet { .. } => entry.local_gets += 1,
                        Operator::I32Const { value } => entry.const_values.push(value),
                        Operator::End => break,
                        _ => {}
                    }
                }
                defined_index += 1;
            }
            _ => {}
        }
    }

    let add_zero_index = *function_indices
        .get("add_zero")
        .expect("expected add_zero export");
    let add_zero_stats = stats
        .get(&add_zero_index)
        .expect("missing add_zero stats");
    assert_eq!(add_zero_stats.i32_add, 0, "value + 0 should not emit i32.add");
    assert_eq!(
        add_zero_stats.local_gets, 1,
        "value + 0 should only read the parameter"
    );
    assert!(
        add_zero_stats.const_values.is_empty(),
        "value + 0 should not emit constants"
    );

    let zero_add_index = *function_indices
        .get("zero_add")
        .expect("expected zero_add export");
    let zero_add_stats = stats
        .get(&zero_add_index)
        .expect("missing zero_add stats");
    assert_eq!(zero_add_stats.i32_add, 0, "0 + value should not emit i32.add");
    assert_eq!(
        zero_add_stats.local_gets, 1,
        "0 + value should only read the parameter"
    );
    assert!(
        zero_add_stats.const_values.is_empty(),
        "0 + value should not emit constants"
    );

    let mul_one_index = *function_indices
        .get("mul_one")
        .expect("expected mul_one export");
    let mul_one_stats = stats
        .get(&mul_one_index)
        .expect("missing mul_one stats");
    assert_eq!(mul_one_stats.i32_mul, 0, "value * 1 should not emit i32.mul");
    assert_eq!(
        mul_one_stats.local_gets, 1,
        "value * 1 should only read the parameter"
    );
    assert!(
        mul_one_stats.const_values.is_empty(),
        "value * 1 should not emit constants"
    );

    let one_mul_index = *function_indices
        .get("one_mul")
        .expect("expected one_mul export");
    let one_mul_stats = stats
        .get(&one_mul_index)
        .expect("missing one_mul stats");
    assert_eq!(one_mul_stats.i32_mul, 0, "1 * value should not emit i32.mul");
    assert_eq!(
        one_mul_stats.local_gets, 1,
        "1 * value should only read the parameter"
    );
    assert!(
        one_mul_stats.const_values.is_empty(),
        "1 * value should not emit constants"
    );

    let mul_zero_index = *function_indices
        .get("mul_zero")
        .expect("expected mul_zero export");
    let mul_zero_stats = stats
        .get(&mul_zero_index)
        .expect("missing mul_zero stats");
    assert_eq!(mul_zero_stats.i32_mul, 0, "value * 0 should not emit i32.mul");
    assert_eq!(mul_zero_stats.local_gets, 0, "value * 0 should not read locals");
    assert_eq!(
        mul_zero_stats.const_values,
        vec![0],
        "value * 0 should only emit an i32.const 0"
    );

    let zero_mul_index = *function_indices
        .get("zero_mul")
        .expect("expected zero_mul export");
    let zero_mul_stats = stats
        .get(&zero_mul_index)
        .expect("missing zero_mul stats");
    assert_eq!(zero_mul_stats.i32_mul, 0, "0 * value should not emit i32.mul");
    assert_eq!(zero_mul_stats.local_gets, 0, "0 * value should not read locals");
    assert_eq!(
        zero_mul_stats.const_values,
        vec![0],
        "0 * value should only emit an i32.const 0"
    );

    let div_one_index = *function_indices
        .get("div_one")
        .expect("expected div_one export");
    let div_one_stats = stats
        .get(&div_one_index)
        .expect("missing div_one stats");
    assert_eq!(div_one_stats.i32_div_s, 0, "value / 1 should not emit i32.div_s");
    assert_eq!(
        div_one_stats.local_gets, 1,
        "value / 1 should only read the parameter"
    );
    assert!(
        div_one_stats.const_values.is_empty(),
        "value / 1 should not emit constants"
    );
}

#[test]
fn ast_simplifies_bitwise_and_shift_identities() {
    let source = r#"
fn and_zero(value: i32) -> i32 {
    value & 0
}

fn zero_and(value: i32) -> i32 {
    0 & value
}

fn and_all_bits(value: i32) -> i32 {
    value & -1
}

fn all_bits_and(value: i32) -> i32 {
    -1 & value
}

fn or_zero(value: i32) -> i32 {
    value | 0
}

fn zero_or(value: i32) -> i32 {
    0 | value
}

fn or_all_bits(value: i32) -> i32 {
    value | -1
}

fn all_bits_or(value: i32) -> i32 {
    -1 | value
}

fn shl_zero(value: i32) -> i32 {
    value << 0
}

fn zero_shl(value: i32) -> i32 {
    0 << value
}

fn shr_zero(value: i32) -> i32 {
    value >> 0
}

fn zero_shr(value: i32) -> i32 {
    0 >> value
}

fn main() -> i32 {
    0
}
"#;

    let wasm = compile_with_stage1(source);

    #[derive(Default)]
    struct OperationStats {
        i32_and: u32,
        i32_or: u32,
        i32_shl: u32,
        i32_shr_s: u32,
        local_gets: u32,
        const_values: Vec<i32>,
    }

    let parser = Parser::new(0);
    let mut import_count: u32 = 0;
    let mut defined_index: u32 = 0;
    let mut function_indices = std::collections::HashMap::new();
    let mut stats: std::collections::HashMap<u32, OperationStats> = std::collections::HashMap::new();

    for payload in parser.parse_all(&wasm) {
        let payload = payload.expect("failed to parse wasm payload");
        match payload {
            Payload::ImportSection(imports) => {
                for import in imports {
                    let import = import.expect("failed to parse import");
                    if let TypeRef::Func(_) = import.ty {
                        import_count += 1;
                    }
                }
            }
            Payload::ExportSection(exports) => {
                for export in exports {
                    let export = export.expect("failed to parse export");
                    if let ExternalKind::Func = export.kind {
                        function_indices.insert(export.name.to_string(), export.index);
                    }
                }
            }
            Payload::CodeSectionEntry(body) => {
                let func_index = import_count + defined_index;
                let mut locals_reader = body.get_locals_reader().expect("failed to read locals");
                for _ in 0..locals_reader.get_count() {
                    locals_reader
                        .read()
                        .expect("failed to parse local declaration");
                }

                let mut operators = body
                    .get_operators_reader()
                    .expect("failed to read operators");
                let entry = stats.entry(func_index).or_default();
                while !operators.eof() {
                    match operators.read().expect("failed to read operator") {
                        Operator::I32And { .. } => entry.i32_and += 1,
                        Operator::I32Or { .. } => entry.i32_or += 1,
                        Operator::I32Shl { .. } => entry.i32_shl += 1,
                        Operator::I32ShrS { .. } => entry.i32_shr_s += 1,
                        Operator::LocalGet { .. } => entry.local_gets += 1,
                        Operator::I32Const { value } => entry.const_values.push(value),
                        Operator::End => break,
                        _ => {}
                    }
                }
                defined_index += 1;
            }
            _ => {}
        }
    }

    let and_zero_index = *function_indices
        .get("and_zero")
        .expect("expected and_zero export");
    let and_zero_stats = stats
        .get(&and_zero_index)
        .expect("missing and_zero stats");
    assert_eq!(and_zero_stats.i32_and, 0, "value & 0 should not emit i32.and");
    assert_eq!(and_zero_stats.local_gets, 0, "value & 0 should not read locals");
    assert_eq!(
        and_zero_stats.const_values,
        vec![0],
        "value & 0 should only emit i32.const 0"
    );

    let zero_and_index = *function_indices
        .get("zero_and")
        .expect("expected zero_and export");
    let zero_and_stats = stats
        .get(&zero_and_index)
        .expect("missing zero_and stats");
    assert_eq!(zero_and_stats.i32_and, 0, "0 & value should not emit i32.and");
    assert_eq!(zero_and_stats.local_gets, 0, "0 & value should not read locals");
    assert_eq!(
        zero_and_stats.const_values,
        vec![0],
        "0 & value should only emit i32.const 0"
    );

    let and_all_bits_index = *function_indices
        .get("and_all_bits")
        .expect("expected and_all_bits export");
    let and_all_bits_stats = stats
        .get(&and_all_bits_index)
        .expect("missing and_all_bits stats");
    assert_eq!(
        and_all_bits_stats.i32_and, 0,
        "value & -1 should not emit i32.and"
    );
    assert_eq!(
        and_all_bits_stats.local_gets, 1,
        "value & -1 should only read the parameter"
    );
    assert!(
        and_all_bits_stats.const_values.is_empty(),
        "value & -1 should not emit constants"
    );

    let all_bits_and_index = *function_indices
        .get("all_bits_and")
        .expect("expected all_bits_and export");
    let all_bits_and_stats = stats
        .get(&all_bits_and_index)
        .expect("missing all_bits_and stats");
    assert_eq!(
        all_bits_and_stats.i32_and, 0,
        "-1 & value should not emit i32.and"
    );
    assert_eq!(all_bits_and_stats.local_gets, 1, "-1 & value should read the parameter");
    assert!(
        all_bits_and_stats.const_values.is_empty(),
        "-1 & value should not emit constants"
    );

    let or_zero_index = *function_indices
        .get("or_zero")
        .expect("expected or_zero export");
    let or_zero_stats = stats
        .get(&or_zero_index)
        .expect("missing or_zero stats");
    assert_eq!(or_zero_stats.i32_or, 0, "value | 0 should not emit i32.or");
    assert_eq!(or_zero_stats.local_gets, 1, "value | 0 should read the parameter");
    assert!(
        or_zero_stats.const_values.is_empty(),
        "value | 0 should not emit constants"
    );

    let zero_or_index = *function_indices
        .get("zero_or")
        .expect("expected zero_or export");
    let zero_or_stats = stats
        .get(&zero_or_index)
        .expect("missing zero_or stats");
    assert_eq!(zero_or_stats.i32_or, 0, "0 | value should not emit i32.or");
    assert_eq!(zero_or_stats.local_gets, 1, "0 | value should read the parameter");
    assert!(
        zero_or_stats.const_values.is_empty(),
        "0 | value should not emit constants"
    );

    let or_all_bits_index = *function_indices
        .get("or_all_bits")
        .expect("expected or_all_bits export");
    let or_all_bits_stats = stats
        .get(&or_all_bits_index)
        .expect("missing or_all_bits stats");
    assert_eq!(or_all_bits_stats.i32_or, 0, "value | -1 should not emit i32.or");
    assert_eq!(or_all_bits_stats.local_gets, 0, "value | -1 should not read locals");
    assert_eq!(
        or_all_bits_stats.const_values,
        vec![-1],
        "value | -1 should only emit i32.const -1"
    );

    let all_bits_or_index = *function_indices
        .get("all_bits_or")
        .expect("expected all_bits_or export");
    let all_bits_or_stats = stats
        .get(&all_bits_or_index)
        .expect("missing all_bits_or stats");
    assert_eq!(all_bits_or_stats.i32_or, 0, "-1 | value should not emit i32.or");
    assert_eq!(all_bits_or_stats.local_gets, 0, "-1 | value should not read locals");
    assert_eq!(
        all_bits_or_stats.const_values,
        vec![-1],
        "-1 | value should only emit i32.const -1"
    );

    let shl_zero_index = *function_indices
        .get("shl_zero")
        .expect("expected shl_zero export");
    let shl_zero_stats = stats
        .get(&shl_zero_index)
        .expect("missing shl_zero stats");
    assert_eq!(shl_zero_stats.i32_shl, 0, "value << 0 should not emit i32.shl");
    assert_eq!(shl_zero_stats.local_gets, 1, "value << 0 should read the parameter");
    assert!(
        shl_zero_stats.const_values.is_empty(),
        "value << 0 should not emit constants"
    );

    let zero_shl_index = *function_indices
        .get("zero_shl")
        .expect("expected zero_shl export");
    let zero_shl_stats = stats
        .get(&zero_shl_index)
        .expect("missing zero_shl stats");
    assert_eq!(zero_shl_stats.i32_shl, 0, "0 << value should not emit i32.shl");
    assert_eq!(zero_shl_stats.local_gets, 0, "0 << value should not read locals");
    assert_eq!(
        zero_shl_stats.const_values,
        vec![0],
        "0 << value should only emit i32.const 0"
    );

    let shr_zero_index = *function_indices
        .get("shr_zero")
        .expect("expected shr_zero export");
    let shr_zero_stats = stats
        .get(&shr_zero_index)
        .expect("missing shr_zero stats");
    assert_eq!(shr_zero_stats.i32_shr_s, 0, "value >> 0 should not emit i32.shr_s");
    assert_eq!(shr_zero_stats.local_gets, 1, "value >> 0 should read the parameter");
    assert!(
        shr_zero_stats.const_values.is_empty(),
        "value >> 0 should not emit constants"
    );

    let zero_shr_index = *function_indices
        .get("zero_shr")
        .expect("expected zero_shr export");
    let zero_shr_stats = stats
        .get(&zero_shr_index)
        .expect("missing zero_shr stats");
    assert_eq!(zero_shr_stats.i32_shr_s, 0, "0 >> value should not emit i32.shr_s");
    assert_eq!(zero_shr_stats.local_gets, 0, "0 >> value should not read locals");
    assert_eq!(
        zero_shr_stats.const_values,
        vec![0],
        "0 >> value should only emit i32.const 0"
    );
}
