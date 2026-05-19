use crate::legacy::langs::CCode;
#[cfg(feature = "markdown")]
use crate::legacy::langs::MarkdownCode;
use crate::legacy::languages::C;
use crate::legacy::node::Node;

pub(crate) trait Checker {
    fn is_func_space(_: &Node) -> bool;
    fn is_func(_: &Node) -> bool;
    fn is_closure(_: &Node) -> bool;
    fn is_non_arg(_: &Node) -> bool;
    fn is_else_if(_: &Node) -> bool;
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
