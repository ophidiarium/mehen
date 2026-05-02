use serde::Serialize;
use serde::ser::{SerializeStruct, Serializer};
use std::fmt;

use crate::checker::Checker;
use crate::langs::{LANG, *};
use crate::languages::{Kotlin, Python, Ruby, Rust, Tsx, Typescript};
use crate::node::Node;
use crate::spaces::SpaceKind;

/// Classifies a function space as a method of a class or interface when its
/// `Npm::compute` pass detected it as such. Propagated up during `merge` so
/// the enclosing class/interface space increments its counters exactly once.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum MethodRole {
    #[default]
    None,
    ClassMethod {
        public: bool,
    },
    InterfaceMethod {
        public: bool,
    },
}

/// The `Npm` metric.
///
/// This metric counts the number of public methods
/// of classes/interfaces.
#[derive(Clone, Debug, Default)]
pub(crate) struct Stats {
    class_npm: usize,
    interface_npm: usize,
    class_nm: usize,
    interface_nm: usize,
    class_npm_sum: usize,
    interface_npm_sum: usize,
    class_nm_sum: usize,
    interface_nm_sum: usize,
    space_kind: SpaceKind,
    not_applicable: bool,
    /// Classification of *this* function-space, set when `Npm::compute`
    /// recognises the node that opens the space as a method.
    method_role: MethodRole,
    /// True once any class-like or interface-like space has been observed in
    /// the subtree being aggregated. Kept separate from the numeric sums so
    /// an empty class / interface (no methods) still keeps unit-level `npm`
    /// visible.
    has_class_like: bool,
}

impl Serialize for Stats {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut st = serializer.serialize_struct("npm", 9)?;
        st.serialize_field("classes", &self.class_npm_sum())?;
        st.serialize_field("interfaces", &self.interface_npm_sum())?;
        st.serialize_field("class_methods", &self.class_nm_sum())?;
        st.serialize_field("interface_methods", &self.interface_nm_sum())?;
        st.serialize_field("classes_average", &self.class_coa())?;
        st.serialize_field("interfaces_average", &self.interface_coa())?;
        st.serialize_field("total", &self.total_npm())?;
        st.serialize_field("total_methods", &self.total_nm())?;
        st.serialize_field("average", &self.total_coa())?;
        st.end()
    }
}

impl fmt::Display for Stats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "classes: {}, interfaces: {}, class_methods: {}, interface_methods: {}, classes_average: {}, interfaces_average: {}, total: {}, total_methods: {}, average: {}",
            self.class_npm_sum(),
            self.interface_npm_sum(),
            self.class_nm_sum(),
            self.interface_nm_sum(),
            self.class_coa(),
            self.interface_coa(),
            self.total_npm(),
            self.total_nm(),
            self.total_coa()
        )
    }
}

impl Stats {
    /// Merges a second `Npm` metric into the first one
    pub(crate) fn merge(&mut self, other: &Self) {
        use SpaceKind::*;

        // If the child space was classified as a method, bump the enclosing
        // class/interface counters by one. We prefer the child's explicit
        // role over a kind-based heuristic so Rust `impl` / `trait` routing
        // is correct too.
        if matches!(self.space_kind, Class | Impl | Interface | Trait | Unit) {
            match other.method_role {
                MethodRole::ClassMethod { public } => {
                    self.class_nm += 1;
                    if public {
                        self.class_npm += 1;
                    }
                }
                MethodRole::InterfaceMethod { public } => {
                    self.interface_nm += 1;
                    if public {
                        self.interface_npm += 1;
                    }
                }
                MethodRole::None => {}
            }
        }

        self.class_npm_sum += other.class_npm_sum;
        self.interface_npm_sum += other.interface_npm_sum;
        self.class_nm_sum += other.class_nm_sum;
        self.interface_nm_sum += other.interface_nm_sum;
        self.not_applicable |= other.not_applicable;
        self.has_class_like |= other.has_class_like;
    }

