use crate::langs::{
    GoCode, KotlinCode, PowershellCode, PythonCode, RubyCode, RustCode, TsxCode, TsxParser,
    TypescriptCode, TypescriptParser,
};
use crate::languages::{Go, Kotlin, Powershell, Python, Ruby, Rust, Tsx, Typescript};
use crate::node::Node;

macro_rules! check_if_func {
    ($parser: ident, $node: ident) => {
        $node.count_specific_ancestors::<$parser>(
            |node| {
                matches!(
                    node.kind_id().into(),
                    VariableDeclarator | AssignmentExpression | LabeledStatement | Pair
                )
            },
            |node| {
                matches!(
                    node.kind_id().into(),
                    StatementBlock | ReturnStatement | NewExpression | Arguments
                )
            },
        ) > 0
            || $node.is_child(Identifier as u16)
    };
}

macro_rules! check_if_arrow_func {
    ($parser: ident, $node: ident) => {
        $node.count_specific_ancestors::<$parser>(
            |node| {
                matches!(
                    node.kind_id().into(),
                    VariableDeclarator | AssignmentExpression | LabeledStatement
                )
            },
            |node| {
                matches!(
                    node.kind_id().into(),
                    StatementBlock | ReturnStatement | NewExpression | CallExpression
                )
            },
        ) > 0
            || $node.has_sibling(PropertyIdentifier as u16)
    };
}

macro_rules! is_js_func {
    ($parser: ident, $node: ident) => {
        match $node.kind_id().into() {
            FunctionDeclaration | MethodDefinition => true,
            FunctionExpression => check_if_func!($parser, $node),
            ArrowFunction => check_if_arrow_func!($parser, $node),
            _ => false,
        }
    };
}

macro_rules! is_js_closure {
    ($parser: ident, $node: ident) => {
        match $node.kind_id().into() {
            GeneratorFunction | GeneratorFunctionDeclaration => true,
            FunctionExpression => !check_if_func!($parser, $node),
            ArrowFunction => !check_if_arrow_func!($parser, $node),
            _ => false,
        }
    };
}

macro_rules! is_js_func_and_closure_checker {
    ($parser: ident, $language: ident) => {
        #[inline(always)]
        fn is_func(node: &Node) -> bool {
            use $language::*;
            is_js_func!($parser, node)
        }

        #[inline(always)]
        fn is_closure(node: &Node) -> bool {
            use $language::*;
            is_js_closure!($parser, node)
        }
    };
}

pub(crate) trait Checker {
    fn is_comment(_: &Node) -> bool;
    fn is_func_space(_: &Node) -> bool;
    fn is_func(_: &Node) -> bool;
    fn is_closure(_: &Node) -> bool;
    fn is_call(_: &Node) -> bool;
    fn is_non_arg(_: &Node) -> bool;
    fn is_string(_: &Node) -> bool;
    fn is_else_if(_: &Node) -> bool;

    fn is_error(node: &Node) -> bool {
        node.has_error()
    }
}

impl Checker for PythonCode {
    fn is_comment(node: &Node) -> bool {
        node.kind_id() == Python::Comment
    }

    fn is_func_space(node: &Node) -> bool {
        matches!(
            node.kind_id().into(),
            Python::Module | Python::FunctionDefinition | Python::ClassDefinition
        )
    }

    fn is_func(node: &Node) -> bool {
        node.kind_id() == Python::FunctionDefinition
    }

    fn is_closure(node: &Node) -> bool {
        node.kind_id() == Python::Lambda
    }

    fn is_call(node: &Node) -> bool {
        node.kind_id() == Python::Call
    }

    fn is_non_arg(node: &Node) -> bool {
        matches!(
            node.kind_id().into(),
            Python::LPAREN | Python::COMMA | Python::RPAREN
        )
    }

    fn is_string(node: &Node) -> bool {
        node.kind_id() == Python::String || node.kind_id() == Python::ConcatenatedString
    }

    fn is_else_if(_: &Node) -> bool {
        false
    }
}

impl Checker for TypescriptCode {
    fn is_comment(node: &Node) -> bool {
        node.kind_id() == Typescript::Comment
    }

    fn is_func_space(node: &Node) -> bool {
        matches!(
            node.kind_id().into(),
            Typescript::Program
                | Typescript::FunctionExpression
                | Typescript::Class
                | Typescript::GeneratorFunction
                | Typescript::FunctionDeclaration
                | Typescript::MethodDefinition
                | Typescript::GeneratorFunctionDeclaration
                | Typescript::ClassDeclaration
                | Typescript::InterfaceDeclaration
                | Typescript::ArrowFunction
        )
    }

    is_js_func_and_closure_checker!(TypescriptParser, Typescript);

    fn is_call(node: &Node) -> bool {
        node.kind_id() == Typescript::CallExpression
    }

    fn is_non_arg(node: &Node) -> bool {
        matches!(
            node.kind_id().into(),
            Typescript::LPAREN | Typescript::COMMA | Typescript::RPAREN
        )
    }

    fn is_string(node: &Node) -> bool {
        node.kind_id() == Typescript::String || node.kind_id() == Typescript::TemplateString
    }

    #[inline(always)]
    fn is_else_if(node: &Node) -> bool {
        if node.kind_id() != Typescript::IfStatement {
            return false;
        }
        if let Some(parent) = node.parent() {
            return parent.kind_id() == Typescript::ElseClause;
        }
        false
    }
}

impl Checker for TsxCode {
    fn is_comment(node: &Node) -> bool {
        node.kind_id() == Tsx::Comment
    }

