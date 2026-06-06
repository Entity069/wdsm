use std::collections::{HashMap, HashSet};

use crate::errors::WdsmError;
use crate::ir::*;

pub fn validate(ir: &mut WitIR) -> Result<Vec<String>, WdsmError> {
    let mut warnings = Vec::new();

    check_unique_type(ir)?;
    check_unique_field(ir)?;
    check_unique_param(ir)?;
    validate_names(ir)?;
    check_refs(ir)?;
    toposort_ty(ir)?;
    warnings.extend(audit_confidence(ir));

    Ok(warnings)
}

fn check_unique_type(ir: &WitIR) -> Result<(), WdsmError> {
    let mut seen: HashMap<&str, &SourceSpan> = HashMap::new();

    for td in &ir.types {
        let name = td.name();
        let source = match td {
            TypeDef::Record(r) => &r.source,
            TypeDef::Variant(v) => &v.source,
            TypeDef::Enum(e) => &e.source,
            TypeDef::Flags(f) => &f.source,
            TypeDef::Alias(a) => &a.source,
        };

        if let Some(prev) = seen.get(name) {
            return Err(WdsmError::DuplicateType {
                name: name.to_string(),
                first: format!("{}:{}", prev.file, prev.line),
                second: format!("{}:{}", source.file, source.line),
            });
        }
        seen.insert(name, source);
    }

    Ok(())
}

fn check_unique_field(ir: &WitIR) -> Result<(), WdsmError> {
    for td in &ir.types {
        if let TypeDef::Record(rec) = td {
            let mut seen = HashSet::new();
            for field in &rec.fields {
                if !seen.insert(&field.wit_name) {
                    return Err(WdsmError::DuplicateField {
                        record: rec.name.clone(),
                        field: field.name.clone(),
                    });
                }
            }
        }
    }
    Ok(())
}

fn check_unique_param(ir: &WitIR) -> Result<(), WdsmError> {
    for func in &ir.functions {
        let mut seen = HashSet::new();
        for param in &func.params {
            if !seen.insert(&param.wit_name) {
                return Err(WdsmError::DuplicateParam {
                    function: func.name.clone(),
                    param: param.name.clone(),
                });
            }
        }
    }
    Ok(())
}

fn validate_names(ir: &WitIR) -> Result<(), WdsmError> {
    for td in &ir.types {
        crate::errors::validate_wit_name(td.wit_name())?;

        if let TypeDef::Record(rec) = td {
            for field in &rec.fields {
                crate::errors::validate_wit_name(&field.wit_name)?;
            }
        }
    }

    for func in &ir.functions {
        crate::errors::validate_wit_name(&func.wit_name)?;

        for param in &func.params {
            crate::errors::validate_wit_name(&param.wit_name)?;
        }
    }

    Ok(())
}

fn check_refs(ir: &WitIR) -> Result<(), WdsmError> {
    let known: HashSet<&str> = ir.types.iter().map(|td| td.name()).collect();

    for td in &ir.types {
        let deps = td.dependencies();
        let source = match td {
            TypeDef::Record(r) => &r.source,
            TypeDef::Variant(v) => &v.source,
            TypeDef::Enum(e) => &e.source,
            TypeDef::Flags(f) => &f.source,
            TypeDef::Alias(a) => &a.source,
        };

        for dep in deps {
            if !known.contains(dep) {
                return Err(WdsmError::UnresolvedType {
                    name: dep.to_string(),
                    ref_in: td.name().to_string(),
                    src_file: source.file.clone(),
                    src_line: source.line,
                });
            }
        }
    }

    for func in &ir.functions {
        for param in &func.params {
            for named_ref in param.ty.named_refs() {
                if !known.contains(named_ref) {
                    return Err(WdsmError::UnresolvedType {
                        name: named_ref.to_string(),
                        ref_in: format!("{}({})", func.name, param.name),
                        src_file: func.source.file.clone(),
                        src_line: func.source.line,
                    });
                }
            }
        }

        if let ReturnType::Type(ty) = &func.returns {
            for named_ref in ty.named_refs() {
                if !known.contains(named_ref) {
                    return Err(WdsmError::UnresolvedType {
                        name: named_ref.to_string(),
                        ref_in: format!("{} return type", func.name),
                        src_file: func.source.file.clone(),
                        src_line: func.source.line,
                    });
                }
            }
        }
    }

    Ok(())
}

fn toposort_ty(ir: &mut WitIR) -> Result<(), WdsmError> {
    let sorted = toposort(&ir.types)?;
    ir.types = sorted;
    Ok(())
}

