use anyhow::{Context, Result};
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use swc_common::sync::Lrc;
use swc_common::{FileName, SourceMap, Spanned};
use swc_ecma_ast::*;
use swc_ecma_parser::{Parser, StringInput, Syntax, TsSyntax};
use swc_ecma_visit::{Visit, VisitWith};

use crate::errors::WdsmError;
use crate::ir::*;
use super::{FrontendConfig, LanguageFrontend};

pub struct TypeScriptFrontend;

impl LanguageFrontend for TypeScriptFrontend {
    fn extract(&self, source: &Path, config: &FrontendConfig) -> Result<WitIR> {
        extract_ir(source, config)
    }

    fn language(&self) -> &str {
        "typescript"
    }
}

fn extract_ir(ts_file: &Path, config: &FrontendConfig) -> Result<WitIR> {
    let src_code = fs::read_to_string(ts_file).context("Failed to read TypeScript source")?;
    let src_name = ts_file.to_string_lossy().to_string();

    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(FileName::Custom(src_name.clone()).into(), src_code.clone());

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

    let target_fn = config.target_functions.as_ref().map(|fns| {
        fns.iter().cloned().collect::<HashSet<String>>()
    });

    let mut visitor = IRExtractor::new(
        src_name,
        target_fn,
        cm,
    );

    // collect type definitions
    module.visit_with(&mut visitor);
    if let Some(err) = visitor.errors.into_iter().next() {
        return Err(err);
    }

    let ir = WitIR {
        package: config.package_name.clone(),
        world_name: config.world_name.clone(),
        types: visitor.types,
        functions: visitor.functions,
        imports: vec![],
    };

    Ok(ir)
}

struct IRExtractor {
    src: String,
    target_fn: Option<HashSet<String>>,
    src_map: Lrc<SourceMap>,
    types: Vec<TypeDef>,
    functions: Vec<FunctionDef>,
    known_types: HashSet<String>,
    errors: Vec<anyhow::Error>,
}

impl IRExtractor {
    fn new(src: String, target_fn: Option<HashSet<String>>, src_map: Lrc<SourceMap>) -> Self {
        Self {
            src,
            target_fn,
            src_map,
            types: Vec::new(),
            functions: Vec::new(),
            known_types: HashSet::new(),
            errors: Vec::new(),
        }
    }

    fn extract_fn_bool(&self, name: &str) -> bool {
        match &self.target_fn {
            Some(targets) => targets.contains(name),
            None => true,
        }
    }

    fn get_line(&self, span: swc_common::Span) -> u64 {
        self.src_map
            .lookup_char_pos(span.lo)
            .line as u64
    }

    fn make_span(&self, span: swc_common::Span, confidence: Confidence) -> SourceSpan {
        SourceSpan {
            file: self.src.clone(),
            line: self.get_line(span),
            confidence,
        }
    }

    fn map_ts_type(&self, ts_type: &TsType) -> Result<IRType> {
        match ts_type {
            TsType::TsKeywordType(keyword) => self.map_keyword_type(keyword),

            TsType::TsTypeRef(type_ref) => self.map_type_ref(type_ref),

            TsType::TsArrayType(array) => {
                let inner = self.map_ts_type(&array.elem_type)?;
                Ok(IRType::List(Box::new(inner)))
            }

            TsType::TsTupleType(tuple) => {
                let types = tuple
                    .elem_types
                    .iter()
                    .map(|elem| self.map_ts_type(&elem.ty))
                    .collect::<Result<Vec<_>>>()?;
                Ok(IRType::Tuple(types))
            }

            TsType::TsUnionOrIntersectionType(union) => self.map_union_type(union),

            TsType::TsTypeLit(_) => Err(WdsmError::UnsupportedType {
                ty: "inline object type literal".to_string(),
                suggestion: Some("use a named interface or type alias instead".to_string()),
            }
            .into()),

            TsType::TsParenthesizedType(paren) => self.map_ts_type(&paren.type_ann),

            _ => Err(WdsmError::UnsupportedType {
                ty: format!("{:?}", ts_type),
                suggestion: Some("use a named type or supported primitive".to_string()),
            }
            .into()),
        }
    }

