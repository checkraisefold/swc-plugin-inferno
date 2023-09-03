#![allow(clippy::redundant_allocation)]

use std::{borrow::Cow, sync::Arc};

use serde::{Deserialize, Serialize};
use swc_atoms::{js_word, Atom, JsWord};
use swc_common::{
    comments::Comments,
    errors::HANDLER,
    iter::IdentifyLast,
    sync::Lrc,
    util::take::Take,
    FileName, Mark, SourceMap, Span, Spanned, DUMMY_SP,
};
use swc_config::merge::Merge;
use swc_ecma_ast::*;
use swc_ecma_parser::{parse_file_as_expr, Syntax};
use swc_ecma_utils::{drop_span, prepend_stmt, quote_ident, ExprFactory, StmtLike};
use swc_ecma_visit::{as_folder, noop_visit_mut_type, Fold, VisitMut, VisitMutWith};

use crate::{
    inferno_flags::{ChildFlags, VNodeFlags},
    refresh::options::{deserialize_refresh, RefreshOptions},
};

#[cfg(test)]
mod tests;

#[derive(Debug, Default, Clone, Serialize, Deserialize, Eq, PartialEq, Merge)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct Options {
    /// If this is `true`, swc will behave just like babel 8 with
    /// `BABEL_8_BREAKING: true`.
    #[serde(skip, default)]
    pub next: Option<bool>,

    #[serde(default)]
    pub import_source: Option<String>,

    #[serde(default)]
    pub development: Option<bool>,

    #[serde(default, deserialize_with = "deserialize_refresh")]
    // default to disabled since this is still considered as experimental by now
    pub refresh: Option<RefreshOptions>,
}

pub fn default_import_source() -> String {
    "inferno".into()
}

pub fn parse_expr_for_jsx(
    cm: &SourceMap,
    name: &str,
    src: String,
    top_level_mark: Mark,
) -> Arc<Box<Expr>> {
    let fm = cm.new_source_file(FileName::Custom(format!("<jsx-config-{}.js>", name)), src);

    parse_file_as_expr(
        &fm,
        Syntax::default(),
        Default::default(),
        None,
        &mut vec![],
    )
    .map_err(|e| {
        if HANDLER.is_set() {
            HANDLER.with(|h| {
                e.into_diagnostic(h)
                    .note("error detected while parsing option for classic jsx transform")
                    .emit()
            })
        }
    })
    .map(drop_span)
    .map(|mut expr| {
        apply_mark(&mut expr, top_level_mark);
        expr
    })
    .map(Arc::new)
    .unwrap_or_else(|()| {
        panic!(
            "failed to parse jsx option {}: '{}' is not an expression",
            name, fm.src,
        )
    })
}

fn apply_mark(e: &mut Expr, mark: Mark) {
    match e {
        Expr::Ident(i) => {
            i.span = i.span.apply_mark(mark);
        }
        Expr::Member(MemberExpr { obj, .. }) => {
            apply_mark(obj, mark);
        }
        _ => {}
    }
}

#[derive(PartialEq)]
pub enum VNodeType {
    Element = 0,
    Component = 1,
    Fragment = 2,
}

///
/// Turn JSX into Inferno function calls
///
///
/// `top_level_mark` should be [Mark] passed to
/// [swc_ecma_transforms_base::resolver::resolver_with_mark].
pub fn jsx<C>(
    cm: Lrc<SourceMap>,
    comments: Option<C>,
    options: Options,
    top_level_mark: Mark,
    unresolved_mark: Mark,
) -> impl Fold + VisitMut
where
    C: Comments,
{
    as_folder(Jsx {
        cm: cm.clone(),
        top_level_mark,
        unresolved_mark,
        import_source: options
            .import_source
            .unwrap_or_else(default_import_source)
            .into(),
        import_create_vnode: None,
        import_create_component: None,
        import_create_text_vnode: None,
        import_fragment: None,
        import_normalize_props: None,

        comments,
        top_level_node: true,
    })
}

