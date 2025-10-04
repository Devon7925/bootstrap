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
                                    "unexpected multiple constants in constant_expr body"
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