    fn map_keyword_type(&self, keyword: &TsKeywordType) -> Result<IRType> {
        match keyword.kind {
            TsKeywordTypeKind::TsStringKeyword => Ok(IRType::String),
            TsKeywordTypeKind::TsNumberKeyword => Ok(IRType::F64),
            TsKeywordTypeKind::TsBooleanKeyword => Ok(IRType::Bool),
            TsKeywordTypeKind::TsBigIntKeyword => Ok(IRType::S64),
            TsKeywordTypeKind::TsVoidKeyword |
            TsKeywordTypeKind::TsUndefinedKeyword |
            TsKeywordTypeKind::TsNullKeyword |
            TsKeywordTypeKind::TsNeverKeyword => Err(WdsmError::UnsupportedType {
                ty: format!("{:?}", keyword.kind),
                suggestion: Some("use string, number, boolean, or bigint".to_string()),
            }
            .into()),
            _ => Err(WdsmError::UnsupportedType {
                ty: format!("{:?}", keyword.kind),
                suggestion: Some("use string, number, boolean, or bigint".to_string()),
            }
            .into()),
        }
    }

    fn map_type_ref(&self, type_ref: &TsTypeRef) -> Result<IRType> {
        if let TsEntityName::Ident(ident) = &type_ref.type_name {
            let type_name = ident.sym.to_string();

            match type_name.as_str() {
                "Array" => {
                    if let Some(params) = &type_ref.type_params {
                        if let Some(first) = params.params.first() {
                            return Ok(IRType::List(Box::new(self.map_ts_type(first)?)));
                        }
                    }
                    Ok(IRType::List(Box::new(IRType::String)))
                }
                "Promise" => {
                    if let Some(params) = &type_ref.type_params {
                        if let Some(first) = params.params.first() {
                            return self.map_ts_type(first);
                        }
                    }
                    Ok(IRType::String)
                }
                "Map" | "Record" => {
                    if let Some(params) = &type_ref.type_params {
                        if params.params.len() == 2 {
                            let key = self.map_ts_type(&params.params[0])?;
                            let val = self.map_ts_type(&params.params[1])?;
                            return Ok(IRType::List(Box::new(IRType::Tuple(vec![key, val]))));
                        }
                    }
                    Ok(IRType::List(Box::new(IRType::Tuple(vec![
                        IRType::String,
                        IRType::String,
                    ]))))
                }
                "Set" => {
                    if let Some(params) = &type_ref.type_params {
                        if let Some(first) = params.params.first() {
                            return Ok(IRType::List(Box::new(self.map_ts_type(first)?)));
                        }
                    }
                    Ok(IRType::List(Box::new(IRType::String)))
                }
                _ => Ok(IRType::Named(type_name)),
            }
        } else {
            Err(WdsmError::UnsupportedType {
                ty: "qualified type name".to_string(),
                suggestion: Some("use a simple, non-qualified type name".to_string()),
            }
            .into())
        }
    }

    fn map_union_type(&self, union: &TsUnionOrIntersectionType) -> Result<IRType> {
        let types = match union {
            TsUnionOrIntersectionType::TsUnionType(u) => &u.types,
            TsUnionOrIntersectionType::TsIntersectionType(i) => &i.types,
        };

        let nullable_types: Vec<_> = types
            .iter()
            .filter(|t| {
                matches!(
                    &***t,
                    TsType::TsKeywordType(TsKeywordType {
                        kind: TsKeywordTypeKind::TsNullKeyword
                            | TsKeywordTypeKind::TsUndefinedKeyword,
                        ..
                    })
                )
            })
            .collect();

        if !nullable_types.is_empty() {
            let non_null: Vec<_> = types
                .iter()
                .filter(|t| {
                    !matches!(
                        &***t,
                        TsType::TsKeywordType(TsKeywordType {
                            kind: TsKeywordTypeKind::TsNullKeyword
                                | TsKeywordTypeKind::TsUndefinedKeyword,
                            ..
                        })
                    )
                })
                .collect();

            if non_null.len() == 1 {
                let inner = self.map_ts_type(&non_null[0])?;
                return Ok(IRType::Option(Box::new(inner)));
            }
        }

        let all_strings = types.iter().all(|t| {
            matches!(&**t, TsType::TsLitType(lit) if matches!(&lit.lit, TsLit::Str(_)))
        });

        if all_strings {
            return Err(WdsmError::UnsupportedType {
                ty: "inline union of string literals".to_string(),
                suggestion: Some("use a named type alias instead".to_string()),
            }
            .into());
        }

        Err(WdsmError::UnsupportedType {
            ty: "inline union type".to_string(),
            suggestion: Some("use a named type alias instead".to_string()),
        }
        .into())
    }