struct Jsx<C>
where
    C: Comments,
{
    cm: Lrc<SourceMap>,

    top_level_mark: Mark,
    unresolved_mark: Mark,

    import_source: JsWord,

    import_create_vnode: Option<Ident>,
    import_create_component: Option<Ident>,
    import_create_text_vnode: Option<Ident>,
    import_fragment: Option<Ident>,
    import_normalize_props: Option<Ident>,
    top_level_node: bool,

    comments: Option<C>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct JsxDirectives {
    pub import_source: Option<JsWord>,
}

impl<C> Jsx<C>
where
    C: Comments,
{
    fn inject_runtime<T, F>(&mut self, body: &mut Vec<T>, inject: F)
    where
        T: StmtLike,
        F: Fn(Vec<Ident>, &str, &mut Vec<T>),
    {
        let mut imports = vec![];

        if let Some(_local) = self.import_create_vnode.take() {
            imports.push(quote_ident!("createVNode"))
        }
        if let Some(_local) = self.import_create_component.take() {
            imports.push(quote_ident!("createComponentVNode"))
        }
        if let Some(_local) = self.import_create_text_vnode.take() {
            imports.push(quote_ident!("createTextVNode"))
        }
        if let Some(_local) = self.import_normalize_props.take() {
            imports.push(quote_ident!("normalizeProps"))
        }
        if let Some(_local) = self.import_fragment.take() {
            imports.push(quote_ident!("createFragment"))
        }

        if !imports.is_empty() {
            inject(imports, &self.import_source, body);
        }
    }

    fn jsx_frag_to_expr(&mut self, el: JSXFragment) -> Expr {
        let span = el.span();

        if let Some(comments) = &self.comments {
            comments.add_pure_comment(span.lo);
        }

        let fragment = self
            .import_fragment
            .get_or_insert_with(|| quote_ident!("createFragment"))
            .clone();

        let mut children_requires_normalization: bool = false;
        let mut parent_can_be_keyed: bool = false;
        let mut children_count: u16 = 0;

        let mut children = vec![];
        for child in el.children {
            let child_expr = Some(match child {
                JSXElementChild::JSXText(text) => {
                    // TODO(kdy1): Optimize
                    let value = jsx_text_to_str(text.value);
                    let s = Str {
                        span: text.span,
                        raw: None,
                        value,
                    };

                    if s.value.is_empty() {
                        continue;
                    }

                    ExprOrSpread {
                        spread: None,
                        expr: Box::new(Expr::Call(CallExpr {
                            span: DUMMY_SP,
                            callee: self
                                .import_create_text_vnode
                                .get_or_insert_with(|| quote_ident!("createTextVNode"))
                                .clone()
                                .as_callee(),
                            args: vec![s.as_arg()],
                            type_args: Default::default(),
                        })),
                    }
                }
                JSXElementChild::JSXExprContainer(JSXExprContainer {
                    expr: JSXExpr::Expr(e),
                    ..
                }) => {
                    children_requires_normalization = true;
                    parent_can_be_keyed = false;
                    e.as_arg()
                }
                JSXElementChild::JSXExprContainer(JSXExprContainer {
                    expr: JSXExpr::JSXEmptyExpr(..),
                    ..
                }) => continue,
                JSXElementChild::JSXElement(el) => {
                    if !parent_can_be_keyed && !children_requires_normalization {
                        // Loop direct children to check if they have key property set
                        parent_can_be_keyed = Self::does_children_have_key_defined(&el);
                    }
                    self.jsx_elem_to_expr(*el).as_arg()
                }
                JSXElementChild::JSXFragment(el) => self.jsx_frag_to_expr(el).as_arg(),
                JSXElementChild::JSXSpreadChild(JSXSpreadChild { span, expr, .. }) => {
                    ExprOrSpread {
                        spread: Some(span),
                        expr,
                    }
                }
            });

            children_count += 1;

            children.push(child_expr)
        }

        let child_flags;

        if !children_requires_normalization {
            if children_count >= 1 {
                if parent_can_be_keyed {
                    child_flags = ChildFlags::HasKeyedChildren;
                } else {
                    child_flags = ChildFlags::HasNonKeyedChildren;
                }
            } else {
                child_flags = ChildFlags::HasInvalidChildren;
            }
        } else {
            child_flags = ChildFlags::UnknownChildren;
        };

        Expr::Call(CallExpr {
            span,
            callee: fragment.as_callee(),
            args: create_fragment_vnode_args(children, false, child_flags as u16, None, None),
            type_args: None,
        })
    }

    fn jsx_elem_to_expr(&mut self, el: JSXElement) -> Expr {
        let top_level_node = self.top_level_node;
        let span = el.span();
        self.top_level_node = false;

        let name_span: Span = el.opening.name.span();
        let name_expr;
        let mut mut_flags: u16;
        let vnode_kind: VNodeType;

        match el.opening.name {
            JSXElementName::Ident(ident) => {
                if ident.sym == js_word!("this") {
                    vnode_kind = VNodeType::Component;
                    mut_flags = VNodeFlags::ComponentUnknown as u16;
                    name_expr = Box::new(Expr::This(ThisExpr { span: name_span }));
                } else if is_component_vnode(&ident) {
                    match ident.sym {
                        js_word!("Fragment") => {
                            vnode_kind = VNodeType::Fragment;
                            mut_flags = VNodeFlags::ComponentUnknown as u16;
                            name_expr =
                                Box::new(Expr::Ident(Ident::new(js_word!("Fragment"), ident.span)));
                        }
                        _ => {
                            vnode_kind = VNodeType::Component;
                            mut_flags = VNodeFlags::ComponentUnknown as u16;
                            name_expr = Box::new(Expr::Ident(ident))
                        }
                    }
                } else {
                    vnode_kind = VNodeType::Element;
                    mut_flags = crate::vnode_types::parse_vnode_flag(&ident.sym);
                    name_expr = Box::new(Expr::Lit(Lit::Str(Str {
                        span: name_span,
                        raw: None,
                        value: ident.sym,
                    })))
                }
            }
            JSXElementName::JSXNamespacedName(_) => {
                HANDLER.with(|handler| {
                    handler
                        .struct_span_err(
                            name_span,
                            "JSX Namespace is disabled"
                        )
                        .emit()
                });

                return Expr::Invalid(Invalid { span: DUMMY_SP });
            }
            JSXElementName::JSXMemberExpr(JSXMemberExpr { obj, prop }) => {
                vnode_kind = VNodeType::Component;
                mut_flags = VNodeFlags::ComponentUnknown as u16;

                fn convert_obj(obj: JSXObject) -> Box<Expr> {
                    let span = obj.span();

                    (match obj {
                        JSXObject::Ident(i) => {
                            if i.sym == js_word!("this") {
                                Expr::This(ThisExpr { span })
                            } else {
                                Expr::Ident(i)
                            }
                        }
                        JSXObject::JSXMemberExpr(e) => Expr::Member(MemberExpr {
                            span,
                            obj: convert_obj(e.obj),
                            prop: MemberProp::Ident(e.prop),
                        }),
                    })
                    .into()
                }
                name_expr = Box::new(Expr::Member(MemberExpr {
                    span: name_span,
                    obj: convert_obj(obj),
                    prop: MemberProp::Ident(prop.clone()),
                }))
            }
        }

        if let Some(comments) = &self.comments {
            comments.add_pure_comment(span.lo);
        }

        let mut props_obj = ObjectLit {
            span: DUMMY_SP,
            props: vec![],
        };

        let mut key_prop = None;
        let mut ref_prop = None;
        let mut class_name_param: Option<Box<Expr>> = None;
        let mut has_text_children: bool = false;
        let mut has_keyed_children: bool = false;
        let mut has_non_keyed_children: bool = false;
        let mut children_known: bool = false;
        let mut needs_normalization: bool = false;
        let mut has_re_create_flag: bool = false;
        let mut child_flags_override_param = None;
        let mut flags_override_param = None;
        let mut content_editable_props: bool = false;

        for attr in el.opening.attrs {
            match attr {
                JSXAttrOrSpread::JSXAttr(attr) => {
                    //
                    match attr.name {
                        JSXAttrName::Ident(i) => {
                            //
                            match i.sym {
                                js_word!("class") | js_word!("className") => {
                                    if vnode_kind == VNodeType::Element {
                                        if let Some(v) = attr.value {
                                            class_name_param = jsx_attr_value_to_expr(v)
                                        }

                                        continue;
                                    }
                                }
                                js_word!("htmlFor") => {
                                    if vnode_kind == VNodeType::Element {
                                        props_obj.props.push(PropOrSpread::Prop(Box::new(
                                            Prop::KeyValue(KeyValueProp {
                                                key: PropName::Str(Str {
                                                    span: i.span,
                                                    raw: None,
                                                    value: js_word!("for"),
                                                }),
                                                value: match attr.value {
                                                    Some(v) => jsx_attr_value_to_expr(v)
                                                        .expect("empty expression?"),
                                                    None => Box::new(Expr::Lit(Lit::Null(Null {
                                                        span: DUMMY_SP,
                                                    }))),
                                                },
                                            }),
                                        )));
                                        continue;
                                    }
                                }
                                js_word!("onDoubleClick") => {
                                    props_obj.props.push(PropOrSpread::Prop(Box::new(
                                        Prop::KeyValue(KeyValueProp {
                                            key: PropName::Ident(Ident::new(
                                                js_word!("onDblClick"),
                                                span,
                                            )),
                                            value: match attr.value {
                                                Some(v) => jsx_attr_value_to_expr(v)
                                                    .expect("empty expression?"),
                                                None => true.into(),
                                            },
                                        }),
                                    )));
                                    continue;
                                }
                                js_word!("key") => {
                                    key_prop = attr
                                        .value
                                        .and_then(jsx_attr_value_to_expr)
                                        .map(|expr| expr.as_arg());

                                    if key_prop.is_none() {
                                        HANDLER.with(|handler| {
                                            handler
                                                .struct_span_err(
                                                    i.span,
                                                    "The value of property 'key' should not be \
                                                     empty",
                                                )
                                                .emit();
                                        });
                                    }
                                    continue;
                                }
                                js_word!("ref") => {
                                    ref_prop = attr
                                        .value
                                        .and_then(jsx_attr_value_to_expr)
                                        .map(|expr| expr.as_arg());

                                    if ref_prop.is_none() {
                                        HANDLER.with(|handler| {
                                            handler
                                                .struct_span_err(
                                                    i.span,
                                                    "The value of property 'ref' should not be \
                                                     empty",
                                                )
                                                .emit();
                                        });
                                    }
                                    continue;
                                }
                                js_word!("$ChildFlag") => {
                                    child_flags_override_param = attr
                                        .value
                                        .and_then(jsx_attr_value_to_expr)
                                        .map(|expr| expr.as_arg());

                                    if child_flags_override_param.is_none() {
                                        HANDLER.with(|handler| {
                                            handler
                                                .struct_span_err(
                                                    i.span,
                                                    "The value of property '$ChildFlag' should \
                                                     not be empty",
                                                )
                                                .emit();
                                        });
                                    }
                                    children_known = true;
                                    continue;
                                }
                                js_word!("$HasVNodeChildren") => {
                                    children_known = true;
                                    continue;
                                }
                                js_word!("$Flags") => {
                                    flags_override_param = attr
                                        .value
                                        .and_then(jsx_attr_value_to_expr)
                                        .map(|expr| expr.as_arg());

                                    if flags_override_param.is_none() {
                                        HANDLER.with(|handler| {
                                            handler
                                                .struct_span_err(
                                                    i.span,
                                                    "The value of property '$Flags' should not be \
                                                     empty",
                                                )
                                                .emit();
                                        });
                                    }
                                    continue;
                                }
                                js_word!("$HasTextChildren") => {
                                    children_known = true;
                                    has_text_children = true;
                                    continue;
                                }
                                js_word!("$HasNonKeyedChildren") => {
                                    children_known = true;
                                    has_non_keyed_children = true;
                                    continue;
                                }
                                js_word!("$HasKeyedChildren") => {
                                    children_known = true;
                                    has_keyed_children = true;
                                    continue;
                                }
                                js_word!("$ReCreate") => {
                                    has_re_create_flag = true;
                                    continue;
                                }
                                _ => {}
                            }

                            if i.sym.to_ascii_lowercase() == js_word!("contenteditable") {
                                content_editable_props = true;
                            } else if i.sym == js_word!("children") {
                                if attr.value.is_some() {
                                    HANDLER.with(|handler| {
                                        handler
                                            .struct_span_err(
                                                i.span,
                                                "'children' property is not supported in JSX. Use \
                                                 nesting instead.",
                                            )
                                            .emit();
                                    });
                                }

                                continue;
                            }

                            let value = match attr.value {
                                Some(v) => {
                                    jsx_attr_value_to_expr(v).expect("empty expression container?")
                                }
                                None => true.into(),
                            };

                            // TODO: Check if `i` is a valid identifier.

                            let converted_sym = crate::vnode_types::convert_svg_attrs(i);

                            props_obj
                                .props
                                .push(PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                                    key: converted_sym,
                                    value,
                                }))));
                        }
                        JSXAttrName::JSXNamespacedName(_) => {
                            HANDLER.with(|handler| {
                                handler
                                    .struct_span_err(
                                        span,
                                        "JSX Namespace is disabled"
                                    )
                                    .emit()
                            });
                        }
                    }
                }
                JSXAttrOrSpread::SpreadElement(attr) => match *attr.expr {
                    Expr::Object(obj) => {
                        needs_normalization = true;
                        props_obj.props.extend(obj.props);
                    }
                    _ => {
                        needs_normalization = true;
                        props_obj.props.push(PropOrSpread::Spread(attr));
                    }
                },
            }
        }

        let mut children_requires_normalization: bool = false;
        let mut children_found_text: bool = false;
        let mut parent_can_be_keyed: bool = false;
        let mut children_count: u16 = 0;

        let mut children = vec![];

        for child in el.children {
            let child_expr = Some(match child {
                JSXElementChild::JSXText(text) => {
                    // TODO(kdy1): Optimize
                    let value = jsx_text_to_str(text.value);
                    let s = Str {
                        span: text.span,
                        raw: None,
                        value,
                    };

                    if s.value.is_empty() {
                        continue;
                    }

                    if vnode_kind == VNodeType::Fragment {
                        ExprOrSpread {
                            spread: None,
                            expr: Box::new(Expr::Call(CallExpr {
                                span: DUMMY_SP,
                                callee: self
                                    .import_create_text_vnode
                                    .get_or_insert_with(|| quote_ident!("createTextVNode"))
                                    .clone()
                                    .as_callee(),
                                args: vec![s.as_arg()],
                                type_args: Default::default(),
                            })),
                        }
                    } else {
                        children_found_text = true;
                        Lit::Str(s).as_arg()
                    }
                }
                JSXElementChild::JSXExprContainer(JSXExprContainer {
                    expr: JSXExpr::Expr(e),
                    ..
                }) => {
                    children_requires_normalization = true;
                    parent_can_be_keyed = false;
                    e.as_arg()
                }
                JSXElementChild::JSXExprContainer(JSXExprContainer {
                    expr: JSXExpr::JSXEmptyExpr(..),
                    ..
                }) => continue,
                JSXElementChild::JSXElement(el) => {
                    if vnode_kind != VNodeType::Component
                        && !parent_can_be_keyed
                        && !children_known
                        && !children_requires_normalization
                    {
                        // Loop direct children to check if they have key property set
                        parent_can_be_keyed = Self::does_children_have_key_defined(&el);
                    }
                    self.jsx_elem_to_expr(*el).as_arg()
                }
                JSXElementChild::JSXFragment(el) => self.jsx_frag_to_expr(el).as_arg(),
                JSXElementChild::JSXSpreadChild(JSXSpreadChild { span, expr, .. }) => {
                    ExprOrSpread {
                        spread: Some(span),
                        expr,
                    }
                }
            });

            children_count += 1;

            children.push(child_expr)
        }

        if children_found_text {
            match children_count {
                1 => has_text_children = true,
                _ => {
                    for i in 0..children.len() {
                        let child = &children[i];

                        match child {
                            Some(v) => {
                                if let Expr::Lit(Lit::Str(text)) = &*v.expr {
                                    children[i] = Some(ExprOrSpread {
                                        spread: None,
                                        expr: Box::new(Expr::Call(CallExpr {
                                            span: DUMMY_SP,
                                            callee: self
                                                .import_create_text_vnode
                                                .get_or_insert_with(|| {
                                                    quote_ident!("createTextVNode")
                                                })
                                                .clone()
                                                .as_callee(),
                                            args: vec![text.clone().as_arg()],
                                            type_args: Default::default(),
                                        })),
                                    })
                                }
                            }
                            _ => continue,
                        }
                    }
                }
            }
        }

        parent_can_be_keyed =
            children_count > 1 && parent_can_be_keyed && !children_requires_normalization;
        let parent_can_be_non_keyed =
            children_count > 1 && !parent_can_be_keyed && !children_requires_normalization;

        let child_flags: ChildFlags;

        if !children_requires_normalization || children_known {
            if has_keyed_children || parent_can_be_keyed {
                child_flags = ChildFlags::HasKeyedChildren;
            } else if has_non_keyed_children || parent_can_be_non_keyed {
                child_flags = ChildFlags::HasNonKeyedChildren;
            } else if children_count == 1 {
                if has_text_children {
                    child_flags = ChildFlags::HasTextChildren;
                } else {
                    if vnode_kind == VNodeType::Fragment {
                        child_flags = ChildFlags::HasNonKeyedChildren;
                    } else {
                        child_flags = ChildFlags::HasVNodeChildren;
                    }
                }
            } else {
                child_flags = ChildFlags::HasInvalidChildren
            }
        } else {
            if has_keyed_children {
                child_flags = ChildFlags::HasKeyedChildren;
            } else if has_non_keyed_children {
                child_flags = ChildFlags::HasNonKeyedChildren;
            } else if has_text_children {
                child_flags = ChildFlags::HasTextChildren;
            } else {
                child_flags = ChildFlags::UnknownChildren;
            }
        }

        // TODO: Remove children from props?

        if vnode_kind == VNodeType::Component {
            match children.len() {
                0 => {}
                1 if children[0].as_ref().unwrap().spread.is_none() => {
                    props_obj
                        .props
                        .push(PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                            key: PropName::Ident(quote_ident!("children")),
                            value: children.take().into_iter().next().flatten().unwrap().expr,
                        }))));
                }
                _ => {
                    props_obj
                        .props
                        .push(PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                            key: PropName::Ident(quote_ident!("children")),
                            value: Box::new(Expr::Array(ArrayLit {
                                span: DUMMY_SP,
                                elems: children.take(),
                            })),
                        }))));
                }
            }
        }

        self.top_level_node = top_level_node;

        if has_re_create_flag {
            mut_flags = mut_flags | VNodeFlags::ReCreate as u16;
        }
        if content_editable_props {
            mut_flags = mut_flags | VNodeFlags::ContentEditable as u16;
        }

        let flags_expr = match flags_override_param {
            None => Box::new(Expr::Lit(Lit::Num(Number {
                span: DUMMY_SP,
                raw: None,
                value: mut_flags as f64,
            })))
            .as_arg(),
            Some(v) => v,
        };

        let create_method = if vnode_kind == VNodeType::Component {
            self.import_create_component
                .get_or_insert_with(|| quote_ident!("createComponentVNode"))
                .clone()
        } else if vnode_kind == VNodeType::Element {
            self.import_create_vnode
                .get_or_insert_with(|| quote_ident!("createVNode"))
                .clone()
        } else {
            self.import_fragment
                .get_or_insert_with(|| quote_ident!("createFragment"))
                .clone()
        };

        let create_method_args = if vnode_kind == VNodeType::Component {
            create_component_vnode_args(flags_expr, name_expr, props_obj, key_prop, ref_prop)
        } else if vnode_kind == VNodeType::Element {
            create_vnode_args(
                flags_expr,
                name_expr,
                class_name_param,
                children,
                child_flags as u16,
                child_flags_override_param,
                props_obj,
                key_prop,
                ref_prop,
            )
        } else {
            create_fragment_vnode_args(
                children,
                has_non_keyed_children
                    || has_keyed_children
                    || child_flags_override_param.is_some(),
                child_flags as u16,
                child_flags_override_param,
                key_prop,
            )
        };

        let create_expr = Expr::Call(CallExpr {
            span,
            callee: create_method.as_callee(),
            args: create_method_args,
            type_args: Default::default(),
        });

        if needs_normalization {
            return Expr::Call(CallExpr {
                span,
                callee: self
                    .import_normalize_props
                    .get_or_insert_with(|| quote_ident!("normalizeProps"))
                    .clone()
                    .as_callee(),
                args: vec![create_expr.as_arg()],
                type_args: Default::default(),
            });
        }

        return create_expr;
    }

    fn does_children_have_key_defined(el: &JSXElement) -> bool {
        for attr in &el.opening.attrs {
            match attr {
                JSXAttrOrSpread::JSXAttr(attr) => {
                    //
                    match &attr.name {
                        JSXAttrName::Ident(i) => {
                            if i.sym == js_word!("key") {
                                return true;
                            }
                        }
                        JSXAttrName::JSXNamespacedName(_) => {
                            continue;
                        }
                    }
                }
                JSXAttrOrSpread::SpreadElement(_attr) => {
                    continue;
                }
            }
        }

        return false;
    }
}