    /// Returns the number of class public methods sum in a space.
    #[inline(always)]
    pub(crate) fn class_npm_sum(&self) -> f64 {
        self.class_npm_sum as f64
    }

    /// Returns the number of interface public methods sum in a space.
    #[inline(always)]
    pub(crate) fn interface_npm_sum(&self) -> f64 {
        self.interface_npm_sum as f64
    }

    /// Returns the number of class methods sum in a space.
    #[inline(always)]
    pub(crate) fn class_nm_sum(&self) -> f64 {
        self.class_nm_sum as f64
    }

    /// Returns the number of interface methods sum in a space.
    #[inline(always)]
    pub(crate) fn interface_nm_sum(&self) -> f64 {
        self.interface_nm_sum as f64
    }

    /// Returns the class `Coa` metric value
    ///
    /// The `Class Operation Accessibility` metric value for a class
    /// is computed by dividing the `Npm` value of the class
    /// by the total number of methods defined in the class.
    ///
    /// This metric is an adaptation of the `Classified Operation Accessibility` (`COA`)
    /// security metric for not classified methods.
    /// Paper: <https://ieeexplore.ieee.org/abstract/document/5381538>
    #[inline(always)]
    pub(crate) fn class_coa(&self) -> f64 {
        self.class_npm_sum() / self.class_nm_sum()
    }

    /// Returns the interface `Coa` metric value
    ///
    /// The `Class Operation Accessibility` metric value for an interface
    /// is computed by dividing the `Npm` value of the interface
    /// by the total number of methods defined in the interface.
    ///
    /// This metric is an adaptation of the `Classified Operation Accessibility` (`COA`)
    /// security metric for not classified methods.
    /// Paper: <https://ieeexplore.ieee.org/abstract/document/5381538>
    #[inline(always)]
    pub(crate) fn interface_coa(&self) -> f64 {
        // For the Java language it's not necessary to compute the metric value
        // The metric value in Java can only be 1.0 or f64:NAN
        if self.interface_npm_sum == self.interface_nm_sum && self.interface_npm_sum != 0 {
            1.0
        } else {
            self.interface_npm_sum() / self.interface_nm_sum()
        }
    }

    /// Returns the total `Coa` metric value
    ///
    /// The total `Class Operation Accessibility` metric value
    /// is computed by dividing the total `Npm` value
    /// by the total number of methods.
    ///
    /// This metric is an adaptation of the `Classified Operation Accessibility` (`COA`)
    /// security metric for not classified methods.
    /// Paper: <https://ieeexplore.ieee.org/abstract/document/5381538>
    #[inline(always)]
    pub(crate) fn total_coa(&self) -> f64 {
        self.total_npm() / self.total_nm()
    }

    /// Returns the total number of public methods in a space.
    #[inline(always)]
    pub(crate) fn total_npm(&self) -> f64 {
        self.class_npm_sum() + self.interface_npm_sum()
    }

    /// Returns the total number of methods in a space.
    #[inline(always)]
    pub(crate) fn total_nm(&self) -> f64 {
        self.class_nm_sum() + self.interface_nm_sum()
    }

    // Accumulates the number of class and interface
    // public and not public methods into the sums
    #[inline(always)]
    pub(crate) fn compute_sum(&mut self) {
        self.class_npm_sum += self.class_npm;
        self.interface_npm_sum += self.interface_npm;
        self.class_nm_sum += self.class_nm;
        self.interface_nm_sum += self.interface_nm;
    }

    // Checks if the `Npm` metric is disabled
    #[inline(always)]
    pub(crate) fn is_disabled(&self) -> bool {
        if self.not_applicable {
            return true;
        }
        match self.space_kind {
            SpaceKind::Function | SpaceKind::Unknown => true,
            // Use the presence flag — an empty class / interface (no methods)
            // is still class-like and should keep `npm` visible at the unit.
            SpaceKind::Unit => !self.has_class_like,
            _ => false,
        }
    }

