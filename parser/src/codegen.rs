use crate::ir::*;

pub fn generate_wit(ir: &WitIR) -> String {
    let mut wit = String::new();

    wit.push_str(&format!("package {};\n\n", ir.package));
    wit.push_str(&format!("world {} {{\n", ir.world_name));

    for td in &ir.types {
        match td {
            TypeDef::Record(rec) => emit_record(&mut wit, rec),
            TypeDef::Variant(var) => emit_variant(&mut wit, var),
            TypeDef::Enum(enm) => emit_enum(&mut wit, enm),
            TypeDef::Flags(flags) => emit_flags(&mut wit, flags),
            TypeDef::Alias(alias) => emit_alias(&mut wit, alias),
        }
        wit.push('\n');
    }

    for func in &ir.functions {
        emit_function(&mut wit, func);
    }

    wit.push_str("}\n");

    wit
}

fn emit_record(wit: &mut String, rec: &RecordDef) {
    wit.push_str(&format!("  record {} {{\n", rec.wit_name));

    for field in &rec.fields {
        wit.push_str(&format!(
            "    {}: {},\n",
            field.wit_name,
            field.ty.to_wit_str()
        ));
    }

    wit.push_str("  }\n");
}

fn emit_variant(wit: &mut String, var: &VariantDef) {
    wit.push_str(&format!("  variant {} {{\n", var.wit_name));

    for case in &var.cases {
        if let Some(payload) = &case.payload {
            wit.push_str(&format!(
                "    {}({}),\n",
                case.name,
                payload.to_wit_str()
            ));
        } else {
            wit.push_str(&format!("    {},\n", case.name));
        }
    }

    wit.push_str("  }\n");
}

fn emit_enum(wit: &mut String, enm: &EnumDef) {
    wit.push_str(&format!("  enum {} {{\n", enm.wit_name));

    for case in &enm.cases {
        wit.push_str(&format!("    {},\n", case));
    }

    wit.push_str("  }\n");
}

fn emit_flags(wit: &mut String, flags: &FlagsDef) {
    wit.push_str(&format!("  flags {} {{\n", flags.wit_name));

    for flag in &flags.flags {
        wit.push_str(&format!("    {},\n", flag));
    }

    wit.push_str("  }\n");
}

fn emit_alias(wit: &mut String, alias: &AliasDef) {
    wit.push_str(&format!(
        "  type {} = {};\n",
        alias.wit_name,
        alias.target.to_wit_str()
    ));
}