#[inline(always)]
fn create_vnode_args(
    flags: ExprOrSpread,
    name: Box<Expr>,
    class_name: Option<Box<Expr>>,
    mut children: Vec<Option<ExprOrSpread>>,
    child_flags: u16,
    child_flags_override_param: Option<ExprOrSpread>,
    props: ObjectLit,
    key: Option<ExprOrSpread>,
    refs: Option<ExprOrSpread>,
) -> Vec<ExprOrSpread> {
    let mut args: Vec<ExprOrSpread> = vec![flags, name.as_arg()];

    let has_children = !children.is_empty();
    let has_child_flags = child_flags_override_param.is_some()
        || child_flags != (ChildFlags::HasInvalidChildren as u16);
    let has_props = !props.props.is_empty();
    let has_key = key.is_some();
    let has_ref = refs.is_some();

    match class_name {
        None => {
            if has_children || has_child_flags || has_props || has_key || has_ref {
                args.push(Box::new(Expr::Lit(Lit::Null(Null { span: DUMMY_SP }))).as_arg());
            }
        }
        Some(some_class_name) => {
            args.push(some_class_name.as_arg());
        }
    }

    match children.len() {
        0 => {
            if has_child_flags || has_props || has_key || has_ref {
                args.push(Box::new(Expr::Lit(Lit::Null(Null { span: DUMMY_SP }))).as_arg());
            }
        }
        1 => args.push(
            children
                .take()
                .into_iter()
                .next()
                .flatten()
                .unwrap()
                .expr
                .as_arg(),
        ),
        _ => args.push(
            Box::new(Expr::Array(ArrayLit {
                span: DUMMY_SP,
                elems: children.take(),
            }))
            .as_arg(),
        ),
    }

    if has_child_flags {
        match child_flags_override_param {
            Some(some_child_flags_override_param) => {
                args.push(some_child_flags_override_param);
            }
            None => args.push(
                Box::new(Expr::Lit(Lit::Num(Number {
                    span: DUMMY_SP,
                    raw: None,
                    value: (child_flags) as f64,
                })))
                .as_arg(),
            ),
        }
    } else if has_props || has_key || has_ref {
        args.push(
            Box::new(Expr::Lit(Lit::Num(Number {
                span: DUMMY_SP,
                raw: None,
                value: (ChildFlags::HasInvalidChildren as u16) as f64,
            })))
            .as_arg(),
        );
    }

    if has_props {
        args.push(props.as_arg());
    } else if has_key || has_ref {
        args.push(Box::new(Expr::Lit(Lit::Null(Null { span: DUMMY_SP }))).as_arg());
    }

    match key {
        None => {
            if has_ref {
                args.push(Box::new(Expr::Lit(Lit::Null(Null { span: DUMMY_SP }))).as_arg());
            }
        }
        Some(some_key) => {
            args.push(some_key);
        }
    }

    match refs {
        None => {}
        Some(some_refs) => {
            args.push(some_refs);
        }
    }

    return args;
}

