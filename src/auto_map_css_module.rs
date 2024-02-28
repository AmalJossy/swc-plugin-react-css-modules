use std::{collections::HashMap, path::PathBuf};

use path_absolutize::*;
use swc_core::{
    common::{Span, Spanned, DUMMY_SP},
    ecma::{
        ast::{
            BindingIdent, CallExpr, Callee, Decl, Expr, ExprOrSpread, Ident, ImportDecl,
            ImportDefaultSpecifier, ImportPhase, ImportSpecifier, JSXAttr, JSXAttrName,
            JSXAttrOrSpread, JSXAttrValue, JSXExpr, JSXExprContainer, JSXOpeningElement,
            KeyValueProp, Lit, Module, ModuleDecl, ModuleItem, ObjectLit, Pat, Prop, PropName,
            PropOrSpread, Stmt, Str, Tpl, TplElement, VarDecl, VarDeclKind, VarDeclarator,
        },
        atoms::JsWord,
        visit::{VisitMut, VisitMutWith},
    },
    plugin::errors::HANDLER,
};

use crate::{process_stylesheet::CssModuleParser, Config};

pub struct AutoMapCssModules {
    /// holds the directory of the file being processed
    dir: PathBuf,

    /// holds the virtual directory of the file being processed
    virtual_dir: PathBuf,

    /// project root
    context: PathBuf,

    /// holds the configuration for the plugin
    config: Config,

    /// holds the mapping of style names to generated class names
    style_maps_for_file: HashMap<JsWord, HashMap<String, String>>,

    /// flag to determine if the runtime helper should be injected
    is_runtime_helper_req: bool,
}

/// Returns the full path to the file's directory.
///
/// - swc/loader and swc/jest pass full `filepath`
/// - swc/cli pass relative `filepath`
fn get_dirs(mut context: PathBuf, filepath: PathBuf) -> (PathBuf, PathBuf) {
    // swc allows fs access only under /cwd alias
    // https://github.com/swc-project/swc/pull/4279
    // this check is to enure tests still work
    // TODO: figure out a better way to distinguish `cargo test`
    let mut virtual_dir = if std::fs::metadata("/cwd").is_ok() {
        PathBuf::from("/cwd")
    } else {
        context.clone()
    };

    let file_dir = filepath.parent().unwrap().to_path_buf();

    // If the filepath has a root, it's already the full path.
    if file_dir.has_root() {
        virtual_dir.push(file_dir.strip_prefix(context.clone()).unwrap());
        return (file_dir, virtual_dir);
    }

    context.push(file_dir.clone());
    virtual_dir.push(file_dir);
    (context, virtual_dir)
}

impl AutoMapCssModules {
    pub fn new(cwd: &str, filepath: &str, config: Config) -> Self {
        let context = PathBuf::from(if config.root.is_empty() {
            cwd.to_string()
        } else {
            config.root.clone()
        });

        let (dir, virtual_dir) = get_dirs(context.clone(), PathBuf::from(filepath));

        Self {
            dir,
            virtual_dir,
            context,
            config: config.clone(),
            style_maps_for_file: HashMap::new(),
            is_runtime_helper_req: false,
        }
    }

    fn add_import(&mut self, name: &JsWord, src: &JsWord) {
        let src_path = PathBuf::from(src.to_string());

        let file_path = src_path
            .absolutize_from(self.dir.clone())
            .unwrap()
            .to_path_buf();

        if !file_path.has_root() {
            panic!(
                "src_path: {}; file_path: {}",
                src_path.to_str().unwrap(),
                file_path.to_str().unwrap()
            )
        }

        let virtual_path = src_path
            .absolutize_from(self.virtual_dir.clone())
            .unwrap()
            .to_path_buf();

        let css_parser = CssModuleParser::new(
            self.config.generate_scoped_name.clone(),
            self.context.clone(),
            self.config.hash_prefix.clone(),
            virtual_path,
            file_path,
        );

        let style_name_map = css_parser.generate_style_name_map();
        match style_name_map {
            Ok(style_name_map) => {
                self.style_maps_for_file
                    .insert(name.clone(), style_name_map);
            }
            Err(err_str) => HANDLER.with(|handler| handler.struct_err(&err_str).emit()),
        }
    }