fn emit_function(wit: &mut String, func: &FunctionDef) {
    let params: Vec<String> = func
        .params
        .iter()
        .map(|p| format!("{}: {}", p.wit_name, p.ty.to_wit_str()))
        .collect();

    match &func.returns {
        ReturnType::None => {
            wit.push_str(&format!(
                "  export {}: func({});\n",
                func.wit_name,
                params.join(", ")
            ));
        }
        ReturnType::Type(ty) => {
            wit.push_str(&format!(
                "  export {}: func({}) -> {};\n",
                func.wit_name,
                params.join(", "),
                ty.to_wit_str()
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_simple_ir() -> WitIR {
        WitIR {
            package: "wdsm:hello".to_string(),
            world_name: "hello".to_string(),
            types: vec![],
            functions: vec![FunctionDef {
                name: "hello".to_string(),
                wit_name: "hello".to_string(),
                params: vec![ParamDef {
                    name: "name".to_string(),
                    wit_name: "name".to_string(),
                    ty: IRType::String,
                }],
                returns: ReturnType::Type(IRType::String),
                docs: None,
                source: SourceSpan::default(),
            }],
            imports: vec![],
        }
    }

    #[test]
    fn test_simple_function() {
        let ir = make_simple_ir();
        let wit = generate_wit(&ir);

        assert!(wit.contains("package wdsm:hello;"));
        assert!(wit.contains("world hello {"));
        assert!(wit.contains("export hello: func(name: string) -> string;"));
        assert!(wit.contains("}"));
    }

    #[test]
    fn test_void_return_omits_arrow() {
        let ir = WitIR {
            package: "wdsm:test".to_string(),
            world_name: "test".to_string(),
            types: vec![],
            functions: vec![FunctionDef {
                name: "doStuff".to_string(),
                wit_name: "do-stuff".to_string(),
                params: vec![ParamDef {
                    name: "x".to_string(),
                    wit_name: "x".to_string(),
                    ty: IRType::String,
                }],
                returns: ReturnType::None,
                docs: None,
                source: SourceSpan::default(),
            }],
            imports: vec![],
        };

        let wit = generate_wit(&ir);
        assert!(wit.contains("export do-stuff: func(x: string);"));
        assert!(!wit.contains("->"));
    }

    #[test]
    fn test_record_emission() {
        let ir = WitIR {
            package: "wdsm:test".to_string(),
            world_name: "test".to_string(),
            types: vec![TypeDef::Record(RecordDef {
                name: "User".to_string(),
                wit_name: "user".to_string(),
                fields: vec![
                    FieldDef {
                        name: "name".to_string(),
                        wit_name: "name".to_string(),
                        ty: IRType::String,
                        optional: false,
                    },
                    FieldDef {
                        name: "age".to_string(),
                        wit_name: "age".to_string(),
                        ty: IRType::F64,
                        optional: false,
                    },
                ],
                source: SourceSpan::default(),
            })],
            functions: vec![],
            imports: vec![],
        };

        let wit = generate_wit(&ir);
        assert!(wit.contains("record user {"));
        assert!(wit.contains("name: string,"));
        assert!(wit.contains("age: f64,"));
    }

    #[test]
    fn test_enum_emission() {
        let ir = WitIR {
            package: "wdsm:test".to_string(),
            world_name: "test".to_string(),
            types: vec![TypeDef::Enum(EnumDef {
                name: "Status".to_string(),
                wit_name: "status".to_string(),
                cases: vec!["active".to_string(), "inactive".to_string()],
                source: SourceSpan::default(),
            })],
            functions: vec![],
            imports: vec![],
        };

        let wit = generate_wit(&ir);
        assert!(wit.contains("enum status {"));
        assert!(wit.contains("active,"));
        assert!(wit.contains("inactive,"));
    }

    #[test]
    fn test_full_module() {
        let ir = WitIR {
            package: "wdsm:user".to_string(),
            world_name: "user".to_string(),
            types: vec![TypeDef::Record(RecordDef {
                name: "User".to_string(),
                wit_name: "user".to_string(),
                fields: vec![
                    FieldDef {
                        name: "id".to_string(),
                        wit_name: "id".to_string(),
                        ty: IRType::F64,
                        optional: false,
                    },
                    FieldDef {
                        name: "name".to_string(),
                        wit_name: "name".to_string(),
                        ty: IRType::String,
                        optional: false,
                    },
                    FieldDef {
                        name: "roles".to_string(),
                        wit_name: "roles".to_string(),
                        ty: IRType::List(Box::new(IRType::String)),
                        optional: false,
                    },
                ],
                source: SourceSpan::default(),
            })],
            functions: vec![FunctionDef {
                name: "createUser".to_string(),
                wit_name: "create-user".to_string(),
                params: vec![
                    ParamDef {
                        name: "name".to_string(),
                        wit_name: "name".to_string(),
                        ty: IRType::String,
                    },
                    ParamDef {
                        name: "age".to_string(),
                        wit_name: "age".to_string(),
                        ty: IRType::F64,
                    },
                ],
                returns: ReturnType::Type(IRType::Named("User".to_string())),
                docs: None,
                source: SourceSpan::default(),
            }],
            imports: vec![],
        };

        let wit = generate_wit(&ir);

        assert!(wit.contains("package wdsm:user;"));
        assert!(wit.contains("world user {"));
        assert!(wit.contains("record user {"));
        assert!(wit.contains("roles: list<string>,"));
        assert!(wit.contains("export create-user: func(name: string, age: f64) -> user;"));
    }
}
