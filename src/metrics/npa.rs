use serde::Serialize;
use serde::ser::{SerializeStruct, Serializer};
use std::fmt;

use crate::checker::Checker;
use crate::langs::{LANG, *};
use crate::languages::{Kotlin, Python, Ruby, Rust, Tsx, Typescript};
use crate::node::Node;
use crate::spaces::SpaceKind;

/// The `Npa` metric.
///
/// This metric counts the number of public attributes
/// of classes/interfaces.
#[derive(Clone, Debug, Default)]
pub(crate) struct Stats {
    class_npa: usize,
    interface_npa: usize,
    class_na: usize,
    interface_na: usize,
    class_npa_sum: usize,
    interface_npa_sum: usize,
    class_na_sum: usize,
    interface_na_sum: usize,
    space_kind: SpaceKind,
    not_applicable: bool,
    /// True once any class-like or interface-like space has been observed in
    /// the subtree being aggregated. Kept separate from the numeric sums so
    /// an empty class (no attributes) still keeps unit-level `npa` visible.
    has_class_like: bool,
}

impl Serialize for Stats {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut st = serializer.serialize_struct("npa", 9)?;
        st.serialize_field("classes", &self.class_npa_sum())?;
        st.serialize_field("interfaces", &self.interface_npa_sum())?;
        st.serialize_field("class_attributes", &self.class_na_sum())?;
        st.serialize_field("interface_attributes", &self.interface_na_sum())?;
        st.serialize_field("classes_average", &self.class_cda())?;
        st.serialize_field("interfaces_average", &self.interface_cda())?;
        st.serialize_field("total", &self.total_npa())?;
        st.serialize_field("total_attributes", &self.total_na())?;
        st.serialize_field("average", &self.total_cda())?;
        st.end()
    }
}

impl fmt::Display for Stats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "classes: {}, interfaces: {}, class_attributes: {}, interface_attributes: {}, classes_average: {}, interfaces_average: {}, total: {}, total_attributes: {}, average: {}",
            self.class_npa_sum(),
            self.interface_npa_sum(),
            self.class_na_sum(),
            self.interface_na_sum(),
            self.class_cda(),
            self.interface_cda(),
            self.total_npa(),
            self.total_na(),
            self.total_cda()
        )
    }
}

impl Stats {
    /// Merges a second `Npa` metric into the first one
    pub(crate) fn merge(&mut self, other: &Self) {
        self.class_npa_sum += other.class_npa_sum;
        self.interface_npa_sum += other.interface_npa_sum;
        self.class_na_sum += other.class_na_sum;
        self.interface_na_sum += other.interface_na_sum;
        self.not_applicable |= other.not_applicable;
        self.has_class_like |= other.has_class_like;
    }

    /// Increments the attribute counters on the enclosing class/interface
    /// space. `container` specifies whether the attribute belongs to a
    /// class-like (Class / Impl) or interface-like (Interface / Trait) scope.
    #[inline(always)]
    pub(crate) fn record_attribute(&mut self, container: SpaceKind, is_public: bool) {
        match container {
            SpaceKind::Class | SpaceKind::Impl => {
                self.class_na += 1;
                if is_public {
                    self.class_npa += 1;
                }
            }
            SpaceKind::Interface | SpaceKind::Trait => {
                self.interface_na += 1;
                if is_public {
                    self.interface_npa += 1;
                }
            }
            _ => {}
        }
    }

    /// Returns the number of class public attributes sum in a space.
    #[inline(always)]
    pub(crate) fn class_npa_sum(&self) -> f64 {
        self.class_npa_sum as f64
    }

    /// Returns the number of interface public attributes sum in a space.
    #[inline(always)]
    pub(crate) fn interface_npa_sum(&self) -> f64 {
        self.interface_npa_sum as f64
    }

    /// Returns the number of class attributes sum in a space.
    #[inline(always)]
    pub(crate) fn class_na_sum(&self) -> f64 {
        self.class_na_sum as f64
    }