    fn interface_anal(&mut self, decl: &TsInterfaceDecl) {
        let name = decl.id.sym.to_string();
        let mut fields = Vec::new();

        for member in &decl.body.body {
            if let TsTypeElement::TsPropertySignature(prop) = member {
                if let Some(ident) = prop.key.as_ident() {
                    let field_name = ident.sym.to_string();
                    let optional = prop.optional;

                    let field_type = if let Some(type_ann) = &prop.type_ann {
                        match self.map_ts_type(&type_ann.type_ann) {
                            Ok(t) => t,
                            Err(e) => {
                                self.errors.push(e);
                                IRType::String
                            }
                        }
                    } else {
                        IRType::String
                    };

                    let final_type = if optional && !matches!(field_type, IRType::Option(_)) {
                        IRType::Option(Box::new(field_type))
                    } else {
                        field_type
                    };

                    fields.push(FieldDef {
                        wit_name: to_kebab_case(&field_name),
                        name: field_name,
                        ty: final_type,
                        optional,
                    });
                }
            }
        }

        let wit_name = to_kebab_case(&name);
        self.known_types.insert(name.clone());
        self.types.push(TypeDef::Record(RecordDef {
            wit_name,
            name,
            fields,
            source: self.make_span(decl.span(), Confidence::Explicit),
        }));
    }

    fn typea_anal(&mut self, decl: &TsTypeAliasDecl) {
        let name = decl.id.sym.to_string();

        match &*decl.type_ann {
            TsType::TsTypeLit(type_lit) => {
                let mut fields = Vec::new();

                for member in &type_lit.members {
                    if let TsTypeElement::TsPropertySignature(prop) = member {
                        if let Some(ident) = prop.key.as_ident() {
                            let field_name = ident.sym.to_string();
                            let optional = prop.optional;

                            let field_type = if let Some(type_ann) = &prop.type_ann {
                                match self.map_ts_type(&type_ann.type_ann) {
                                    Ok(t) => t,
                                    Err(e) => {
                                        self.errors.push(e);
                                        IRType::String
                                    }
                                }
                            } else {
                                IRType::String
                            };

                            let final_type =
                                if optional && !matches!(field_type, IRType::Option(_)) {
                                    IRType::Option(Box::new(field_type))
                                } else {
                                    field_type
                                };

                            fields.push(FieldDef {
                                wit_name: to_kebab_case(&field_name),
                                name: field_name,
                                ty: final_type,
                                optional,
                            });
                        }
                    }
                }

                let wit_name = to_kebab_case(&name);
                self.known_types.insert(name.clone());
                self.types.push(TypeDef::Record(RecordDef {
                    wit_name,
                    name,
                    fields,
                    source: self.make_span(decl.span(), Confidence::Explicit),
                }));
            }

            TsType::TsUnionOrIntersectionType(union) => {
                let types = match union {
                    TsUnionOrIntersectionType::TsUnionType(u) => &u.types,
                    TsUnionOrIntersectionType::TsIntersectionType(i) => &i.types,
                };

                let all_strings = types.iter().all(|t| {
                    matches!(&**t, TsType::TsLitType(lit) if matches!(&lit.lit, TsLit::Str(_)))
                });

                if all_strings {
                    let cases: Vec<String> = types
                        .iter()
                        .filter_map(|t| {
                            if let TsType::TsLitType(lit) = &**t {
                                if let TsLit::Str(s) = &lit.lit {
                                    if let Some(str_val) = s.value.as_str() {
                                        return Some(to_kebab_case(str_val));
                                    }
                                }
                            }
                            None
                        })
                        .collect();

                    let wit_name = to_kebab_case(&name);
                    self.known_types.insert(name.clone());
                    self.types.push(TypeDef::Enum(EnumDef {
                        wit_name,
                        name,
                        cases,
                        source: self.make_span(decl.span(), Confidence::Explicit),
                    }));
                } else {
                    let cases: Vec<VariantCase> = types
                        .iter()
                        .map(|t| match &**t {
                            TsType::TsLitType(lit) => {
                                let case_name = match &lit.lit {
                                    TsLit::Str(s) => {
                                        if let Some(str_val) = s.value.as_str() {
                                            to_kebab_case(str_val)
                                        } else {
                                            "unknown".to_string()
                                        }
                                    }
                                    TsLit::Number(n) => {
                                        format!("variant-{}", n.value as i64)
                                    }
                                    _ => "unknown".to_string(),
                                };
                                VariantCase {
                                    name: case_name,
                                    payload: None,
                                }
                            }
                            _ => {
                                let payload = self.map_ts_type(t).ok();
                                let case_name = match &payload {
                                    Some(IRType::Named(n)) => to_kebab_case(n),
                                    Some(other) => format!("{}-val", to_kebab_case(&other.to_wit_str())),
                                    None => match &**t {
                                        TsType::TsKeywordType(kw) => match kw.kind {
                                            TsKeywordTypeKind::TsNullKeyword => "null".to_string(),
                                            TsKeywordTypeKind::TsUndefinedKeyword => "undefined".to_string(),
                                            TsKeywordTypeKind::TsVoidKeyword => "void".to_string(),
                                            _ => "case".to_string(),
                                        },
                                        _ => "case".to_string(),
                                    },
                                };
                                VariantCase {
                                    name: case_name,
                                    payload,
                                }
                            }
                        })
                        .collect();

                    let wit_name = to_kebab_case(&name);
                    self.known_types.insert(name.clone());
                    self.types.push(TypeDef::Variant(VariantDef {
                        wit_name,
                        name,
                        cases,
                        source: self.make_span(decl.span(), Confidence::Explicit),
                    }));
                }
            }

            _ => {
                if let Ok(target) = self.map_ts_type(&decl.type_ann) {
                    let wit_name = to_kebab_case(&name);
                    self.known_types.insert(name.clone());
                    self.types.push(TypeDef::Alias(AliasDef {
                        wit_name,
                        name,
                        target,
                        source: self.make_span(decl.span(), Confidence::Explicit),
                    }));
                }
            }
        }
    }

