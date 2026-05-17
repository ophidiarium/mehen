use crate::legacy::languages::Rust;
use crate::legacy::node::Node;

fn parent_kind(node: &Node) -> Option<Rust> {
    node.parent().map(|parent| parent.kind_id().into())
}

pub(crate) fn is_inside_rust_macro_tokens(node: &Node) -> bool {
    let mut current = *node;
    while let Some(parent) = current.parent() {
        if matches!(
            parent.kind_id().into(),
            Rust::MacroInvocation | Rust::MacroDefinition
        ) {
            return true;
        }
        current = parent;
    }
    false
}

pub(crate) fn is_rust_comparison_operator(node: &Node) -> bool {
    matches!(
        node.kind_id().into(),
        Rust::EQEQ | Rust::BANGEQ | Rust::LT | Rust::GT | Rust::LTEQ | Rust::GTEQ
    ) && parent_kind(node).is_some_and(|kind| kind == Rust::BinaryExpression)
}

pub(crate) fn is_rust_logical_operator(node: &Node) -> bool {
    matches!(node.kind_id().into(), Rust::AMPAMP | Rust::PIPEPIPE)
        && parent_kind(node).is_some_and(|kind| {
            matches!(
                kind,
                Rust::BinaryExpression | Rust::LetChain | Rust::LetChain2
            )
        })
}

pub(crate) fn is_rust_tail_expression(node: &Node) -> bool {
    parent_kind(node).is_some_and(|kind| kind == Rust::Block)
        && matches!(
            node.kind_id().into(),
            Rust::Identifier
                | Rust::Zelf
                | Rust::Super
                | Rust::Crate
                | Rust::IntegerLiteral
                | Rust::FloatLiteral
                | Rust::BooleanLiteral
                | Rust::StringLiteral
                | Rust::RawStringLiteral
                | Rust::CharLiteral
                | Rust::MacroInvocation
                | Rust::ScopedIdentifier
                | Rust::RangeExpression
                | Rust::UnaryExpression
                | Rust::TryExpression
                | Rust::ReferenceExpression
                | Rust::BinaryExpression
                | Rust::AssignmentExpression
                | Rust::CompoundAssignmentExpr
                | Rust::TypeCastExpression
                | Rust::ReturnExpression
                | Rust::YieldExpression
                | Rust::CallExpression
                | Rust::ArrayExpression
                | Rust::ParenthesizedExpression
                | Rust::TupleExpression
                | Rust::UnitExpression
                | Rust::StructExpression
                | Rust::IfExpression
                | Rust::MatchExpression
                | Rust::WhileExpression
                | Rust::LoopExpression
                | Rust::ForExpression
                | Rust::Block
                | Rust::ConstBlock
                | Rust::ClosureExpression
                | Rust::BreakExpression
                | Rust::ContinueExpression
                | Rust::IndexExpression
                | Rust::AwaitExpression
                | Rust::FieldExpression
                | Rust::UnsafeBlock
                | Rust::AsyncBlock
                | Rust::GenBlock
                | Rust::TryBlock
        )
}