    /// Marks this metric as not applicable to the current language so it is
    /// omitted from output rather than serialized as a measured zero.
    #[inline(always)]
    pub(crate) fn mark_not_applicable(&mut self) {
        self.not_applicable = true;
    }

    /// Returns whether the `Npm` metric is meaningful for the given language.
    /// Languages without class-like constructs opt out.
    #[inline(always)]
    pub(crate) fn applies_to(lang: LANG) -> bool {
        !matches!(lang, LANG::Go)
    }

    /// Records the kind of the enclosing space. Also flags the stats as
    /// having observed a class-like space if applicable, so unit-level
    /// aggregation can distinguish "no classes" from "classes with no
    /// counted methods".
    #[inline(always)]
    pub(crate) fn set_space_kind(&mut self, kind: SpaceKind) {
        self.space_kind = kind;
        if matches!(
            kind,
            SpaceKind::Class | SpaceKind::Interface | SpaceKind::Impl | SpaceKind::Trait
        ) {
            self.has_class_like = true;
        }
    }
}

pub(crate) trait Npm
where
    Self: Checker,
{
    fn compute(node: &Node, code: &[u8], stats: &mut Stats);
}

/// Flags the method's own function-space with its role. `merge` later uses
/// this flag to increment the enclosing class or interface counters by one,
/// which avoids the double-counting that happens when counters are carried
/// on both the method's and its parent's own `class_nm`/`class_nm_sum`.
///
/// `container` is the `SpaceKind` of the class-like or interface-like space
/// that owns this method (derived from the AST parent chain, not
/// `stats.space_kind`, since the method's stats are created with
/// `space_kind = Function`).
#[inline(always)]
fn record_method(stats: &mut Stats, container: SpaceKind, is_public: bool) {
    stats.method_role = match container {
        SpaceKind::Class | SpaceKind::Impl => MethodRole::ClassMethod { public: is_public },
        SpaceKind::Interface | SpaceKind::Trait => {
            MethodRole::InterfaceMethod { public: is_public }
        }
        _ => return,
    };
}

/// Returns whether the given Python method is considered public. Python uses a
/// leading-underscore convention: names starting with `_` are non-public
/// (double-underscore `__name` is name-mangled and also private). Dunder
/// methods like `__init__` are conventionally public.
fn python_method_is_public(name: &str) -> bool {
    if name.starts_with("__") && name.ends_with("__") {
        return true;
    }
    !name.starts_with('_')
}

impl Npm for PythonCode {
    fn compute(node: &Node, code: &[u8], stats: &mut Stats) {
        if node.kind_id() != Python::FunctionDefinition {
            return;
        }
        // The function must be a direct member of a class body:
        //   class_definition -> block -> function_definition
        let inside_class = node
            .parent()
            .and_then(|p| p.parent())
            .is_some_and(|grand| grand.kind_id() == Python::ClassDefinition);
        if !inside_class {
            return;
        }
        let is_public = node
            .child_by_field_name("name")
            .and_then(|name| std::str::from_utf8(&code[name.start_byte()..name.end_byte()]).ok())
            .map(python_method_is_public)
            .unwrap_or(true);
        record_method(stats, SpaceKind::Class, is_public);
    }
}

impl Npm for TypescriptCode {
    fn compute(node: &Node, code: &[u8], stats: &mut Stats) {
        let kind_id = node.kind_id();
        let container = if kind_id == Typescript::MethodDefinition {
            SpaceKind::Class
        } else if kind_id == Typescript::MethodSignature {
            SpaceKind::Interface
        } else {
            return;
        };
        let is_public = ts_method_is_public(node, code, |id| match id.into() {
            Typescript::AccessibilityModifier => TsAccessKind::Modifier,
            Typescript::PrivatePropertyIdentifier => TsAccessKind::PrivateName,
            _ => TsAccessKind::Other,
        });
        record_method(stats, container, is_public);
    }
}

