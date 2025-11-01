use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use swc_common::sync::Lrc;
use swc_common::{FileName, SourceMap};
use swc_ecma_ast::*;
use swc_ecma_parser::{Parser, StringInput, Syntax};
use swc_ecma_visit::{Visit, VisitWith};

#[derive(Debug, Clone)]
pub struct FnSign {
    pub params: Vec<FnParams>,
    pub return_type: String,
}

#[derive(Debug, Clone)]
pub struct FnParams {
    pub name: String,
    pub param_type: String,
}

struct FnWalker {
    target_function: String,
    signature: Option<FnSign>,
}

impl Visit for FnWalker {
    fn visit_export_decl(&mut self, n: &ExportDecl) {
        if let Decl::Fn(fn_decl) = &n.decl {
            if fn_decl.ident.sym.to_string() == self.target_function {
                self.signature = Some(get_fnsign(&fn_decl.function));
            }
        }
        n.visit_children_with(self);
    }

    fn visit_export_default_decl(&mut self, n: &ExportDefaultDecl) {
        if let DefaultDecl::Fn(fn_expr) = &n.decl {
            if let Some(ident) = &fn_expr.ident {
                if ident.sym.to_string() == self.target_function {
                    self.signature = Some(get_fnsign(&fn_expr.function));
                }
            }
        }
        n.visit_children_with(self);
    }

    fn visit_named_export(&mut self, n: &NamedExport) {
        n.visit_children_with(self);
    }
}

fn get_fnsign(func: &Function) -> FnSign {
    let mut params = Vec::new();

    for param in &func.params {
        if let Pat::Ident(ident) = &param.pat {
            let name = ident.sym.to_string();
            let param_type = get_type_annotation(&ident.type_ann);
            params.push(FnParams { name, param_type });
        }
    }

    let return_type = match &func.return_type {
        Some(ts_type) => get_return_type(ts_type),
        None => "string".to_string(),
    };

    FnSign {
        params,
        return_type,
    }
}

fn get_type_annotation(type_ann: &Option<Box<TsTypeAnn>>) -> String {
    match type_ann {
        Some(ann) => match &*ann.type_ann {
            TsType::TsKeywordType(keyword) => match keyword.kind {
                TsKeywordTypeKind::TsStringKeyword => "string".to_string(),
                TsKeywordTypeKind::TsNumberKeyword => "f64".to_string(),
                TsKeywordTypeKind::TsBooleanKeyword => "bool".to_string(),
                _ => "string".to_string(),
            },
            _ => "string".to_string(),
        },
        None => "string".to_string(),
    }
}

fn get_return_type(return_type: &TsTypeAnn) -> String {
    match &*return_type.type_ann {
        TsType::TsTypeRef(type_ref) => {
            if let TsEntityName::Ident(ident) = &type_ref.type_name {
                if ident.sym.to_string() == "Promise" {
                    // Extract Promise<T>
                    if let Some(type_params) = &type_ref.type_params {
                        if let Some(first) = type_params.params.first() {
                            return get_ts_type(first);
                        }
                    }
                }
            }
            "string".to_string()
        }
        TsType::TsKeywordType(keyword) => match keyword.kind {
            TsKeywordTypeKind::TsStringKeyword => "string".to_string(),
            TsKeywordTypeKind::TsNumberKeyword => "f64".to_string(),
            TsKeywordTypeKind::TsBooleanKeyword => "bool".to_string(),
            _ => "string".to_string(),
        },
        _ => "string".to_string(),
    }
}

fn get_ts_type(ts_type: &TsType) -> String {
    match ts_type {
        TsType::TsKeywordType(keyword) => match keyword.kind {
            TsKeywordTypeKind::TsStringKeyword => "string".to_string(),
            TsKeywordTypeKind::TsNumberKeyword => "f64".to_string(),
            TsKeywordTypeKind::TsBooleanKeyword => "bool".to_string(),
            _ => "string".to_string(),
        },
        _ => "string".to_string(),
    }
}

pub fn anal_fn(js_file: &Path, function_name: &str) -> Result<FnSign> {
    let source = fs::read_to_string(js_file).context("Failed to read JavaScript file")?;

    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(
        FileName::Custom(js_file.to_string_lossy().to_string()).into(),
        source,
    );

    let syntax = Syntax::Es(Default::default());

    let mut parser = Parser::new(syntax, StringInput::from(&*fm), None);

    let module = parser
        .parse_module()
        .map_err(|e| anyhow::anyhow!("Failed to parse JavaScript: {:?}", e))?;

    let mut visitor = FnWalker {
        target_function: function_name.to_string(),
        signature: None,
    };

    module.visit_with(&mut visitor);

    visitor.signature.context(format!(
        "Function '{}' not found in {}",
        function_name,
        js_file.display()
    ))
}