    fn get_generated_name(&self, style_name: &str, span: &Span) -> String {
        let mut style_name_parts: Vec<&str> = style_name.splitn(2, ".").collect();

        let generated_name_opt = match style_name_parts.len() {
            // without prefix, ie styleName="foo-bar"
            1 => {
                let no_prefix_name = match self.style_maps_for_file.get(&JsWord::from("")) {
                    Some(style_map) => style_map.get(&style_name.to_string()),
                    None => None,
                };
                if no_prefix_name.is_none() {
                    self.style_maps_for_file
                        .iter()
                        .find_map(|(_, v)| v.get(&style_name.to_string()))
                } else {
                    no_prefix_name
                }
            }
            // with prefix,ie styleName="styles.foo-bar"
            2 => {
                let module = style_name_parts.remove(0);
                let name = style_name_parts.remove(0);

                match self.style_maps_for_file.get(&JsWord::from(module)) {
                    Some(style_map) => style_map.get(name),
                    None => None,
                }
            }
            // more than 1 dot in name
            _ => None,
        };

        match generated_name_opt {
            Some(generated_name) => generated_name.to_string(),
            None => {
                // enable warning in build
                HANDLER.with(|handler| {
                    handler
                        .struct_span_warn(
                            *span,
                            &format!("Could not resolve styleName {}", style_name),
                        )
                        .emit();
                });
                String::default()
            }
        }
    }

    /// Returns the styleName object declaration \
    /// each key corresponds to a css import and the values is an object holdings all mapped class names
    fn get_stylename_map_decl(&self) -> ModuleItem {
        create_style_map_decl(&self.style_maps_for_file)
    }
}

fn extract_expr_from_jsx_expr(expr: JSXExpr) -> Box<Expr> {
    match expr {
        JSXExpr::Expr(expr) => expr,
        _ => Box::new(Expr::Lit(Lit::Str(Str {
            span: DUMMY_SP,
            value: "".into(),
            raw: None,
        }))),
    }
}

fn create_expr_expr_tpl(left_expr: &Box<Expr>, right_expr: &Box<Expr>) -> Box<Expr> {
    Box::new(Expr::Tpl(Tpl {
        span: DUMMY_SP,
        exprs: vec![left_expr.clone(), right_expr.clone()],
        quasis: vec![
            TplElement {
                raw: "".into(),
                cooked: Some("".into()),
                span: DUMMY_SP,
                tail: false,
            },
            TplElement {
                raw: " ".into(),
                cooked: Some(" ".into()),
                span: DUMMY_SP,
                tail: false,
            },
            TplElement {
                raw: "".into(),
                cooked: Some("".into()),
                span: DUMMY_SP,
                tail: true,
            },
        ],
    }))
}

fn create_lit_expr_tpl(left_expr: &str, right_expr: &Box<Expr>) -> Box<Expr> {
    let left_padded_expr = format!("{} ", left_expr);
    Box::new(Expr::Tpl(Tpl {
        span: DUMMY_SP,
        exprs: vec![right_expr.clone()],
        quasis: vec![
            TplElement {
                raw: left_padded_expr.clone().into(),
                cooked: Some(left_padded_expr.into()),
                span: DUMMY_SP,
                tail: false,
            },
            TplElement {
                raw: "".into(),
                cooked: Some("".into()),
                span: DUMMY_SP,
                tail: true,
            },
        ],
    }))
}

fn update_element_class_names(
    n: &mut JSXOpeningElement,
    class_names_opt: Option<JSXAttr>,
    generated_name: String,
) {
    if let Some(mut class_names) = class_names_opt {
        match class_names.value {
            // className="literal" styeName="literal"
            Some(JSXAttrValue::Lit(Lit::Str(str_lit_val))) => {
                let combined_class_names =
                    JsWord::from(format!("{} {}", str_lit_val.value, generated_name));
                // merge generated class names with existing class names
                class_names.value = Some(JSXAttrValue::Lit(Lit::Str(Str {
                    span: str_lit_val.span,
                    value: combined_class_names,
                    raw: None,
                })));
                n.attrs.push(JSXAttrOrSpread::JSXAttr(class_names));
            }
            // className={expression} styleName="literal"
            Some(JSXAttrValue::JSXExprContainer(JSXExprContainer { expr, .. })) => {
                class_names.value = Some(JSXAttrValue::JSXExprContainer(JSXExprContainer {
                    span: expr.span(),
                    expr: JSXExpr::Expr(create_lit_expr_tpl(
                        &generated_name.to_string(),
                        &extract_expr_from_jsx_expr(expr),
                    )),
                }));
                n.attrs.push(JSXAttrOrSpread::JSXAttr(class_names));
            }
            _ => (),
        }
    } else {
        // no existing class names
        n.attrs.push(JSXAttrOrSpread::JSXAttr(JSXAttr {
            span: DUMMY_SP,
            name: JSXAttrName::Ident(Ident {
                span: DUMMY_SP,
                sym: "className".into(),
                optional: false,
            }),
            value: Some(JSXAttrValue::Lit(Lit::Str(Str {
                span: DUMMY_SP,
                value: generated_name.into(),
                raw: None,
            }))),
        }));
    }
}