    /// Returns the number of interface attributes sum in a space.
    #[inline(always)]
    pub(crate) fn interface_na_sum(&self) -> f64 {
        self.interface_na_sum as f64
    }

    /// Returns the class `Cda` metric value
    ///
    /// The `Class Data Accessibility` metric value for a class
    /// is computed by dividing the `Npa` value of the class
    /// by the total number of attributes defined in the class.
    ///
    /// This metric is an adaptation of the `Classified Class Data Accessibility` (`CCDA`)
    /// security metric for not classified attributes.
    /// Paper: <https://ieeexplore.ieee.org/abstract/document/5381538>
    #[inline(always)]
    pub(crate) fn class_cda(&self) -> f64 {
        self.class_npa_sum() / self.class_na_sum as f64
    }

    /// Returns the interface `Cda` metric value
    ///
    /// The `Class Data Accessibility` metric value for an interface
    /// is computed by dividing the `Npa` value of the interface
    /// by the total number of attributes defined in the interface.
    ///
    /// This metric is an adaptation of the `Classified Class Data Accessibility` (`CCDA`)
    /// security metric for not classified attributes.
    /// Paper: <https://ieeexplore.ieee.org/abstract/document/5381538>
    #[inline(always)]
    pub(crate) fn interface_cda(&self) -> f64 {
        // For the Java language it's not necessary to compute the metric value
        // The metric value in Java can only be 1.0 or f64:NAN
        if self.interface_npa_sum == self.interface_na_sum && self.interface_npa_sum != 0 {
            1.0
        } else {
            self.interface_npa_sum() / self.interface_na_sum()
        }
    }

    /// Returns the total `Cda` metric value
    ///
    /// The total `Class Data Accessibility` metric value
    /// is computed by dividing the total `Npa` value
    /// by the total number of attributes.
    ///
    /// This metric is an adaptation of the `Classified Class Data Accessibility` (`CCDA`)
    /// security metric for not classified attributes.
    /// Paper: <https://ieeexplore.ieee.org/abstract/document/5381538>
    #[inline(always)]
    pub(crate) fn total_cda(&self) -> f64 {
        self.total_npa() / self.total_na()
    }

    /// Returns the total number of public attributes in a space.
    #[inline(always)]
    pub(crate) fn total_npa(&self) -> f64 {
        self.class_npa_sum() + self.interface_npa_sum()
    }

    /// Returns the total number of attributes in a space.
    #[inline(always)]
    pub(crate) fn total_na(&self) -> f64 {
        self.class_na_sum() + self.interface_na_sum()
    }

    // Accumulates the number of class and interface
    // public and not public attributes into the sums
    #[inline(always)]
    pub(crate) fn compute_sum(&mut self) {
        self.class_npa_sum += self.class_npa;
        self.interface_npa_sum += self.interface_npa;
        self.class_na_sum += self.class_na;
        self.interface_na_sum += self.interface_na;
    }

