#[cfg(feature = "markdown")]
use crate::legacy::langs::MarkdownCode;
use crate::legacy::langs::{CCode, GoCode, KotlinCode, PhpCode, RubyCode, RustCode};
use crate::legacy::languages::{C, Go, Kotlin, Php, Ruby, Rust};
use crate::legacy::node::Node;

pub(crate) trait Checker {
    fn is_func_space(_: &Node) -> bool;
    fn is_func(_: &Node) -> bool;
    fn is_closure(_: &Node) -> bool;
    fn is_non_arg(_: &Node) -> bool;
    fn is_else_if(_: &Node) -> bool;
}

impl Checker for RustCode {
    fn is_func_space(node: &Node) -> bool {
        matches!(
            node.kind_id().into(),
            Rust::SourceFile
                | Rust::FunctionItem
                | Rust::ImplItem
                | Rust::TraitItem
                | Rust::ClosureExpression
        )
    }

    fn is_func(node: &Node) -> bool {
        node.kind_id() == Rust::FunctionItem
    }

    fn is_closure(node: &Node) -> bool {
        node.kind_id() == Rust::ClosureExpression
    }

    fn is_non_arg(node: &Node) -> bool {
        matches!(
            node.kind_id().into(),
            Rust::LPAREN | Rust::COMMA | Rust::RPAREN | Rust::PIPE | Rust::AttributeItem
        )
    }

    #[inline(always)]
    fn is_else_if(node: &Node) -> bool {
        if node.kind_id() != Rust::IfExpression {
            return false;
        }
        if let Some(parent) = node.parent() {
            return parent.kind_id() == Rust::ElseClause;
        }
        false
    }
}

impl Checker for GoCode {
    fn is_func_space(node: &Node) -> bool {
        matches!(
            node.kind_id().into(),
            Go::SourceFile | Go::FunctionDeclaration | Go::MethodDeclaration | Go::FuncLiteral
        )
    }

    fn is_func(node: &Node) -> bool {
        matches!(
            node.kind_id().into(),
            Go::FunctionDeclaration | Go::MethodDeclaration
        )
    }

    fn is_closure(node: &Node) -> bool {
        node.kind_id() == Go::FuncLiteral
    }

    fn is_non_arg(node: &Node) -> bool {
        matches!(node.kind_id().into(), Go::LPAREN | Go::COMMA | Go::RPAREN)
    }

    fn is_else_if(node: &Node) -> bool {
        if node.kind_id() != Go::IfStatement {
            return false;
        }
        if let Some(parent) = node.parent() {
            return parent.kind_id() == Go::IfStatement;
        }
        false
    }
}

impl Checker for KotlinCode {
    fn is_func_space(node: &Node) -> bool {
        matches!(
            node.kind_id().into(),
            Kotlin::SourceFile
                | Kotlin::FunctionDeclaration
                | Kotlin::AnonymousFunction
                | Kotlin::LambdaLiteral
                | Kotlin::ClassDeclaration
                | Kotlin::ObjectDeclaration
                | Kotlin::CompanionObject
                | Kotlin::SecondaryConstructor
                | Kotlin::Getter
                | Kotlin::Setter
        )
    }

    fn is_func(node: &Node) -> bool {
        matches!(
            node.kind_id().into(),
            Kotlin::FunctionDeclaration
                | Kotlin::AnonymousFunction
                | Kotlin::SecondaryConstructor
                | Kotlin::Getter
                | Kotlin::Setter
        )
    }

    fn is_closure(node: &Node) -> bool {
        node.kind_id() == Kotlin::LambdaLiteral
    }

    fn is_non_arg(node: &Node) -> bool {
        matches!(
            node.kind_id().into(),
            Kotlin::LPAREN | Kotlin::RPAREN | Kotlin::COMMA
        )
    }

    #[inline(always)]
    fn is_else_if(node: &Node) -> bool {
        // Kotlin has no dedicated `else if` node; an `else if` parses as an
        // inner `if_expression` whose direct parent is a `control_structure_body`
        // (the braced or single-statement body) referenced from the outer
        // `if_expression`. The tree-sitter-kotlin grammar names the two
        // branches: `consequence` (then) and `alternative` (else). We only
        // want to flatten the nesting for the `else if` case, so we check
        // that the `control_structure_body` we live in is the *alternative*
        // of the outer `if_expression` — not its consequence. Otherwise a
        // nested `if` in the then-branch (e.g. `if (a) if (b) ...`) would be
        // incorrectly treated as an `else if` and undercount cognitive
        // complexity.
        if node.kind_id() != Kotlin::IfExpression {
            return false;
        }
        let Some(parent) = node.parent() else {
            return false;
        };
        if parent.kind_id() != Kotlin::ControlStructureBody {
            return false;
        }
        let Some(grand) = parent.parent() else {
            return false;
        };
        if grand.kind_id() != Kotlin::IfExpression {
            return false;
        }
        // Must be sitting in the `alternative` (else) slot of the outer if.
        grand
            .child_by_field_name("alternative")
            .is_some_and(|alt| alt.id() == parent.id())
    }
}