#[inline(always)]
fn create_component_vnode_args(
    flags: ExprOrSpread,
    name: Box<Expr>,
    props_literal: ObjectLit,
    key: Option<ExprOrSpread>,
    refs: Option<ExprOrSpread>,
) -> Vec<ExprOrSpread> {
    let mut args: Vec<ExprOrSpread> = vec![flags, name.as_arg()];

    if props_literal.props.is_empty() {
        if key.is_some() || refs.is_some() {
            args.push(Box::new(Expr::Lit(Lit::Null(Null { span: DUMMY_SP }))).as_arg());
        }
    } else {
        args.push(props_literal.as_arg());
    }

    match key {
        None => {
            if refs.is_some() {
                args.push(Box::new(Expr::Lit(Lit::Null(Null { span: DUMMY_SP }))).as_arg());
            }
        }
        Some(some_key) => {
            args.push(some_key);
        }
    }

    match refs {
        None => {}
        Some(some_ref) => {
            args.push(some_ref);
        }
    }

    return args;
}

#[inline(always)]
fn create_fragment_vnode_args(
    mut children: Vec<Option<ExprOrSpread>>,
    children_shape_is_user_defined: bool,
    child_flags: u16,
    child_flags_override_param: Option<ExprOrSpread>,
    key: Option<ExprOrSpread>,
) -> Vec<ExprOrSpread> {
    let mut args: Vec<ExprOrSpread> = vec![];
    let has_child_flags = child_flags_override_param.is_some()
        || child_flags != (ChildFlags::HasInvalidChildren as u16);
    let has_key = key.is_some();

    match children.len() {
        0 => {
            if has_child_flags || has_key {
                args.push(Box::new(Expr::Lit(Lit::Null(Null { span: DUMMY_SP }))).as_arg());
            }
        }
        1 => {
            if children_shape_is_user_defined || child_flags == ChildFlags::UnknownChildren as u16 {
                args.push(
                    children
                        .take()
                        .into_iter()
                        .next()
                        .flatten()
                        .unwrap()
                        .expr
                        .as_arg(),
                );
            } else {
                args.push(
                    Box::new(Expr::Array(ArrayLit {
                        span: DUMMY_SP,
                        elems: children.take(),
                    }))
                    .as_arg(),
                );
            }
        }
        _ => args.push(
            Box::new(Expr::Array(ArrayLit {
                span: DUMMY_SP,
                elems: children.take(),
            }))
            .as_arg(),
        ),
    }

    if has_child_flags {
        match child_flags_override_param {
            Some(some_child_flags_override_param) => {
                args.push(some_child_flags_override_param);
            }
            None => args.push(
                Box::new(Expr::Lit(Lit::Num(Number {
                    span: DUMMY_SP,
                    raw: None,
                    value: (child_flags) as f64,
                })))
                .as_arg(),
            ),
        }
    } else if has_key {
        args.push(
            Box::new(Expr::Lit(Lit::Num(Number {
                span: DUMMY_SP,
                raw: None,
                value: (ChildFlags::HasInvalidChildren as u16) as f64,
            })))
            .as_arg(),
        );
    }

    match key {
        None => {}
        Some(some_key) => {
            args.push(some_key);
        }
    }

    return args;
}