    fn enum_anal(&mut self, decl: &TsEnumDecl) {
        let name = decl.id.sym.to_string();

        let cases: Vec<String> = decl
            .members
            .iter()
            .filter_map(|member| {
                if let TsEnumMemberId::Ident(ident) = &member.id {
                    Some(to_kebab_case(&ident.sym.to_string()))
                } else {
                    None
                }
            })
            .collect();

        let wit_name = to_kebab_case(&name);
        self.known_types.insert(name.clone());
        self.types.push(TypeDef::Enum(EnumDef {
            wit_name,
            name,
            cases,
            source: self.make_span(decl.span(), Confidence::Explicit),
        }));
    }

    fn fn_anal(&self, name: &str, func: &Function) -> Result<FunctionDef> {
        let mut params = Vec::new();

        for param in &func.params {
            if let Pat::Ident(ident) = &param.pat {
                let param_name = ident.sym.to_string();

                let param_type = if let Some(type_ann) = &ident.type_ann {
                    self.map_ts_type(&type_ann.type_ann)?
                } else {
                    return Err(WdsmError::MissingTypeAnnotation {
                        file: self.src.clone(),
                        line: self.get_line(param.span()),
                        function: name.to_string(),
                        param: param_name,
                    }
                    .into());
                };

                params.push(ParamDef {
                    wit_name: to_kebab_case(&param_name),
                    name: param_name,
                    ty: param_type,
                });
            } else {
                return Err(WdsmError::UnsupportedType {
                    ty: "destructured or complex parameter pattern".to_string(),
                    suggestion: Some("use a simple named parameter instead".to_string()),
                }
                .into());
            }
        }

        let returns = if let Some(ts_type) = &func.return_type {
            match &*ts_type.type_ann {
                TsType::TsKeywordType(kw)
                    if matches!(
                        kw.kind,
                        TsKeywordTypeKind::TsVoidKeyword
                            | TsKeywordTypeKind::TsNeverKeyword
                            | TsKeywordTypeKind::TsUndefinedKeyword
                    ) =>
                {
                    ReturnType::None
                }
                other => {
                    if let TsType::TsTypeRef(type_ref) = other {
                        if let TsEntityName::Ident(ident) = &type_ref.type_name {
                            if ident.sym.to_string() == "Promise" {
                                if let Some(type_params) = &type_ref.type_params {
                                    if let Some(first) = type_params.params.first() {
                                        if matches!(
                                            &**first,
                                            TsType::TsKeywordType(TsKeywordType {
                                                kind: TsKeywordTypeKind::TsVoidKeyword,
                                                ..
                                            })
                                        ) {
                                            return Ok(FunctionDef {
                                                wit_name: to_kebab_case(name),
                                                name: name.to_string(),
                                                params,
                                                returns: ReturnType::None,
                                                docs: None,
                                                source: self.make_span(
                                                    func.span(),
                                                    Confidence::Explicit,
                                                ),
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                    ReturnType::Type(self.map_ts_type(other)?)
                }
            }
        } else {
            ReturnType::None
        };

        Ok(FunctionDef {
            wit_name: to_kebab_case(name),
            name: name.to_string(),
            params,
            returns,
            docs: None,
            source: self.make_span(func.span(), Confidence::Explicit),
        })
    }

    fn arrowfn_anal(&self, name: &str, arrow: &ArrowExpr) -> Result<FunctionDef> {
        let mut params = Vec::new();

        for param in &arrow.params {
            if let Pat::Ident(ident) = param {
                let param_name = ident.sym.to_string();

                let param_type = if let Some(type_ann) = &ident.type_ann {
                    self.map_ts_type(&type_ann.type_ann)?
                } else {
                    return Err(WdsmError::MissingTypeAnnotation {
                        file: self.src.clone(),
                        line: self.get_line(arrow.span()),
                        function: name.to_string(),
                        param: param_name,
                    }
                    .into());
                };

                params.push(ParamDef {
                    wit_name: to_kebab_case(&param_name),
                    name: param_name,
                    ty: param_type,
                });
            } else {
                return Err(WdsmError::UnsupportedType {
                    ty: "destructured or complex parameter pattern".to_string(),
                    suggestion: Some("use a simple named parameter instead".to_string()),
                }
                .into());
            }
        }

        let returns = if let Some(ts_type) = &arrow.return_type {
            match &*ts_type.type_ann {
                TsType::TsKeywordType(kw)
                    if matches!(
                        kw.kind,
                        TsKeywordTypeKind::TsVoidKeyword
                            | TsKeywordTypeKind::TsNeverKeyword
                            | TsKeywordTypeKind::TsUndefinedKeyword
                    ) =>
                {
                    ReturnType::None
                }
                other => {
                    if let TsType::TsTypeRef(type_ref) = other {
                        if let TsEntityName::Ident(ident) = &type_ref.type_name {
                            if ident.sym.to_string() == "Promise" {
                                if let Some(type_params) = &type_ref.type_params {
                                    if let Some(first) = type_params.params.first() {
                                        if matches!(
                                            &**first,
                                            TsType::TsKeywordType(TsKeywordType {
                                                kind: TsKeywordTypeKind::TsVoidKeyword,
                                                ..
                                            })
                                        ) {
                                            return Ok(FunctionDef {
                                                wit_name: to_kebab_case(name),
                                                name: name.to_string(),
                                                params,
                                                returns: ReturnType::None,
                                                docs: None,
                                                source: self.make_span(
                                                    arrow.span(),
                                                    Confidence::Explicit,
                                                ),
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                    ReturnType::Type(self.map_ts_type(other)?)
                }
            }
        } else {
            ReturnType::None
        };

        Ok(FunctionDef {
            wit_name: to_kebab_case(name),
            name: name.to_string(),
            params,
            returns,
            docs: None,
            source: self.make_span(arrow.span(), Confidence::Explicit),
        })
    }
}

impl Visit for IRExtractor {
    fn visit_export_decl(&mut self, n: &ExportDecl) {
        match &n.decl {
            Decl::Fn(fn_decl) => {
                let fn_name = fn_decl.ident.sym.to_string();

                if self.extract_fn_bool(&fn_name) {
                    match self.fn_anal(&fn_name, &fn_decl.function) {
                        Ok(func_def) => self.functions.push(func_def),
                        Err(e) => self.errors.push(e),
                    }
                }
            }

            // export const foo = (...) => { ... }
            Decl::Var(var_decl) => {
                for decl in &var_decl.decls {
                    if let Pat::Ident(ident) = &decl.name {
                        let var_name = ident.sym.to_string();
                        if self.extract_fn_bool(&var_name) {
                            if let Some(init) = &decl.init {
                                if let Expr::Arrow(arrow) = &**init {
                                    match self.arrowfn_anal(&var_name, arrow) {
                                        Ok(func_def) => self.functions.push(func_def),
                                        Err(e) => self.errors.push(e),
                                    }
                                }
                            }
                        }
                    }
                }
            }

            Decl::TsInterface(interface_decl) => {
                self.interface_anal(interface_decl);
            }

            Decl::TsTypeAlias(type_alias) => {
                self.typea_anal(type_alias);
            }

            Decl::TsEnum(enum_decl) => {
                self.enum_anal(enum_decl);
            }

            _ => {}
        }

        n.visit_children_with(self);
    }

    fn visit_export_default_decl(&mut self, n: &ExportDefaultDecl) {
        if let DefaultDecl::Fn(fn_expr) = &n.decl {
            if let Some(ident) = &fn_expr.ident {
                let fn_name = ident.sym.to_string();
                if self.extract_fn_bool(&fn_name) {
                    match self.fn_anal(&fn_name, &fn_expr.function) {
                        Ok(func_def) => self.functions.push(func_def),
                        Err(e) => self.errors.push(e),
                    }
                }
            } else {
                if self.extract_fn_bool("default") {
                    match self.fn_anal("default", &fn_expr.function) {
                        Ok(func_def) => self.functions.push(func_def),
                        Err(e) => self.errors.push(e),
                    }
                }
            }
        }
        n.visit_children_with(self);
    }

    fn visit_ts_interface_decl(&mut self, n: &TsInterfaceDecl) {
        let name = n.id.sym.to_string();
        if !self.known_types.contains(&name) {
            self.interface_anal(n);
        }
        n.visit_children_with(self);
    }

    fn visit_ts_type_alias_decl(&mut self, n: &TsTypeAliasDecl) {
        let name = n.id.sym.to_string();
        if !self.known_types.contains(&name) {
            self.typea_anal(n);
        }
        n.visit_children_with(self);
    }

    fn visit_ts_enum_decl(&mut self, n: &TsEnumDecl) {
        let name = n.id.sym.to_string();
        if !self.known_types.contains(&name) {
            self.enum_anal(n);
        }
        n.visit_children_with(self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn extract_from_source(source: &str) -> Result<WitIR> {
        let mut tmp = NamedTempFile::new().unwrap();
        write!(tmp, "{}", source).unwrap();

        let config = FrontendConfig {
            package_name: "wdsm:test".to_string(),
            world_name: "test".to_string(),
            target_functions: None,
        };

        extract_ir(tmp.path(), &config)
    }

    #[test]
    fn test_simple_function() {
        let ir = extract_from_source(
            r#"export function hello(name: string): string {
                return `Hello, ${name}!`;
            }"#,
        )
        .unwrap();

        assert_eq!(ir.functions.len(), 1);
        assert_eq!(ir.functions[0].name, "hello");
        assert_eq!(ir.functions[0].wit_name, "hello");
        assert_eq!(ir.functions[0].params.len(), 1);
        assert_eq!(ir.functions[0].params[0].name, "name");
        assert_eq!(ir.functions[0].params[0].ty, IRType::String);
        assert_eq!(ir.functions[0].returns, ReturnType::Type(IRType::String));
    }

    #[test]
    fn test_void_return() {
        let ir = extract_from_source(
            r#"export function doStuff(x: string): void {
                console.log(x);
            }"#,
        )
        .unwrap();

        assert_eq!(ir.functions[0].returns, ReturnType::None);
    }

    #[test]
    fn test_record_from_interface() {
        let ir = extract_from_source(
            r#"
            export interface User {
                name: string;
                age: number;
            }
            export function getUser(id: string): User {
                return { name: "test", age: 42 };
            }
            "#,
        )
        .unwrap();

        assert_eq!(ir.types.len(), 1);
        if let TypeDef::Record(rec) = &ir.types[0] {
            assert_eq!(rec.name, "User");
            assert_eq!(rec.wit_name, "user");
            assert_eq!(rec.fields.len(), 2);
            assert_eq!(rec.fields[0].name, "name");
            assert_eq!(rec.fields[0].ty, IRType::String);
            assert_eq!(rec.fields[1].name, "age");
            assert_eq!(rec.fields[1].ty, IRType::F64);
        } else {
            panic!("Expected Record");
        }
    }

    #[test]
    fn test_list_type() {
        let ir = extract_from_source(
            r#"export function getNames(count: number): string[] {
                return [];
            }"#,
        )
        .unwrap();

        assert_eq!(
            ir.functions[0].returns,
            ReturnType::Type(IRType::List(Box::new(IRType::String)))
        );
    }

    #[test]
    fn test_promise_unwrap() {
        let ir = extract_from_source(
            r#"export function fetchData(url: string): Promise<string> {
                return Promise.resolve("");
            }"#,
        )
        .unwrap();
        assert_eq!(ir.functions[0].returns, ReturnType::Type(IRType::String));
    }

    #[test]
    fn test_missing_type_annotation_errors() {
        let result = extract_from_source(
            r#"export function hello(name) {
                return `Hello, ${name}!`;
            }"#,
        );

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("missing type annotation"));
    }
}