impl Checker for RubyCode {
    fn is_func_space(node: &Node) -> bool {
        match node.kind_id().into() {
            Ruby::Program
            | Ruby::Method
            | Ruby::SingletonMethod
            | Ruby::Class
            | Ruby::Module
            | Ruby::SingletonClass
            | Ruby::Lambda => true,
            // `Block` and `DoBlock` are closure spaces on their own only when
            // they are NOT the direct body of a `Lambda`; otherwise they would
            // double-count the same callable.
            Ruby::Block | Ruby::DoBlock => node
                .parent()
                .is_none_or(|parent| parent.kind_id() != Ruby::Lambda),
            _ => false,
        }
    }

    fn is_func(node: &Node) -> bool {
        matches!(node.kind_id().into(), Ruby::Method | Ruby::SingletonMethod)
    }

    fn is_closure(node: &Node) -> bool {
        match node.kind_id().into() {
            Ruby::Lambda => true,
            // See `is_func_space`: skip lambda-owned blocks.
            Ruby::Block | Ruby::DoBlock => node
                .parent()
                .is_none_or(|parent| parent.kind_id() != Ruby::Lambda),
            _ => false,
        }
    }

    fn is_non_arg(node: &Node) -> bool {
        matches!(
            node.kind_id().into(),
            Ruby::LPAREN | Ruby::RPAREN | Ruby::COMMA | Ruby::PIPE
        )
    }

    #[inline(always)]
    fn is_else_if(node: &Node) -> bool {
        // Ruby has a dedicated `elsif` named node so nested `if` in the `else`
        // branch never appears as an `if` child of another `if`. No special
        // else-if detection is needed.
        if node.kind_id() != Ruby::If {
            return false;
        }
        if let Some(parent) = node.parent() {
            return parent.kind_id() == Ruby::Else;
        }
        false
    }
}

impl Checker for CCode {
    fn is_func_space(node: &Node) -> bool {
        matches!(
            node.kind_id().into(),
            C::TranslationUnit | C::FunctionDefinition | C::FunctionDefinition2
        )
    }

    fn is_func(node: &Node) -> bool {
        matches!(
            node.kind_id().into(),
            C::FunctionDefinition | C::FunctionDefinition2
        )
    }

    fn is_closure(_: &Node) -> bool {
        // C has no closures.
        false
    }

    fn is_non_arg(node: &Node) -> bool {
        matches!(
            node.kind_id().into(),
            C::LPAREN | C::LPAREN2 | C::COMMA | C::RPAREN
        )
    }

    #[inline(always)]
    fn is_else_if(node: &Node) -> bool {
        // C's grammar exposes an explicit `else_clause` wrapper around the
        // nested body. An `if_statement` whose parent is an `else_clause` is
        // the `else if` form and should not increment nesting twice.
        if node.kind_id() != C::IfStatement {
            return false;
        }
        if let Some(parent) = node.parent() {
            return parent.kind_id() == C::ElseClause;
        }
        false
    }
}

impl Checker for PhpCode {
    fn is_func_space(node: &Node) -> bool {
        matches!(
            node.kind_id().into(),
            Php::Program
                | Php::FunctionDefinition
                | Php::MethodDeclaration
                | Php::AnonymousFunction
                | Php::ArrowFunction
                | Php::ClassDeclaration
                | Php::AnonymousClass
                | Php::InterfaceDeclaration
                | Php::TraitDeclaration
                | Php::EnumDeclaration
        )
    }

    fn is_func(node: &Node) -> bool {
        matches!(
            node.kind_id().into(),
            Php::FunctionDefinition | Php::MethodDeclaration
        )
    }

    fn is_closure(node: &Node) -> bool {
        matches!(
            node.kind_id().into(),
            Php::AnonymousFunction | Php::ArrowFunction
        )
    }

    fn is_non_arg(node: &Node) -> bool {
        matches!(
            node.kind_id().into(),
            Php::LPAREN | Php::LPAREN2 | Php::RPAREN | Php::RPAREN2 | Php::COMMA
        )
    }

    #[inline(always)]
    fn is_else_if(node: &Node) -> bool {
        // PHP has a dedicated `else_if_clause` named node for the `elseif`
        // keyword. The `else if` (with a space) form parses as a nested
        // `if_statement` whose direct parent is an `else_clause`. We
        // flatten this case so the cognitive-complexity score matches
        // `elseif`: the inner `if`'s structural cost has already been paid
        // by the outer `else_clause`'s `+1`.
        if node.kind_id() != Php::IfStatement {
            return false;
        }
        if let Some(parent) = node.parent() {
            return matches!(parent.kind_id().into(), Php::ElseClause | Php::ElseClause2);
        }
        false
    }
}

#[cfg(feature = "markdown")]
impl Checker for MarkdownCode {
    // Markdown is a documentation language; its AST has no code-shaped nodes,
    // so the source-code `Checker` predicates all return `false`. The dedicated
    // Markdown analyzer (see `src/markdown/`) bypasses this trait entirely.
    fn is_func_space(_: &Node) -> bool {
        false
    }

    fn is_func(_: &Node) -> bool {
        false
    }

    fn is_closure(_: &Node) -> bool {
        false
    }

    fn is_non_arg(_: &Node) -> bool {
        false
    }

    #[inline(always)]
    fn is_else_if(_: &Node) -> bool {
        false
    }
}
