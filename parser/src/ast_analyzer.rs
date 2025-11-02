use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use swc_common::sync::Lrc;
use swc_common::{FileName, SourceMap};
use swc_ecma_ast::*;
use swc_ecma_parser::{Parser, StringInput, Syntax, TsSyntax};
use swc_ecma_visit::{Visit, VisitWith};

#[derive(Debug, Clone)]
pub struct FnSign {
    pub params: Vec<FnParams>,
    pub return_type: WitType,
}

#[derive(Debug, Clone)]
pub struct FnParams {
    pub name: String,
    pub param_type: WitType,
}

#[derive(Debug, Clone)]
pub enum WitType {
    Primitive(String),      
    List(Box<WitType>),     
    Option(Box<WitType>),   
    Result(Box<WitType>, Box<WitType>), 
    Tuple(Vec<WitType>),    
    Record(String),         
    Variant(String),        
    Enum(String),           
}

impl WitType {
    pub fn to_wit_string(&self) -> String {
        match self {
            WitType::Primitive(p) => p.clone(),
            WitType::List(inner) => format!("list<{}>", inner.to_wit_string()),
            WitType::Option(inner) => format!("option<{}>", inner.to_wit_string()),
            WitType::Result(ok, err) => {
                format!("result<{}, {}>", ok.to_wit_string(), err.to_wit_string())
            }
            WitType::Tuple(types) => {
                let type_strs: Vec<String> = types.iter().map(|t| t.to_wit_string()).collect();
                format!("tuple<{}>", type_strs.join(", "))
            }
            WitType::Record(name) | WitType::Variant(name) | WitType::Enum(name) => name.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RecordDef {
    pub name: String,
    pub fields: Vec<(String, WitType)>,
}

#[derive(Debug, Clone)]
pub struct VariantDef {
    pub name: String,
    pub cases: Vec<VariantCase>,
}

#[derive(Debug, Clone)]
pub struct VariantCase {
    pub name: String,
    pub payload: Option<WitType>,
}

#[derive(Debug, Clone)]
pub struct EnumDef {
    pub name: String,
    pub cases: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ModuleAnalysis {
    pub functions: HashMap<String, FnSign>,
    pub records: HashMap<String, RecordDef>,
    pub variants: HashMap<String, VariantDef>,
    pub enums: HashMap<String, EnumDef>,
}

struct TypeAnalyzer {
    target_function: String,
    analysis: ModuleAnalysis,
}

impl TypeAnalyzer {
    fn new(target_function: String) -> Self {
        Self {
            target_function,
            analysis: ModuleAnalysis {
                functions: HashMap::new(),
                records: HashMap::new(),
                variants: HashMap::new(),
                enums: HashMap::new(),
            },
        }
    }

    fn analyze_ts_type(&self, ts_type: &TsType) -> WitType {
        match ts_type {
            TsType::TsKeywordType(keyword) => self.map_keyword_type(keyword.kind),
            
            TsType::TsTypeRef(type_ref) => {
                if let TsEntityName::Ident(ident) = &type_ref.type_name {
                    let type_name = ident.sym.to_string();
                    
                    match type_name.as_str() {
                        "Array" => {
                            if let Some(params) = &type_ref.type_params {
                                if let Some(first) = params.params.first() {
                                    return WitType::List(Box::new(self.analyze_ts_type(first)));
                                }
                            }
                            WitType::List(Box::new(WitType::Primitive("string".to_string())))
                        }
                        "Promise" => {
                            if let Some(params) = &type_ref.type_params {
                                if let Some(first) = params.params.first() {
                                    return self.analyze_ts_type(first);
                                }
                            }
                            WitType::Primitive("string".to_string())
                        }
                        _ => {
                            
                            if self.analysis.records.contains_key(&type_name) {
                                WitType::Record(self.to_kebab_case(&type_name))
                            } else if self.analysis.variants.contains_key(&type_name) {
                                WitType::Variant(self.to_kebab_case(&type_name))
                            } else if self.analysis.enums.contains_key(&type_name) {
                                WitType::Enum(self.to_kebab_case(&type_name))
                            } else {
                                
                                WitType::Record(self.to_kebab_case(&type_name))
                            }
                        }
                    }
                } else {
                    WitType::Primitive("string".to_string())
                }
            }
            
            TsType::TsArrayType(array) => {
                WitType::List(Box::new(self.analyze_ts_type(&array.elem_type)))
            }
            
            TsType::TsTupleType(tuple) => {
                let types = tuple.elem_types.iter()
                    .map(|elem| self.analyze_ts_type(&elem.ty))
                    .collect();
                WitType::Tuple(types)
            }
            
            TsType::TsUnionOrIntersectionType(union) => {
                
                if self.is_option_union(union) {
                    if let Some(inner_type) = self.extract_option_inner(union) {
                        return WitType::Option(Box::new(inner_type));
                    }
                }
                
                
                WitType::Variant("variant".to_string())
            }
            
            TsType::TsTypeLit(_type_lit) => {
                
                WitType::Record("record".to_string())
            }
            
            _ => WitType::Primitive("string".to_string()),
        }
    }

    fn map_keyword_type(&self, kind: TsKeywordTypeKind) -> WitType {
        let type_str = match kind {
            TsKeywordTypeKind::TsStringKeyword => "string",
            TsKeywordTypeKind::TsNumberKeyword => "f64",
            TsKeywordTypeKind::TsBooleanKeyword => "bool",
            TsKeywordTypeKind::TsBigIntKeyword => "s64",
            TsKeywordTypeKind::TsVoidKeyword => "string",
            TsKeywordTypeKind::TsUndefinedKeyword => "string",
            TsKeywordTypeKind::TsNullKeyword => "string",
            _ => "string",
        };
        WitType::Primitive(type_str.to_string())
    }

    fn is_option_union(&self, union: &TsUnionOrIntersectionType) -> bool {
        let types = match union {
            TsUnionOrIntersectionType::TsUnionType(u) => &u.types,
            TsUnionOrIntersectionType::TsIntersectionType(i) => &i.types,
        };
        
        types.iter().any(|t| matches!(**t, 
            TsType::TsKeywordType(TsKeywordType { kind: TsKeywordTypeKind::TsNullKeyword, .. }) |
            TsType::TsKeywordType(TsKeywordType { kind: TsKeywordTypeKind::TsUndefinedKeyword, .. })
        ))
    }

    fn extract_option_inner(&self, union: &TsUnionOrIntersectionType) -> Option<WitType> {
        let types = match union {
            TsUnionOrIntersectionType::TsUnionType(u) => &u.types,
            TsUnionOrIntersectionType::TsIntersectionType(i) => &i.types,
        };
        
        for ts_type in types {
            if !matches!(**ts_type, 
                TsType::TsKeywordType(TsKeywordType { kind: TsKeywordTypeKind::TsNullKeyword, .. }) |
                TsType::TsKeywordType(TsKeywordType { kind: TsKeywordTypeKind::TsUndefinedKeyword, .. })
            ) {
                return Some(self.analyze_ts_type(ts_type));
            }
        }
        None
    }

    fn to_kebab_case(&self, s: &str) -> String {
        let mut result = String::new();
        let mut chars = s.chars().peekable();
        
        while let Some(c) = chars.next() {
            if c.is_uppercase() {
                if !result.is_empty() {
                    result.push('-');
                }
                result.push(c.to_ascii_lowercase());
            } else {
                result.push(c);
            }
        }
        
        result
    }

    fn analyze_interface(&mut self, decl: &TsInterfaceDecl) {
        let mut fields = Vec::new();
        
        for member in &decl.body.body {
            if let TsTypeElement::TsPropertySignature(prop) = member {
                if let Some(ident) = prop.key.as_ident() {
                    let field_name = ident.sym.to_string();
                    let field_type = if let Some(type_ann) = &prop.type_ann {
                        self.analyze_ts_type(&type_ann.type_ann)
                    } else {
                        WitType::Primitive("string".to_string())
                    };
                    
                    fields.push((self.to_kebab_case(&field_name), field_type));
                }
            }
        }
        
        let record_name = self.to_kebab_case(&decl.id.sym.to_string());
        self.analysis.records.insert(
            decl.id.sym.to_string(),
            RecordDef {
                name: record_name,
                fields,
            },
        );
    }

    fn analyze_type_alias(&mut self, decl: &TsTypeAliasDecl) {
        let type_name = decl.id.sym.to_string();
        
        match &*decl.type_ann {
            TsType::TsTypeLit(type_lit) => {
                
                let mut fields = Vec::new();
                
                for member in &type_lit.members {
                    if let TsTypeElement::TsPropertySignature(prop) = member {
                        if let Some(ident) = prop.key.as_ident() {
                            let field_name = ident.sym.to_string();
                            let field_type = if let Some(type_ann) = &prop.type_ann {
                                self.analyze_ts_type(&type_ann.type_ann)
                            } else {
                                WitType::Primitive("string".to_string())
                            };
                            
                            fields.push((self.to_kebab_case(&field_name), field_type));
                        }
                    }
                }
                
                let record_name = self.to_kebab_case(&type_name);
                self.analysis.records.insert(
                    type_name,
                    RecordDef {
                        name: record_name,
                        fields,
                    },
                );
            }
            
            TsType::TsUnionOrIntersectionType(union) => {
                
                if self.is_enum_union(union) {
                    let cases = self.extract_enum_cases(union);
                    let enum_name = self.to_kebab_case(&type_name);
                    self.analysis.enums.insert(
                        type_name,
                        EnumDef {
                            name: enum_name,
                            cases,
                        },
                    );
                } else {
                    let cases = self.extract_variant_cases(union);
                    let variant_name = self.to_kebab_case(&type_name);
                    self.analysis.variants.insert(
                        type_name,
                        VariantDef {
                            name: variant_name,
                            cases,
                        },
                    );
                }
            }
            
            _ => {
                
            }
        }
    }

    fn is_enum_union(&self, union: &TsUnionOrIntersectionType) -> bool {
        let types = match union {
            TsUnionOrIntersectionType::TsUnionType(u) => &u.types,
            TsUnionOrIntersectionType::TsIntersectionType(i) => &i.types,
        };
        
        types.iter().all(|t| matches!(**t, 
            TsType::TsLitType(_)
        ))
    }

    fn extract_enum_cases(&self, union: &TsUnionOrIntersectionType) -> Vec<String> {
        let types = match union {
            TsUnionOrIntersectionType::TsUnionType(u) => &u.types,
            TsUnionOrIntersectionType::TsIntersectionType(i) => &i.types,
        };
        
        types.iter()
            .filter_map(|t| {
                if let TsType::TsLitType(lit) = &**t {
                    match &lit.lit {
                        TsLit::Str(s) => {
                            if let Some(str_val) = s.value.as_str() {
                                Some(self.to_kebab_case(str_val))
                            } else {
                                None
                            }
                        }
                        TsLit::Number(n) => Some(format!("variant-{}", n.value as i64)),
                        _ => None,
                    }
                } else {
                    None
                }
            })
            .collect()
    }

    fn extract_variant_cases(&self, union: &TsUnionOrIntersectionType) -> Vec<VariantCase> {
        let types = match union {
            TsUnionOrIntersectionType::TsUnionType(u) => &u.types,
            TsUnionOrIntersectionType::TsIntersectionType(i) => &i.types,
        };
        
        types.iter()
            .map(|t| {
                match &**t {
                    TsType::TsLitType(lit) => {
                        let name = match &lit.lit {
                            TsLit::Str(s) => {
                                if let Some(str_val) = s.value.as_str() {
                                    self.to_kebab_case(str_val)
                                } else {
                                    "unknown".to_string()
                                }
                            }
                            TsLit::Number(n) => format!("variant-{}", n.value as i64),
                            _ => "unknown".to_string(),
                        };
                        VariantCase {
                            name,
                            payload: None,
                        }
                    }
                    _ => {
                        VariantCase {
                            name: "variant".to_string(),
                            payload: Some(self.analyze_ts_type(t)),
                        }
                    }
                }
            })
            .collect()
    }

    fn analyze_enum(&mut self, decl: &TsEnumDecl) {
        let cases: Vec<String> = decl.members.iter()
            .filter_map(|member| {
                if let TsEnumMemberId::Ident(ident) = &member.id {
                    Some(self.to_kebab_case(&ident.sym.to_string()))
                } else {
                    None
                }
            })
            .collect();
        
        let enum_name = self.to_kebab_case(&decl.id.sym.to_string());
        self.analysis.enums.insert(
            decl.id.sym.to_string(),
            EnumDef {
                name: enum_name,
                cases,
            },
        );
    }
}

impl Visit for TypeAnalyzer {
    fn visit_export_decl(&mut self, n: &ExportDecl) {
        match &n.decl {
            Decl::Fn(fn_decl) => {
                let fn_name = fn_decl.ident.sym.to_string();
                if fn_name == self.target_function {
                    let signature = self.analyze_function(&fn_decl.function);
                    self.analysis.functions.insert(fn_name, signature);
                }
            }
            Decl::TsInterface(interface_decl) => {
                self.analyze_interface(interface_decl);
            }
            Decl::TsTypeAlias(type_alias) => {
                self.analyze_type_alias(type_alias);
            }
            Decl::TsEnum(enum_decl) => {
                self.analyze_enum(enum_decl);
            }
            _ => {}
        }
        n.visit_children_with(self);
    }

    fn visit_export_default_decl(&mut self, n: &ExportDefaultDecl) {
        if let DefaultDecl::Fn(fn_expr) = &n.decl {
            if let Some(ident) = &fn_expr.ident {
                if ident.sym.to_string() == self.target_function {
                    let signature = self.analyze_function(&fn_expr.function);
                    self.analysis.functions.insert(self.target_function.clone(), signature);
                }
            }
        }
        n.visit_children_with(self);
    }

    fn visit_ts_interface_decl(&mut self, n: &TsInterfaceDecl) {
        self.analyze_interface(n);
        n.visit_children_with(self);
    }

    fn visit_ts_type_alias_decl(&mut self, n: &TsTypeAliasDecl) {
        self.analyze_type_alias(n);
        n.visit_children_with(self);
    }

    fn visit_ts_enum_decl(&mut self, n: &TsEnumDecl) {
        self.analyze_enum(n);
        n.visit_children_with(self);
    }
}

impl TypeAnalyzer {
    fn analyze_function(&self, func: &Function) -> FnSign {
        let mut params = Vec::new();

        for param in &func.params {
            if let Pat::Ident(ident) = &param.pat {
                let name = ident.sym.to_string();
                let param_type = if let Some(type_ann) = &ident.type_ann {
                    self.analyze_ts_type(&type_ann.type_ann)
                } else {
                    WitType::Primitive("string".to_string())
                };
                params.push(FnParams { name, param_type });
            }
        }

        let return_type = if let Some(ts_type) = &func.return_type {
            self.analyze_ts_type(&ts_type.type_ann)
        } else {
            WitType::Primitive("string".to_string())
        };

        FnSign {
            params,
            return_type,
        }
    }
}

pub fn anal_fn(ts_file: &Path, function_name: &str) -> Result<ModuleAnalysis> {
    let source = fs::read_to_string(ts_file).context("Failed to read TypeScript file")?;

    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(
        FileName::Custom(ts_file.to_string_lossy().to_string()).into(),
        source,
    );

    let syntax = Syntax::Typescript(TsSyntax {
        tsx: false,
        decorators: false,
        dts: false,
        no_early_errors: true,
        disallow_ambiguous_jsx_like: true,
    });

    let mut parser = Parser::new(syntax, StringInput::from(&*fm), None);

    let module = parser
        .parse_module()
        .map_err(|e| anyhow::anyhow!("Failed to parse TypeScript: {:?}", e))?;

    let mut visitor = TypeAnalyzer::new(function_name.to_string());

    module.visit_with(&mut visitor);

    if !visitor.analysis.functions.contains_key(function_name) {
        anyhow::bail!(
            "Function '{}' not found in {}",
            function_name,
            ts_file.display()
        );
    }

    Ok(visitor.analysis)
}