fn update_element_class_names_with_expr(
    n: &mut JSXOpeningElement,
    class_names_opt: Option<JSXAttr>,
    style_name_expr: &Box<Expr>,
) {
    // getClassName(<style_name_expr>, _styleNameObjMap)
    let runtime_expr = Box::new(Expr::Call(CallExpr {
        span: DUMMY_SP,
        callee: Callee::Expr(Box::new(Expr::Ident(Ident {
            span: DUMMY_SP,
            sym: "_getClassNames$0".into(),
            optional: false,
        }))),
        args: vec![
            ExprOrSpread {
                spread: None,
                expr: style_name_expr.clone(),
            },
            ExprOrSpread {
                spread: None,
                expr: Box::new(Expr::Ident(Ident {
                    span: DUMMY_SP,
                    sym: "_styleNameObjMap$0".into(),
                    optional: false,
                })),
            },
        ],
        type_args: None,
    }));

    match class_names_opt {
        // element has className attribute
        Some(mut class_names) => {
            let class_name_expr = match class_names.value {
                // className={expresssion}
                Some(JSXAttrValue::JSXExprContainer(JSXExprContainer { expr, .. })) => {
                    create_expr_expr_tpl(&extract_expr_from_jsx_expr(expr), &runtime_expr)
                }
                // className="literal"
                Some(JSXAttrValue::Lit(Lit::Str(str_lit_val))) => {
                    create_lit_expr_tpl(&str_lit_val.value.to_string(), &runtime_expr)
                }
                _ => "".into(),
            };
            // className={`${<class_name_expr>} ${getClassName(<style_name_expr>, _styleNameObjMap)}``}
            class_names.value = Some(JSXAttrValue::JSXExprContainer(JSXExprContainer {
                span: DUMMY_SP,
                expr: JSXExpr::Expr(class_name_expr),
            }));
            n.attrs.push(JSXAttrOrSpread::JSXAttr(class_names));
        }
        // no existing class names
        None => {
            n.attrs.push(JSXAttrOrSpread::JSXAttr(JSXAttr {
                span: DUMMY_SP,
                name: JSXAttrName::Ident(Ident {
                    span: DUMMY_SP,
                    sym: "className".into(),
                    optional: false,
                }),
                value: Some(JSXAttrValue::JSXExprContainer(JSXExprContainer {
                    span: DUMMY_SP,
                    expr: JSXExpr::Expr(runtime_expr),
                })),
            }));
        }
    }
}

fn create_helper_import_decl() -> ModuleItem {
    // import { getClassName } from "swc-plugin-react-css-modules/dist/browser/getClassName";
    ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
        span: DUMMY_SP,
        specifiers: vec![ImportSpecifier::Default(ImportDefaultSpecifier {
            span: DUMMY_SP,
            local: Ident {
                span: DUMMY_SP,
                sym: "_getClassNames$0".into(),
                optional: false,
            },
        })],
        src: Box::new("swc-plugin-react-css-modules/dist/browser/getClassName".into()),
        type_only: false,
        with: None,
        phase: ImportPhase::Evaluation,
    }))
}

fn create_style_map_decl(style_name_map: &HashMap<JsWord, HashMap<String, String>>) -> ModuleItem {
    let mut props = Vec::new();
    for (import, style_name_map) in style_name_map.iter() {
        let mut nested_props = Vec::new();
        for (key, value) in style_name_map.iter() {
            nested_props.push(PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                key: PropName::Str(key.clone().into()),
                value: value.clone().into(),
            }))));
        }
        let style_map_expr = Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: nested_props,
        });
        props.push(PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
            key: PropName::Str(import.clone().into()),
            value: style_map_expr.into(),
        }))));
    }

    // const _styleNameObjMap = <style_map_expr>;
    ModuleItem::Stmt(Stmt::Decl(Decl::Var(Box::new(VarDecl {
        span: DUMMY_SP,
        kind: VarDeclKind::Const,
        declare: false,
        decls: vec![VarDeclarator {
            span: DUMMY_SP,
            name: Pat::Ident(BindingIdent {
                id: Ident {
                    span: DUMMY_SP,
                    sym: "_styleNameObjMap$0".into(),
                    optional: false,
                },
                type_ann: None,
            }),
            definite: false,
            init: Some(Box::new(Expr::Object(ObjectLit {
                span: DUMMY_SP,
                props,
            }))),
        }],
    }))))
}