pub fn toposort(types: &[TypeDef]) -> Result<Vec<TypeDef>, WdsmError> {
    let n = types.len();
    if n <= 1 {
        return Ok(types.to_vec());
    }

    let name_to_idx: HashMap<&str, usize> = types
        .iter()
        .enumerate()
        .map(|(i, td)| (td.name(), i))
        .collect();

    let mut in_deg = vec![0usize; n];
    let mut dependents: Vec<Vec<usize>> = vec![Vec::new(); n];

    for (i, td) in types.iter().enumerate() {
        for dep in td.dependencies() {
            if let Some(&dep_idx) = name_to_idx.get(dep) {
                dependents[dep_idx].push(i);
                in_deg[i] += 1;
            }
        }
    }

    let mut queue: Vec<usize> = (0..n).filter(|&i| in_deg[i] == 0).collect();
    let mut sorted: Vec<usize> = Vec::with_capacity(n);

    while let Some(idx) = queue.pop() {
        sorted.push(idx);
        for &dependent in &dependents[idx] {
            in_deg[dependent] -= 1;
            if in_deg[dependent] == 0 {
                queue.push(dependent);
            }
        }
    }

    if sorted.len() != n {
        let cycle_members: Vec<String> = (0..n)
            .filter(|i| in_deg[*i] > 0)
            .map(|i| types[i].name().to_string())
            .collect();

        return Err(WdsmError::CircularReference {
            chain: cycle_members.join(" → "),
        });
    }

    Ok(sorted.iter().map(|&i| types[i].clone()).collect())
}

fn audit_confidence(ir: &WitIR) -> Vec<String> {
    let mut warnings = Vec::new();

    for td in &ir.types {
        let source = match td {
            TypeDef::Record(r) => &r.source,
            TypeDef::Variant(v) => &v.source,
            TypeDef::Enum(e) => &e.source,
            TypeDef::Flags(f) => &f.source,
            TypeDef::Alias(a) => &a.source,
        };

        if source.confidence == Confidence::Fallback {
            warnings.push(format!(
                "{}:{}: type `{}` was inferred from config fallback — consider adding explicit type annotations",
                source.file, source.line, td.name()
            ));
        }
    }

    for func in &ir.functions {
        if func.source.confidence == Confidence::Fallback {
            warnings.push(format!(
                "{}:{}: function `{}` types from config fallback",
                func.source.file, func.source.line, func.name
            ));
        }
    }

    warnings
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_record(name: &str, field_types: Vec<(&str, IRType)>) -> TypeDef {
        TypeDef::Record(RecordDef {
            name: name.to_string(),
            wit_name: to_kebab_case(name),
            fields: field_types
                .into_iter()
                .map(|(fname, ftype)| FieldDef {
                    name: fname.to_string(),
                    wit_name: to_kebab_case(fname),
                    ty: ftype,
                    optional: false,
                })
                .collect(),
            source: SourceSpan::default(),
        })
    }

    #[test]
    fn test_toposort_simple() {
        let types = vec![
            make_record("B", vec![("a", IRType::Named("A".to_string()))]),
            make_record("A", vec![("x", IRType::String)]),
        ];

        let sorted = toposort(&types).unwrap();
        assert_eq!(sorted[0].name(), "A");
        assert_eq!(sorted[1].name(), "B");
    }

    #[test]
    fn test_toposort_no_deps() {
        let types = vec![
            make_record("A", vec![("x", IRType::String)]),
            make_record("B", vec![("y", IRType::F64)]),
        ];

        let sorted = toposort(&types).unwrap();
        assert_eq!(sorted.len(), 2);
    }

    #[test]
    fn test_circular_dependency_detected() {
        let types = vec![
            make_record("A", vec![("b", IRType::Named("B".to_string()))]),
            make_record("B", vec![("a", IRType::Named("A".to_string()))]),
        ];

        let result = toposort(&types);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("circular"));
    }

    #[test]
    fn test_duplicate_type_detected() {
        let mut ir = WitIR {
            package: "wdsm:test".to_string(),
            world_name: "test".to_string(),
            types: vec![
                make_record("User", vec![("name", IRType::String)]),
                make_record("User", vec![("email", IRType::String)]),
            ],
            functions: vec![],
            imports: vec![],
        };

        let result = validate(&mut ir);
        assert!(result.is_err());
    }

    #[test]
    fn test_unresolved_reference_detected() {
        let mut ir = WitIR {
            package: "wdsm:test".to_string(),
            world_name: "test".to_string(),
            types: vec![make_record(
                "order",
                vec![("user", IRType::Named("NonExistent".to_string()))],
            )],
            functions: vec![],
            imports: vec![],
        };

        let result = validate(&mut ir);
        assert!(result.is_err());
    }
}