    // Checks if the `Npa` metric is disabled
    #[inline(always)]
    pub(crate) fn is_disabled(&self) -> bool {
        if self.not_applicable {
            return true;
        }
        match self.space_kind {
            SpaceKind::Function | SpaceKind::Unknown => true,
            // Use the presence flag — an empty class (no attributes) is
            // still class-like and should keep `npa` visible at the unit.
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

    /// Returns whether the `Npa` metric is meaningful for the given language.
    /// Languages without class-like constructs opt out.
    #[inline(always)]
    pub(crate) fn applies_to(lang: LANG) -> bool {
        !matches!(lang, LANG::Go)
    }

    /// Records the kind of the enclosing space. Also flags the stats as
    /// having observed a class-like space if applicable, so unit-level
    /// aggregation can distinguish "no classes" from "classes with no
    /// counted attributes".
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

pub(crate) trait Npa
where
    Self: Checker,
{
    fn compute(node: &Node, code: &[u8], stats: &mut Stats);
}

/// Whether a Python attribute name should be considered public. Dunder names
/// (e.g. `__init__`) are public by convention; single or double leading
/// underscore signals non-public.
fn python_attr_is_public(name: &str) -> bool {
    if name.starts_with("__") && name.ends_with("__") {
        return true;
    }
    !name.starts_with('_')
}

impl Npa for PythonCode {
    fn compute(node: &Node, code: &[u8], stats: &mut Stats) {
        if !matches!(
            stats.space_kind,
            SpaceKind::Class | SpaceKind::Interface | SpaceKind::Impl | SpaceKind::Trait
        ) {
            return;
        }
        // Python class attributes are assignments at the top level of a class
        // body: `name: T = value`, `name = value`.
        let parent = match node.parent() {
            Some(p) => p,
            None => return,
        };
        // Direct child of class body: parent is the `block`, grand-parent is
        // the class definition.
        let grand = match parent.parent() {
            Some(g) => g,
            None => return,
        };
        if grand.kind_id() != Python::ClassDefinition {
            return;
        }
        // The attribute is an `expression_statement` wrapping an `assignment`
        // whose left side is a bare identifier.
        if node.kind_id() != Python::ExpressionStatement {
            return;
        }
        let inner = match node.child(0) {
            Some(c) => c,
            None => return,
        };
        if inner.kind_id() != Python::Assignment {
            return;
        }
        let left = match inner.child_by_field_name("left") {
            Some(l) => l,
            None => return,
        };
        if left.kind_id() != Python::Identifier {
            return;
        }
        let text = match std::str::from_utf8(&code[left.start_byte()..left.end_byte()]) {
            Ok(t) => t,
            Err(_) => return,
        };
        stats.record_attribute(SpaceKind::Class, python_attr_is_public(text));
    }
}

impl Npa for TypescriptCode {
    fn compute(node: &Node, code: &[u8], stats: &mut Stats) {
        let kind_id = node.kind_id();
        let container = if kind_id == Typescript::PublicFieldDefinition {
            SpaceKind::Class
        } else if kind_id == Typescript::PropertySignature {
            SpaceKind::Interface
        } else {
            return;
        };
        let is_public = ts_field_is_public(node, code, |id| match id.into() {
            Typescript::AccessibilityModifier => TsFieldKind::Modifier,
            Typescript::PrivatePropertyIdentifier => TsFieldKind::PrivateName,
            _ => TsFieldKind::Other,
        });
        stats.record_attribute(container, is_public);
    }
}

impl Npa for TsxCode {
    fn compute(node: &Node, code: &[u8], stats: &mut Stats) {
        let kind_id = node.kind_id();
        let container = if kind_id == Tsx::PublicFieldDefinition {
            SpaceKind::Class
        } else if kind_id == Tsx::PropertySignature {
            SpaceKind::Interface
        } else {
            return;
        };
        let is_public = ts_field_is_public(node, code, |id| match id.into() {
            Tsx::AccessibilityModifier => TsFieldKind::Modifier,
            Tsx::PrivatePropertyIdentifier => TsFieldKind::PrivateName,
            _ => TsFieldKind::Other,
        });
        stats.record_attribute(container, is_public);
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TsFieldKind {
    Modifier,
    /// A `#name` field identifier (ECMAScript private class fields).
    PrivateName,
    Other,
}

fn ts_field_is_public(node: &Node, code: &[u8], classify: impl Fn(u16) -> TsFieldKind) -> bool {
    for child in node.children() {
        match classify(child.kind_id()) {
            TsFieldKind::Modifier => {
                let text = &code[child.start_byte()..child.end_byte()];
                if text == b"private" || text == b"protected" {
                    return false;
                }
            }
            TsFieldKind::PrivateName => return false,
            TsFieldKind::Other => {}
        }
    }
    true
}

impl Npa for RustCode {
    fn compute(node: &Node, _code: &[u8], stats: &mut Stats) {
        // Rust struct fields live in `struct_item`, which is *not* pushed as
        // a FuncSpace, so attributes are collected at the containing
        // unit/impl scope. Two shapes to handle:
        //   - `struct S { pub a: u32, b: u32 }` -> each field is a
        //     `FieldDeclaration` named node; `pub` is a visibility child.
        //   - `struct S(pub u32, u32)` -> fields live directly as type
        //     children of `OrderedFieldDeclarationList`, with an optional
        //     preceding `visibility_modifier` sibling.
        match node.kind_id().into() {
            Rust::FieldDeclaration => {
                let is_public = node
                    .children()
                    .any(|c| c.kind_id() == Rust::VisibilityModifier);
                stats.record_attribute(SpaceKind::Class, is_public);
            }
            Rust::OrderedFieldDeclarationList => {
                // Walk positional fields, pairing each type with the
                // immediately preceding `visibility_modifier` if any.
                let mut pending_pub = false;
                for child in node.children() {
                    match child.kind_id().into() {
                        Rust::LPAREN | Rust::RPAREN | Rust::COMMA => {
                            pending_pub = false;
                        }
                        Rust::AttributeItem => {}
                        Rust::VisibilityModifier => {
                            pending_pub = true;
                        }
                        _ => {
                            stats.record_attribute(SpaceKind::Class, pending_pub);
                            pending_pub = false;
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

impl Npa for RubyCode {
    fn compute(node: &Node, _code: &[u8], stats: &mut Stats) {
        if !matches!(stats.space_kind, SpaceKind::Class) {
            return;
        }
        // Ruby attributes exposed through `attr_accessor`, `attr_reader`,
        // `attr_writer` are public by convention. Detecting those requires a
        // call-site match — treat any `@instance_variable` assignment that's a
        // direct statement of the class body as an attribute, and count it as
        // non-public (encapsulated) by default.
        if node.kind_id() != Ruby::Assignment {
            return;
        }
        // Ruby class bodies are wrapped in a `body_statement` node, so walk
        // one step up if we land on that wrapper. Mirrors the same hop done
        // in the npm detector.
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
        let left = match node.child_by_field_name("left") {
            Some(l) => l,
            None => return,
        };
        if left.kind_id() != Ruby::InstanceVariable {
            return;
        }
        stats.record_attribute(SpaceKind::Class, false);
    }
}

// Go has no class-like constructs; Npa is not applicable.
impl Npa for GoCode {
    fn compute(_node: &Node, _code: &[u8], _stats: &mut Stats) {}
}

/// Whether a Kotlin class-body property is public. Kotlin members default to
/// `public`; explicit `private`/`protected`/`internal` visibility modifiers
/// override that default.
fn kotlin_visibility_is_public(node: &Node, code: &[u8]) -> bool {
    for child in node.children() {
        if child.kind_id() != Kotlin::Modifiers {
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

impl Npa for KotlinCode {
    fn compute(node: &Node, code: &[u8], stats: &mut Stats) {
        if !matches!(
            stats.space_kind,
            SpaceKind::Class | SpaceKind::Interface | SpaceKind::Impl | SpaceKind::Trait
        ) {
            return;
        }
        if node.kind_id() != Kotlin::PropertyDeclaration {
            return;
        }
        // Must be a direct member of a class/object/interface body, not a
        // local `val`/`var` inside a function.
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
        // Route the attribute to class vs interface counters based on the
        // enclosing declaration's keyword (see `kotlin_member_container`),
        // so Kotlin interfaces don't contaminate class NPA totals.
        let Some(container) = crate::metrics::npm::kotlin_member_container(&parent) else {
            return;
        };
        stats.record_attribute(container, kotlin_visibility_is_public(node, code));
    }
}

#[cfg(test)]
mod tests {
    use crate::langs::{KotlinParser, PythonParser, RubyParser, RustParser, TypescriptParser};
    use crate::tools::check_metrics;

    #[test]
    fn python_npa_counts_class_body_assignments() {
        check_metrics::<PythonParser>(
            "class C:
                 a = 1
                 _b = 2
                 __c = 3",
            "foo.py",
            |metric| {
                // public: a. non-public: _b, __c.
                insta::assert_json_snapshot!(
                    metric.npa,
                    @r###"
                    {
                      "classes": 1.0,
                      "interfaces": 0.0,
                      "class_attributes": 3.0,
                      "interface_attributes": 0.0,
                      "classes_average": 0.3333333333333333,
                      "interfaces_average": null,
                      "total": 1.0,
                      "total_attributes": 3.0,
                      "average": 0.3333333333333333
                    }"###
                );
            },
        );
    }

    #[test]
    fn python_npa_counts_annotated_class_attributes() {
        // PEP-526 class annotations (`x: T = val`, `x: T`) parse as
        // expression_statement > assignment, so the existing detector should
        // already cover them. Lock that in: both annotated-with-default and
        // annotated-only class attributes count as attributes.
        check_metrics::<PythonParser>(
            "class C:
                 a: int = 1
                 b: int
                 _c: str = 'x'
                 __d: bool",
            "foo.py",
            |metric| {
                // 4 attributes total. Public: a, b. Non-public: _c, __d.
                insta::assert_json_snapshot!(
                    metric.npa,
                    @r###"
                    {
                      "classes": 2.0,
                      "interfaces": 0.0,
                      "class_attributes": 4.0,
                      "interface_attributes": 0.0,
                      "classes_average": 0.5,
                      "interfaces_average": null,
                      "total": 2.0,
                      "total_attributes": 4.0,
                      "average": 0.5
                    }"###
                );
            },
        );
    }

    #[test]
    fn typescript_npa_counts_public_fields() {
        check_metrics::<TypescriptParser>(
            "class C {
                 a: number = 1;
                 public b: number = 2;
                 private c: number = 3;
                 protected d: number = 4;
             }",
            "foo.ts",
            |metric| {
                // public: a, b. non-public: c, d.
                insta::assert_json_snapshot!(
                    metric.npa,
                    @r###"
                    {
                      "classes": 2.0,
                      "interfaces": 0.0,
                      "class_attributes": 4.0,
                      "interface_attributes": 0.0,
                      "classes_average": 0.5,
                      "interfaces_average": null,
                      "total": 2.0,
                      "total_attributes": 4.0,
                      "average": 0.5
                    }"###
                );
            },
        );
    }

    #[test]
    fn typescript_npa_counts_ecmascript_private_fields() {
        // ECMAScript `#name` private fields parse as `public_field_definition`
        // nodes whose name child is `private_property_identifier`; they must
        // be counted as non-public.
        check_metrics::<TypescriptParser>(
            "class C {
                 a: number = 1;
                 #b: number = 2;
                 #c: number = 3;
             }",
            "foo.ts",
            |metric| {
                // 3 fields: public = a only; #b, #c are private.
                insta::assert_json_snapshot!(
                    metric.npa,
                    @r###"
                    {
                      "classes": 1.0,
                      "interfaces": 0.0,
                      "class_attributes": 3.0,
                      "interface_attributes": 0.0,
                      "classes_average": 0.3333333333333333,
                      "interfaces_average": null,
                      "total": 1.0,
                      "total_attributes": 3.0,
                      "average": 0.3333333333333333
                    }"###
                );
            },
        );
    }

    #[test]
    fn rust_npa_counts_struct_fields() {
        check_metrics::<RustParser>(
            "struct S {
                 pub a: u32,
                 b: u32,
             }",
            "foo.rs",
            |metric| {
                // 2 fields, 1 public.
                insta::assert_json_snapshot!(
                    metric.npa,
                    @r###"
                    {
                      "classes": 1.0,
                      "interfaces": 0.0,
                      "class_attributes": 2.0,
                      "interface_attributes": 0.0,
                      "classes_average": 0.5,
                      "interfaces_average": null,
                      "total": 1.0,
                      "total_attributes": 2.0,
                      "average": 0.5
                    }"###
                );
            },
        );
    }

    #[test]
    fn rust_npa_counts_tuple_struct_fields() {
        // Tuple-struct fields live as type children of
        // `ordered_field_declaration_list` with an optional preceding
        // `visibility_modifier`; they must count the same as named fields.
        check_metrics::<RustParser>("struct S(pub u32, u32);", "foo.rs", |metric| {
            // 2 positional fields, 1 public.
            insta::assert_json_snapshot!(
                metric.npa,
                @r###"
                    {
                      "classes": 1.0,
                      "interfaces": 0.0,
                      "class_attributes": 2.0,
                      "interface_attributes": 0.0,
                      "classes_average": 0.5,
                      "interfaces_average": null,
                      "total": 1.0,
                      "total_attributes": 2.0,
                      "average": 0.5
                    }"###
            );
        });
    }

    #[test]
    fn kotlin_npa_counts_class_properties() {
        check_metrics::<KotlinParser>(
            "class C {
                 val a: Int = 1
                 private val b: Int = 2
                 protected val c: Int = 3
                 internal val d: Int = 4
             }",
            "foo.kt",
            |metric| {
                // public: a. non-public: b, c, d.
                insta::assert_json_snapshot!(
                    metric.npa,
                    @r###"
                    {
                      "classes": 1.0,
                      "interfaces": 0.0,
                      "class_attributes": 4.0,
                      "interface_attributes": 0.0,
                      "classes_average": 0.25,
                      "interfaces_average": null,
                      "total": 1.0,
                      "total_attributes": 4.0,
                      "average": 0.25
                    }"###
                );
            },
        );
    }

    #[test]
    fn kotlin_npa_routes_interface_properties_to_interface_counters() {
        // Same class-vs-interface routing concern as NPM: tree-sitter-kotlin
        // uses `class_declaration` for both classes and interfaces, so the
        // container must be decided by the declaration's leading keyword.
        check_metrics::<KotlinParser>(
            "interface Foo {
                 val a: Int
                 val b: Int
             }

             class Bar {
                 val c: Int = 1
                 private val d: Int = 2
             }",
            "foo.kt",
            |metric| {
                // class Bar: 2 attrs, 1 public; interface Foo: 2 attrs, 2 public.
                insta::assert_json_snapshot!(
                    metric.npa,
                    @r###"
                    {
                      "classes": 1.0,
                      "interfaces": 2.0,
                      "class_attributes": 2.0,
                      "interface_attributes": 2.0,
                      "classes_average": 0.5,
                      "interfaces_average": 1.0,
                      "total": 3.0,
                      "total_attributes": 4.0,
                      "average": 0.75
                    }"###
                );
            },
        );
    }

    #[test]
    fn ruby_npa_counts_instance_variables_under_body_statement() {
        // Ruby class bodies are wrapped in a `body_statement` node in the
        // tree-sitter grammar, so the ivar assignment's parent is
        // `body_statement`, not `class` directly. The detector must hop
        // through the wrapper.
        check_metrics::<RubyParser>(
            "class C
                 @x = 1
                 @y = 2
             end",
            "foo.rb",
            |metric| {
                // 2 ivar attributes, both non-public by convention.
                insta::assert_json_snapshot!(
                    metric.npa,
                    @r###"
                    {
                      "classes": 0.0,
                      "interfaces": 0.0,
                      "class_attributes": 2.0,
                      "interface_attributes": 0.0,
                      "classes_average": 0.0,
                      "interfaces_average": null,
                      "total": 0.0,
                      "total_attributes": 2.0,
                      "average": 0.0
                    }"###
                );
            },
        );
    }
}