impl VisitMut for AutoMapCssModules {
    // Implement necessary visit_mut_* methods for actual custom transform.
    // A comprehensive list of possible visitor methods can be found here:
    // https://rustdoc.swc.rs/swc_ecma_visit/trait.VisitMut.html

    fn visit_mut_jsx_opening_element(&mut self, n: &mut JSXOpeningElement) {
        let mut class_names: Option<JSXAttr> = None;
        let mut style_names: Option<JSXAttr> = None;

        for attr in n.attrs.iter() {
            match attr {
                JSXAttrOrSpread::JSXAttr(jsx_attr) => {
                    if let JSXAttrName::Ident(Ident { sym, .. }) = &jsx_attr.name {
                        if sym == "styleName" {
                            style_names = Some(jsx_attr.clone());
                        } else if sym == "className" {
                            class_names = Some(jsx_attr.clone());
                        }
                    }
                }
                JSXAttrOrSpread::SpreadElement(_) => {}
            }
        }

        if style_names.is_none() {
            return;
        }

        // delete className and styleName attributes, as they will be replaced
        n.attrs.retain(|attr| match attr {
            JSXAttrOrSpread::JSXAttr(jsx_attr) => {
                if let JSXAttrName::Ident(Ident { sym, .. }) = &jsx_attr.name {
                    sym != "className" && sym != "styleName"
                } else {
                    true
                }
            }
            JSXAttrOrSpread::SpreadElement(_) => true,
        });

        if let Some(style_names) = style_names {
            match &style_names.value {
                // styleName="style1 foo.style2"
                Some(JSXAttrValue::Lit(Lit::Str(str_lit_val))) => {
                    let mut generated_names = Vec::new();
                    for style_name in str_lit_val.value.split_whitespace() {
                        generated_names.push(
                            self.get_generated_name(style_name, &style_names.span)
                                .to_string(),
                        );
                    }
                    update_element_class_names(n, class_names, generated_names.join(" "));
                }
                // styleName={style3}
                Some(JSXAttrValue::JSXExprContainer(JSXExprContainer { expr, .. })) => match expr {
                    JSXExpr::Expr(expr) => {
                        self.is_runtime_helper_req = true;
                        update_element_class_names_with_expr(n, class_names, expr);
                    }
                    _ => (),
                },
                _ => (),
            }
        }

        n.visit_mut_children_with(self);
    }

    fn visit_mut_import_decl(&mut self, n: &mut ImportDecl) {
        n.visit_mut_children_with(self);
        if !n
            .src
            .value
            .ends_with(self.config.css_modules_suffix.as_str())
        {
            return;
        }

        let src = &n.src.value;

        if n.specifiers.is_empty() {
            self.add_import(&JsWord::from(""), src);
            return;
        }

        for specifier in n.specifiers.iter() {
            match specifier {
                ImportSpecifier::Default(default) => {
                    self.add_import(&default.local.sym, src);
                }

                ImportSpecifier::Namespace(namespace) => self.add_import(&namespace.local.sym, src),

                ImportSpecifier::Named(_) => HANDLER.with(|handler| {
                    handler
                        .struct_span_err(n.span, "Named imports are not supported")
                        .emit();
                }),
            }
        }
    }

    fn visit_mut_module(&mut self, n: &mut Module) {
        n.visit_mut_children_with(self);
        if !self.is_runtime_helper_req {
            return;
        }
        let mut pos: i32 = -1;
        for (i, module_item) in n.body.iter().enumerate() {
            match module_item {
                ModuleItem::ModuleDecl(ModuleDecl::Import(_)) => (),
                _ => {
                    if pos == -1 {
                        pos = i as i32;
                        break;
                    }
                }
            }
        }
        if pos == -1 {
            return;
        }
        let pos_usize = pos as usize;
        n.body.insert(pos_usize, create_helper_import_decl());
        n.body.insert(pos_usize + 1, self.get_stylename_map_decl());
    }
}