impl<C> VisitMut for Jsx<C>
where
    C: Comments,
{
    noop_visit_mut_type!();

    fn visit_mut_expr(&mut self, expr: &mut Expr) {
        let top_level_node = self.top_level_node;
        let mut did_work = false;

        if let Expr::JSXElement(el) = expr {
            did_work = true;
            // <div></div> => Inferno.createVNode(...);
            *expr = self.jsx_elem_to_expr(*el.take());
        } else if let Expr::JSXFragment(frag) = expr {
            // <></> => Inferno.createFragment(...);
            did_work = true;
            *expr = self.jsx_frag_to_expr(frag.take());
        } else if let Expr::Paren(ParenExpr {
            expr: inner_expr, ..
        }) = expr
        {
            if let Expr::JSXElement(el) = &mut **inner_expr {
                did_work = true;
                *expr = self.jsx_elem_to_expr(*el.take());
            } else if let Expr::JSXFragment(frag) = &mut **inner_expr {
                // <></> => Inferno.createFragment(...);
                did_work = true;
                *expr = self.jsx_frag_to_expr(frag.take());
            }
        }

        if did_work {
            self.top_level_node = false;
        }

        expr.visit_mut_children_with(self);

        self.top_level_node = top_level_node;
    }

    fn visit_mut_module(&mut self, module: &mut Module) {
        module.visit_mut_children_with(self);

        self.inject_runtime(&mut module.body, |imports, src, stmts| {
            let specifiers = imports
                .into_iter()
                .map(|imported| {
                    ImportSpecifier::Named(ImportNamedSpecifier {
                        span: DUMMY_SP,
                        local: imported,
                        imported: None,
                        is_type_only: false,
                    })
                })
                .collect();

            prepend_stmt(
                stmts,
                ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
                    span: DUMMY_SP,
                    specifiers,
                    src: Str {
                        span: DUMMY_SP,
                        raw: None,
                        value: src.into(),
                    }
                    .into(),
                    type_only: Default::default(),
                    with: Default::default(),
                })),
            )
        });
    }

    fn visit_mut_script(&mut self, script: &mut Script) {
        script.visit_mut_children_with(self);

        let mark = self.unresolved_mark;
        self.inject_runtime(&mut script.body, |imports, src, stmts| {
            prepend_stmt(stmts, add_require(imports, src, mark))
        });
    }
}