    fn is_func_space(node: &Node) -> bool {
        matches!(
            node.kind_id().into(),
            Tsx::Program
                | Tsx::FunctionExpression
                | Tsx::Class
                | Tsx::GeneratorFunction
                | Tsx::FunctionDeclaration
                | Tsx::MethodDefinition
                | Tsx::GeneratorFunctionDeclaration
                | Tsx::ClassDeclaration
                | Tsx::InterfaceDeclaration
                | Tsx::ArrowFunction
        )
    }

    is_js_func_and_closure_checker!(TsxParser, Tsx);

    fn is_call(node: &Node) -> bool {
        node.kind_id() == Tsx::CallExpression
    }

    fn is_non_arg(node: &Node) -> bool {
        matches!(
            node.kind_id().into(),
            Tsx::LPAREN | Tsx::COMMA | Tsx::RPAREN
        )
    }

    fn is_string(node: &Node) -> bool {
        node.kind_id() == Tsx::String || node.kind_id() == Tsx::TemplateString
    }

    fn is_else_if(node: &Node) -> bool {
        if node.kind_id() != Tsx::IfStatement {
            return false;
        }
        if let Some(parent) = node.parent() {
            return node.kind_id() == Tsx::IfStatement && parent.kind_id() == Tsx::IfStatement;
        }
        false
    }
}

impl Checker for RustCode {
    fn is_comment(node: &Node) -> bool {
        node.kind_id() == Rust::LineComment || node.kind_id() == Rust::BlockComment
    }

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

    fn is_call(node: &Node) -> bool {
        node.kind_id() == Rust::CallExpression
    }

    fn is_non_arg(node: &Node) -> bool {
        matches!(
            node.kind_id().into(),
            Rust::LPAREN | Rust::COMMA | Rust::RPAREN | Rust::PIPE | Rust::AttributeItem
        )
    }

    fn is_string(node: &Node) -> bool {
        node.kind_id() == Rust::StringLiteral || node.kind_id() == Rust::RawStringLiteral
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
    fn is_comment(node: &Node) -> bool {
        node.kind_id() == Go::Comment
    }

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

    fn is_call(node: &Node) -> bool {
        node.kind_id() == Go::CallExpression
    }

    fn is_non_arg(node: &Node) -> bool {
        matches!(node.kind_id().into(), Go::LPAREN | Go::COMMA | Go::RPAREN)
    }

    fn is_string(node: &Node) -> bool {
        matches!(
            node.kind_id().into(),
            Go::RawStringLiteral | Go::InterpretedStringLiteral
        )
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
    fn is_comment(node: &Node) -> bool {
        matches!(
            node.kind_id().into(),
            Kotlin::LineComment | Kotlin::MultilineComment
        )
    }

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

    fn is_call(node: &Node) -> bool {
        node.kind_id() == Kotlin::CallExpression
    }

    fn is_non_arg(node: &Node) -> bool {
        matches!(
            node.kind_id().into(),
            Kotlin::LPAREN | Kotlin::RPAREN | Kotlin::COMMA
        )
    }

    fn is_string(node: &Node) -> bool {
        node.kind_id() == Kotlin::StringLiteral
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
    fn is_comment(node: &Node) -> bool {
        node.kind_id() == Ruby::Comment
    }

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

    fn is_call(node: &Node) -> bool {
        // The Ruby grammar aliases several production rules to `call`;
        // the enum generator deduplicates them into Call/Call2/Call3/Call4.
        // Call5 is the internal `_call` supertype and is not emitted as a node kind.
        matches!(
            node.kind_id().into(),
            Ruby::Call | Ruby::Call2 | Ruby::Call3 | Ruby::Call4
        )
    }

    fn is_non_arg(node: &Node) -> bool {
        matches!(
            node.kind_id().into(),
            Ruby::LPAREN | Ruby::RPAREN | Ruby::COMMA | Ruby::PIPE
        )
    }

    fn is_string(node: &Node) -> bool {
        matches!(
            node.kind_id().into(),
            Ruby::String | Ruby::ChainedString | Ruby::HeredocBeginning
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

impl Checker for PowershellCode {
    fn is_comment(node: &Node) -> bool {
        node.kind_id() == Powershell::Comment
    }

    fn is_func_space(node: &Node) -> bool {
        matches!(
            node.kind_id().into(),
            Powershell::Program
                | Powershell::FunctionStatement
                | Powershell::ClassStatement
                | Powershell::ClassMethodDefinition
                | Powershell::ScriptBlockExpression
        )
    }

    fn is_func(node: &Node) -> bool {
        matches!(
            node.kind_id().into(),
            Powershell::FunctionStatement | Powershell::ClassMethodDefinition
        )
    }

    fn is_closure(node: &Node) -> bool {
        // Anonymous script block like `{ param(...) ... }` used as a value or
        // command argument — PowerShell's closest equivalent to a lambda.
        node.kind_id() == Powershell::ScriptBlockExpression
    }

    fn is_call(node: &Node) -> bool {
        // PowerShell has two call forms:
        //   - `command`: cmdlet / command-style invocation (`Get-Thing -Arg x`)
        //   - `invokation_expression`: method / `::` / member call
        matches!(
            node.kind_id().into(),
            Powershell::Command | Powershell::InvokationExpression
        )
    }

    fn is_non_arg(node: &Node) -> bool {
        matches!(
            node.kind_id().into(),
            Powershell::LPAREN | Powershell::RPAREN | Powershell::COMMA
        )
    }

    fn is_string(node: &Node) -> bool {
        matches!(
            node.kind_id().into(),
            Powershell::StringLiteral
                | Powershell::ExpandableStringLiteral
                | Powershell::ExpandableHereStringLiteral
        )
    }

    #[inline(always)]
    fn is_else_if(_node: &Node) -> bool {
        // PowerShell has a dedicated `elseif_clause` named node, so a nested
        // `if_statement` never appears as the body of another `if_statement`.
        // No flattening needed.
        false
    }
}