impl Npm for TsxCode {
    fn compute(node: &Node, code: &[u8], stats: &mut Stats) {
        let kind_id = node.kind_id();
        let container = if kind_id == Tsx::MethodDefinition {
            SpaceKind::Class
        } else if kind_id == Tsx::MethodSignature {
            SpaceKind::Interface
        } else {
            return;
        };
        let is_public = ts_method_is_public(node, code, |id| match id.into() {
            Tsx::AccessibilityModifier => TsAccessKind::Modifier,
            Tsx::PrivatePropertyIdentifier => TsAccessKind::PrivateName,
            _ => TsAccessKind::Other,
        });
        record_method(stats, container, is_public);
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TsAccessKind {
    Modifier,
    /// A `#name` method identifier (ECMAScript private class methods).
    PrivateName,
    Other,
}

/// Walks a TS / TSX method definition's children to decide if the method is
/// public. A method is non-public when either:
///
///   - its identifier is a `private_property_identifier` (`#name`), or
///   - an `accessibility_modifier` spells `private` / `protected`.
///
/// Otherwise the method is public (the default / explicit `public`).
fn ts_method_is_public(node: &Node, code: &[u8], classify: impl Fn(u16) -> TsAccessKind) -> bool {
    for child in node.children() {
        match classify(child.kind_id()) {
            TsAccessKind::Modifier => {
                let text = &code[child.start_byte()..child.end_byte()];
                if text == b"private" || text == b"protected" {
                    return false;
                }
            }
            TsAccessKind::PrivateName => return false,
            TsAccessKind::Other => {}
        }
    }
    true
}

impl Npm for RustCode {
    fn compute(node: &Node, _code: &[u8], stats: &mut Stats) {
        match node.kind_id().into() {
            Rust::FunctionItem => {
                // Only count functions directly inside an `impl` or `trait`
                // block. A `FunctionItem` has a body, so the walker already
                // pushed a new FuncSpace for it — we stamp `method_role` on
                // this space and `merge` rolls it up into the parent.
                let grand_kind = match node.parent().and_then(|p| p.parent()) {
                    Some(g) => g.kind_id(),
                    None => return,
                };
                let container = if grand_kind == Rust::ImplItem {
                    SpaceKind::Impl
                } else if grand_kind == Rust::TraitItem {
                    SpaceKind::Trait
                } else {
                    return;
                };
                // Rust visibility: `pub` / `pub(...)` modifier. Trait items
                // are implicitly public by the visibility of the trait they
                // belong to, so count them as public unconditionally.
                let is_public = container == SpaceKind::Trait
                    || node
                        .children()
                        .any(|c| c.kind_id() == Rust::VisibilityModifier);
                record_method(stats, container, is_public);
            }
            Rust::FunctionSignatureItem => {
                // A signature-only trait item (no default body) does not open
                // a FuncSpace, so `stats` here is the trait's own stats. Bump
                // its interface counters directly — there's no child merge to
                // route the count through. Signatures are always public.
                let grand_is_trait = node
                    .parent()
                    .and_then(|p| p.parent())
                    .is_some_and(|g| g.kind_id() == Rust::TraitItem);
                if !grand_is_trait || stats.space_kind != SpaceKind::Trait {
                    return;
                }
                stats.interface_nm += 1;
                stats.interface_npm += 1;
            }
            _ => {}
        }
    }
}

impl Npm for RubyCode {
    fn compute(node: &Node, _code: &[u8], stats: &mut Stats) {
        if !matches!(node.kind_id().into(), Ruby::Method | Ruby::SingletonMethod) {
            return;
        }
        // In the Ruby grammar the class body may be wrapped in a
        // `body_statement` node, so walk one step up to find the owning
        // class/module.
        let mut container = node.parent();
        if let Some(c) = container
            && matches!(c.kind_id().into(), Ruby::BodyStatement)
        {
            container = c.parent();
        }
        let in_class = container.is_some_and(|p| {
            matches!(
                p.kind_id().into(),
                Ruby::Class | Ruby::SingletonClass | Ruby::Module
            )
        });
        if !in_class {
            return;
        }
        // Ruby methods are public by default. Visibility modifiers like
        // `private`/`protected` are method calls that apply to *subsequent*
        // definitions; fully tracking that state requires a scan pass and is
        // out of scope for this single-node compute. Count every method as
        // public here — this matches the common case and keeps the metric
        // useful as a rough signal.
        record_method(stats, SpaceKind::Class, true);
    }
}

// Go has no class-like constructs; Npm is not applicable.
impl Npm for GoCode {
    fn compute(_node: &Node, _code: &[u8], _stats: &mut Stats) {}
}

/// Whether a Kotlin class member is public. Defaults to `public`; explicit
/// `private`/`protected`/`internal` modifiers override.
pub(crate) fn kotlin_member_is_public(node: &Node, code: &[u8]) -> bool {
    for child in node.children() {
        if !matches!(
            child.kind_id().into(),
            Kotlin::Modifiers | Kotlin::ParameterModifiers
        ) {
            continue;
        }
        for m in child.children() {
            if m.kind_id() != Kotlin::VisibilityModifier {
                continue;
            }
            let text = &code[m.start_byte()..m.end_byte()];
            if text == b"private" || text == b"protected" || text == b"internal" {
                return false;
            }
        }
    }
    true
}

/// Returns the `SpaceKind` container for a Kotlin member whose parent node
/// is a `class_body` / `enum_class_body`. The tree-sitter-kotlin grammar
/// uses a single `class_declaration` node for classes, interfaces, and
/// enums, disambiguated only by the `class` / `interface` / `enum`
/// keyword child — so to tell an interface from a class we have to look
/// at the declaration's keyword children.
pub(crate) fn kotlin_member_container(body_parent: &Node) -> Option<SpaceKind> {
    // `body_parent` is a `class_body` / `enum_class_body`. Its parent is
    // the `class_declaration` (or `object_declaration`). For interfaces
    // the declaration contains an `interface` keyword child; otherwise
    // the member lives in a class-like container.
    let decl = body_parent.parent()?;
    match decl.kind_id().into() {
        Kotlin::ClassDeclaration => {
            if decl.children().any(|c| c.kind_id() == Kotlin::Interface) {
                Some(SpaceKind::Interface)
            } else {
                Some(SpaceKind::Class)
            }
        }
        Kotlin::ObjectDeclaration | Kotlin::CompanionObject => Some(SpaceKind::Class),
        _ => None,
    }
}

impl Npm for KotlinCode {
    fn compute(node: &Node, code: &[u8], stats: &mut Stats) {
        if !matches!(
            node.kind_id().into(),
            Kotlin::FunctionDeclaration | Kotlin::SecondaryConstructor
        ) {
            return;
        }
        let parent = match node.parent() {
            Some(p) => p,
            None => return,
        };
        if !matches!(
            parent.kind_id().into(),
            Kotlin::ClassBody | Kotlin::EnumClassBody
        ) {
            return;
        }
        let Some(container) = kotlin_member_container(&parent) else {
            return;
        };
        record_method(stats, container, kotlin_member_is_public(node, code));
    }
}

#[cfg(test)]
mod tests {
    use crate::langs::{
        KotlinParser, PythonParser, RubyParser, RustParser, TsxParser, TypescriptParser,
    };
    use crate::tools::check_metrics;

    #[test]
    fn python_npm_counts_public_and_private_methods() {
        check_metrics::<PythonParser>(
            "class C:
                 def a(self): pass
                 def _b(self): pass
                 def __c(self): pass
                 def __init__(self): pass",
            "foo.py",
            |metric| {
                // public: a, __init__; non-public: _b, __c
                insta::assert_json_snapshot!(
                    metric.npm,
                    @r#"
                {
                  "classes": 2.0,
                  "interfaces": 0.0,
                  "class_methods": 4.0,
                  "interface_methods": 0.0,
                  "classes_average": 0.5,
                  "interfaces_average": null,
                  "total": 2.0,
                  "total_methods": 4.0,
                  "average": 0.5
                }
                "#
                );
            },
        );
    }

    #[test]
    fn typescript_npm_counts_modifiers() {
        check_metrics::<TypescriptParser>(
            "class C {
                 a() {}
                 public b() {}
                 private c() {}
                 protected d() {}
             }",
            "foo.ts",
            |metric| {
                // public: a, b. non-public: c, d.
                insta::assert_json_snapshot!(
                    metric.npm,
                    @r#"
                {
                  "classes": 2.0,
                  "interfaces": 0.0,
                  "class_methods": 4.0,
                  "interface_methods": 0.0,
                  "classes_average": 0.5,
                  "interfaces_average": null,
                  "total": 2.0,
                  "total_methods": 4.0,
                  "average": 0.5
                }
                "#
                );
            },
        );
    }

    #[test]
    fn typescript_npm_counts_ecmascript_private_methods() {
        // ECMAScript `#name` private methods parse as `method_definition`
        // nodes whose name child is `private_property_identifier`; they must
        // be counted as non-public.
        check_metrics::<TypescriptParser>(
            "class C {
                 a() {}
                 #b() {}
                 #c() {}
             }",
            "foo.ts",
            |metric| {
                // 3 methods: public = a only; #b, #c are private.
                insta::assert_json_snapshot!(
                    metric.npm,
                    @r#"
                {
                  "classes": 1.0,
                  "interfaces": 0.0,
                  "class_methods": 3.0,
                  "interface_methods": 0.0,
                  "classes_average": 0.3333333333333333,
                  "interfaces_average": null,
                  "total": 1.0,
                  "total_methods": 3.0,
                  "average": 0.3333333333333333
                }
                "#
                );
            },
        );
    }