fn add_require(imports: Vec<Ident>, src: &str, unresolved_mark: Mark) -> Stmt {
    Stmt::Decl(Decl::Var(Box::new(VarDecl {
        span: DUMMY_SP,
        kind: VarDeclKind::Const,
        declare: false,
        decls: vec![VarDeclarator {
            span: DUMMY_SP,
            name: Pat::Object(ObjectPat {
                span: DUMMY_SP,
                props: imports
                    .into_iter()
                    .map(|imported| {
                        ObjectPatProp::Assign(AssignPatProp {
                            span: DUMMY_SP,
                            key: imported,
                            value: None,
                        })
                    })
                    .collect(),
                optional: false,
                type_ann: None,
            }),
            // require('inferno')
            init: Some(Box::new(Expr::Call(CallExpr {
                span: DUMMY_SP,
                callee: Callee::Expr(Box::new(Expr::Ident(Ident {
                    span: DUMMY_SP.apply_mark(unresolved_mark),
                    sym: js_word!("require"),
                    optional: false,
                }))),
                args: vec![ExprOrSpread {
                    spread: None,
                    expr: Box::new(Expr::Lit(Lit::Str(Str {
                        span: DUMMY_SP,
                        value: src.into(),
                        raw: None,
                    }))),
                }],
                type_args: None,
            }))),
            definite: false,
        }],
    })))
}

