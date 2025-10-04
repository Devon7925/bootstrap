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