    #[test]
    fn tsx_npm_counts_ecmascript_private_methods() {
        // TSX shares the grammar; lock in the same `#name` -> non-public
        // classification so a regression doesn't sneak in through the TSX
        // parser alone.
        check_metrics::<TsxParser>(
            "class C {
                 a() {}
                 #b() {}
             }",
            "foo.tsx",
            |metric| {
                // 2 methods: public = a only; #b is private.
                insta::assert_json_snapshot!(
                    metric.npm,
                    @r#"
                {
                  "classes": 1.0,
                  "interfaces": 0.0,
                  "class_methods": 2.0,
                  "interface_methods": 0.0,
                  "classes_average": 0.5,
                  "interfaces_average": null,
                  "total": 1.0,
                  "total_methods": 2.0,
                  "average": 0.5
                }
                "#
                );
            },
        );
    }

    #[test]
    fn rust_npm_counts_pub_in_impl_block() {
        check_metrics::<RustParser>(
            "struct S;
             impl S {
                 pub fn a(&self) {}
                 fn b(&self) {}
             }",
            "foo.rs",
            |metric| {
                // impl S -> 2 methods, 1 public
                insta::assert_json_snapshot!(
                    metric.npm,
                    @r#"
                {
                  "classes": 1.0,
                  "interfaces": 0.0,
                  "class_methods": 2.0,
                  "interface_methods": 0.0,
                  "classes_average": 0.5,
                  "interfaces_average": null,
                  "total": 1.0,
                  "total_methods": 2.0,
                  "average": 0.5
                }
                "#
                );
            },
        );
    }

    #[test]
    fn rust_npm_counts_trait_signature_and_default_methods() {
        // `function_signature_item` (no body) doesn't open a FuncSpace, so
        // npm has to bump the trait's interface counters directly at that
        // node. Default-bodied fns still flow through the regular FuncSpace
        // + merge path.
        check_metrics::<RustParser>(
            "trait T {
                 fn a(&self);
                 fn b(&self) {}
             }",
            "foo.rs",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.npm,
                    @r#"
                {
                  "classes": 0.0,
                  "interfaces": 2.0,
                  "class_methods": 0.0,
                  "interface_methods": 2.0,
                  "classes_average": null,
                  "interfaces_average": 1.0,
                  "total": 2.0,
                  "total_methods": 2.0,
                  "average": 1.0
                }
                "#
                );
            },
        );
    }

    #[test]
    fn kotlin_npm_counts_visibility_modifiers() {
        check_metrics::<KotlinParser>(
            "class C {
                 fun a() {}
                 public fun b() {}
                 private fun c() {}
                 protected fun d() {}
                 internal fun e() {}
             }",
            "foo.kt",
            |metric| {
                // public: a, b. non-public: c, d, e.
                insta::assert_json_snapshot!(
                    metric.npm,
                    @r#"
                {
                  "classes": 2.0,
                  "interfaces": 0.0,
                  "class_methods": 5.0,
                  "interface_methods": 0.0,
                  "classes_average": 0.4,
                  "interfaces_average": null,
                  "total": 2.0,
                  "total_methods": 5.0,
                  "average": 0.4
                }
                "#
                );
            },
        );
    }

    #[test]
    fn kotlin_npm_routes_interface_methods_to_interface_counters() {
        // tree-sitter-kotlin parses `class` and `interface` into the same
        // `class_declaration` node; only the leading keyword child
        // distinguishes them. Interface methods must land in the
        // interface_methods / interfaces counters, not class_methods /
        // classes. Regression for PR review comment requesting proper
        // class-vs-interface routing.
        check_metrics::<KotlinParser>(
            "interface Foo {
                 fun a()
                 fun b(): Int
             }

             class Bar {
                 fun c() {}
                 fun d() {}
             }",
            "foo.kt",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.npm,
                    @r#"
                {
                  "classes": 2.0,
                  "interfaces": 2.0,
                  "class_methods": 2.0,
                  "interface_methods": 2.0,
                  "classes_average": 1.0,
                  "interfaces_average": 1.0,
                  "total": 4.0,
                  "total_methods": 4.0,
                  "average": 1.0
                }
                "#
                );
            },
        );
    }

    #[test]
    fn kotlin_npm_counts_secondary_constructors() {
        check_metrics::<KotlinParser>(
            "class C {
                 constructor()
                 private constructor(x: Int)
                 internal constructor(y: String)
                 fun visible() {}
             }",
            "foo.kt",
            |metric| {
                // public: default-visible constructor and visible().
                // non-public: private/internal secondary constructors.
                insta::assert_json_snapshot!(
                    metric.npm,
                    @r#"
                {
                  "classes": 2.0,
                  "interfaces": 0.0,
                  "class_methods": 4.0,
                  "interface_methods": 0.0,
                  "classes_average": 0.5,
                  "interfaces_average": null,
                  "total": 2.0,
                  "total_methods": 4.0,
                  "average": 0.5
                }
                "#
                );
            },
        );
    }

    #[test]
    fn ruby_npm_counts_methods_as_public() {
        check_metrics::<RubyParser>(
            "class C
                 def a; end
                 def b; end
             end",
            "foo.rb",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.npm,
                    @r#"
                {
                  "classes": 2.0,
                  "interfaces": 0.0,
                  "class_methods": 2.0,
                  "interface_methods": 0.0,
                  "classes_average": 1.0,
                  "interfaces_average": null,
                  "total": 2.0,
                  "total_methods": 2.0,
                  "average": 1.0
                }
                "#
                );
            },
        );
    }
}