#[inline]
fn is_component_vnode(i: &Ident) -> bool {
    // If it starts with uppercase
    return i.as_ref().starts_with(|c: char| c.is_ascii_uppercase());
}

#[inline]
fn jsx_text_to_str(t: Atom) -> JsWord {
    let mut buf = String::new();
    let replaced = t.replace('\t', " ");

    for (is_last, (i, line)) in replaced.lines().enumerate().identify_last() {
        if line.is_empty() {
            continue;
        }
        let line = Cow::from(line);
        let line = if i != 0 {
            Cow::Borrowed(line.trim_start_matches(' '))
        } else {
            line
        };
        let line = if is_last {
            line
        } else {
            Cow::Borrowed(line.trim_end_matches(' '))
        };
        if line.len() == 0 {
            continue;
        }
        if i != 0 && !buf.is_empty() {
            buf.push(' ')
        }
        buf.push_str(&line);
    }
    buf.into()
}

fn jsx_attr_value_to_expr(v: JSXAttrValue) -> Option<Box<Expr>> {
    Some(match v {
        JSXAttrValue::Lit(Lit::Str(s)) => {
            let value = transform_jsx_attr_str(&s.value);

            Box::new(Expr::Lit(Lit::Str(Str {
                span: s.span,
                raw: None,
                value: value.into(),
            })))
        }
        JSXAttrValue::Lit(lit) => Box::new(lit.into()),
        JSXAttrValue::JSXExprContainer(e) => match e.expr {
            JSXExpr::JSXEmptyExpr(_) => None?,
            JSXExpr::Expr(e) => e,
        },
        JSXAttrValue::JSXElement(e) => Box::new(Expr::JSXElement(e)),
        JSXAttrValue::JSXFragment(f) => Box::new(Expr::JSXFragment(f)),
    })
}

fn transform_jsx_attr_str(v: &str) -> String {
    let single_quote = false;
    let mut buf = String::with_capacity(v.len());
    let mut iter = v.chars().peekable();

    while let Some(c) = iter.next() {
        match c {
            '\u{0008}' => buf.push_str("\\b"),
            '\u{000c}' => buf.push_str("\\f"),
            ' ' => buf.push(' '),

            '\n' | '\r' | '\t' => {
                buf.push(' ');

                while let Some(' ') = iter.peek() {
                    iter.next();
                }
            }
            '\u{000b}' => buf.push_str("\\v"),
            '\0' => buf.push_str("\\x00"),

            '\'' if single_quote => buf.push_str("\\'"),
            '"' if !single_quote => buf.push('\"'),

            '\x01'..='\x0f' | '\x10'..='\x1f' => {
                buf.push(c);
            }

            '\x20'..='\x7e' => {
                //
                buf.push(c);
            }
            '\u{7f}'..='\u{ff}' => {
                buf.push(c);
            }

            _ => {
                buf.push(c);
            }
        }
    }

    buf
}
