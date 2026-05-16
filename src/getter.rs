#[cfg(feature = "markdown")]
use crate::langs::MarkdownCode;
use crate::langs::{
    CCode, GoCode, KotlinCode, PhpCode, PowershellCode, PythonCode, RubyCode, RustCode, TsxCode,
    TypescriptCode,
};
use crate::languages::{C, Kotlin, Php, Powershell, Python, Ruby, Rust, Tsx, Typescript};
use crate::metrics::halstead::HalsteadType;
use crate::node::Node;
use crate::spaces::SpaceKind;

pub(crate) trait Getter {
    fn get_func_name<'a>(node: &Node, code: &'a [u8]) -> Option<&'a str> {
        Self::get_func_space_name(node, code)
    }

    fn get_func_space_name<'a>(node: &Node, code: &'a [u8]) -> Option<&'a str> {
        // we're in a function or in a class
        if let Some(name) = node.child_by_field_name("name") {
            let code = &code[name.start_byte()..name.end_byte()];
            std::str::from_utf8(code).ok()
        } else {
            Some("<anonymous>")
        }
    }

    fn get_space_kind(_node: &Node) -> SpaceKind {
        SpaceKind::Unknown
    }

    fn get_op_type(_node: &Node) -> HalsteadType {
        HalsteadType::Unknown
    }
}

impl Getter for PythonCode {
    fn get_space_kind(node: &Node) -> SpaceKind {
        match node.kind_id().into() {
            Python::FunctionDefinition => SpaceKind::Function,
            Python::ClassDefinition => SpaceKind::Class,
            Python::Module => SpaceKind::Unit,
            _ => SpaceKind::Unknown,
        }
    }

    fn get_op_type(node: &Node) -> HalsteadType {
        use Python::*;

        match node.kind_id().into() {
            Import | DOT | From | COMMA | As | STAR | GTGT | Assert | COLONEQ | Return | Def
            | Del | Raise | Pass | Break | Continue | If | Elif | Else | Async | For | In
            | While | Try | Except | Finally | With | DASHGT | EQ | Global | Exec | AT | Not
            | And | Or | PLUS | DASH | SLASH | PERCENT | SLASHSLASH | STARSTAR | PIPE | AMP
            | CARET | LTLT | TILDE | LT | LTEQ | EQEQ | BANGEQ | GTEQ | GT | LTGT | Is | PLUSEQ
            | DASHEQ | STAREQ | SLASHEQ | ATEQ | SLASHSLASHEQ | PERCENTEQ | STARSTAREQ | GTGTEQ
            | LTLTEQ | AMPEQ | CARETEQ | PIPEEQ | Yield | Await | Await2 | Print | LPAREN
            | LBRACK | LBRACE | COLON | SEMI => HalsteadType::Operator,
            Identifier | Integer | Float | True | False | None => HalsteadType::Operand,
            String => {
                let mut operator = HalsteadType::Unknown;
                // check if we've a documentation string or a multiline comment
                if let Some(parent) = node.parent()
                    && (parent.kind_id() != ExpressionStatement || parent.child_count() != 1)
                {
                    operator = HalsteadType::Operand;
                };
                operator
            }
            _ => HalsteadType::Unknown,
        }
    }
}

impl Getter for TypescriptCode {
    fn get_space_kind(node: &Node) -> SpaceKind {
        use Typescript::*;

        match node.kind_id().into() {
            FunctionExpression
            | MethodDefinition
            | GeneratorFunction
            | FunctionDeclaration
            | GeneratorFunctionDeclaration
            | ArrowFunction => SpaceKind::Function,
            Class | ClassDeclaration => SpaceKind::Class,
            InterfaceDeclaration => SpaceKind::Interface,
            Program => SpaceKind::Unit,
            _ => SpaceKind::Unknown,
        }
    }

    fn get_func_space_name<'a>(node: &Node, code: &'a [u8]) -> Option<&'a str> {
        if let Some(name) = node.child_by_field_name("name") {
            let code = &code[name.start_byte()..name.end_byte()];
            std::str::from_utf8(code).ok()
        } else {
            // We can be in a pair: foo: function() {}
            // Or in a variable declaration: var aFun = function() {}
            if let Some(parent) = node.parent() {
                match parent.kind_id().into() {
                    Typescript::Pair => {
                        if let Some(name) = parent.child_by_field_name("key") {
                            let code = &code[name.start_byte()..name.end_byte()];
                            return std::str::from_utf8(code).ok();
                        }
                    }
                    Typescript::VariableDeclarator => {
                        if let Some(name) = parent.child_by_field_name("name") {
                            let code = &code[name.start_byte()..name.end_byte()];
                            return std::str::from_utf8(code).ok();
                        }
                    }
                    _ => {}
                }
            }
            Some("<anonymous>")
        }
    }

    fn get_op_type(node: &Node) -> HalsteadType {
        use Typescript::*;

        match node.kind_id().into() {
            Export | Import | Import2 | Extends | DOT | From | LPAREN | COMMA | As | STAR
            | GTGT | GTGTGT | COLON | Return | Delete | Throw | Break | Continue | If | Else
            | Switch | Case | Default | Async | For | In | Of | While | Try | Catch | Finally
            | With | EQ | AT | AMPAMP | PIPEPIPE | PLUS | DASH | DASHDASH | PLUSPLUS | SLASH
            | PERCENT | STARSTAR | PIPE | AMP | LTLT | TILDE | LT | LTEQ | EQEQ | BANGEQ | GTEQ
            | GT | PLUSEQ | BANG | BANGEQEQ | EQEQEQ | DASHEQ | STAREQ | SLASHEQ | PERCENTEQ
            | STARSTAREQ | GTGTEQ | GTGTGTEQ | LTLTEQ | AMPEQ | CARET | CARETEQ | PIPEEQ
            | Yield | LBRACK | LBRACE | Await | QMARK | QMARKQMARK | New | Let | Var | Const
            | Function | FunctionExpression | SEMI => HalsteadType::Operator,
            Identifier | NestedIdentifier | MemberExpression | PropertyIdentifier | String
            | Number | True | False | Null | Void | This | Super | Undefined | Set | Get
            | Typeof | Instanceof => HalsteadType::Operand,
            _ => HalsteadType::Unknown,
        }
    }
}

impl Getter for TsxCode {
    fn get_space_kind(node: &Node) -> SpaceKind {
        use Tsx::*;

        match node.kind_id().into() {
            FunctionExpression
            | MethodDefinition
            | GeneratorFunction
            | FunctionDeclaration
            | GeneratorFunctionDeclaration
            | ArrowFunction => SpaceKind::Function,
            Class | ClassDeclaration => SpaceKind::Class,
            InterfaceDeclaration => SpaceKind::Interface,
            Program => SpaceKind::Unit,
            _ => SpaceKind::Unknown,
        }
    }

    fn get_func_space_name<'a>(node: &Node, code: &'a [u8]) -> Option<&'a str> {
        if let Some(name) = node.child_by_field_name("name") {
            let code = &code[name.start_byte()..name.end_byte()];
            std::str::from_utf8(code).ok()
        } else {
            // We can be in a pair: foo: function() {}
            // Or in a variable declaration: var aFun = function() {}
            if let Some(parent) = node.parent() {
                match parent.kind_id().into() {
                    Tsx::Pair => {
                        if let Some(name) = parent.child_by_field_name("key") {
                            let code = &code[name.start_byte()..name.end_byte()];
                            return std::str::from_utf8(code).ok();
                        }
                    }
                    Tsx::VariableDeclarator => {
                        if let Some(name) = parent.child_by_field_name("name") {
                            let code = &code[name.start_byte()..name.end_byte()];
                            return std::str::from_utf8(code).ok();
                        }
                    }
                    _ => {}
                }
            }
            Some("<anonymous>")
        }
    }

    fn get_op_type(node: &Node) -> HalsteadType {
        use Tsx::*;

        match node.kind_id().into() {
            Export | Import | Import2 | Extends | DOT | From | LPAREN | COMMA | As | STAR
            | GTGT | GTGTGT | COLON | Return | Delete | Throw | Break | Continue | If | Else
            | Switch | Case | Default | Async | For | In | Of | While | Try | Catch | Finally
            | With | EQ | AT | AMPAMP | PIPEPIPE | PLUS | DASH | DASHDASH | PLUSPLUS | SLASH
            | PERCENT | STARSTAR | PIPE | AMP | LTLT | TILDE | LT | LTEQ | EQEQ | BANGEQ | GTEQ
            | GT | PLUSEQ | BANG | BANGEQEQ | EQEQEQ | DASHEQ | STAREQ | SLASHEQ | PERCENTEQ
            | STARSTAREQ | GTGTEQ | GTGTGTEQ | LTLTEQ | AMPEQ | CARET | CARETEQ | PIPEEQ
            | Yield | LBRACK | LBRACE | Await | QMARK | QMARKQMARK | New | Let | Var | Const
            | Function | FunctionExpression | SEMI => HalsteadType::Operator,
            Identifier | NestedIdentifier | MemberExpression | PropertyIdentifier | String
            | String2 | Number | True | False | Null | Void | This | Super | Undefined | Set
            | Get | Typeof | Instanceof => HalsteadType::Operand,
            _ => HalsteadType::Unknown,
        }
    }
}

impl Getter for RustCode {
    fn get_func_space_name<'a>(node: &Node, code: &'a [u8]) -> Option<&'a str> {
        // we're in a function or in a class or an impl
        // for an impl: we've  'impl ... type {...'
        if let Some(name) = node
            .child_by_field_name("name")
            .or_else(|| node.child_by_field_name("type"))
        {
            let code = &code[name.start_byte()..name.end_byte()];
            std::str::from_utf8(code).ok()
        } else {
            Some("<anonymous>")
        }
    }

    fn get_space_kind(node: &Node) -> SpaceKind {
        use Rust::*;

        match node.kind_id().into() {
            FunctionItem | ClosureExpression => SpaceKind::Function,
            TraitItem => SpaceKind::Trait,
            ImplItem => SpaceKind::Impl,
            SourceFile => SpaceKind::Unit,
            _ => SpaceKind::Unknown,
        }
    }

    fn get_op_type(node: &Node) -> HalsteadType {
        use Rust::*;

        match node.kind_id().into() {
            // `||` is treated as an operator only if it's part of a binary expression.
            // This prevents misclassification inside macros where closures without arguments (e.g., `let closure = || { /* ... */ };`)
            // are not recognized as `ClosureExpression` and their `||` node is identified as `PIPEPIPE` instead of `ClosureParameters`.
            //
            // Similarly, exclude `/` when it corresponds to the third slash in `///` (`OuterDocCommentMarker`)
            PIPEPIPE | SLASH => match node.parent() {
                Some(parent) if matches!(parent.kind_id().into(), BinaryExpression) => {
                    HalsteadType::Operator
                }
                _ => HalsteadType::Unknown,
            },
            // Ensure `!` is counted as an operator unless it belongs to an `InnerDocCommentMarker` `//!`
            BANG => match node.parent() {
                Some(parent) if !matches!(parent.kind_id().into(), InnerDocCommentMarker) => {
                    HalsteadType::Operator
                }
                _ => HalsteadType::Unknown,
            },
            LPAREN | LBRACE | LBRACK | EQGT | PLUS | STAR | Async | Await | Continue | For | If
            | Let | Loop | Match | Return | Unsafe | While | EQ | COMMA | DASHGT | QMARK | LT
            | GT | AMP | MutableSpecifier | DOTDOT | DOTDOTEQ | DASH | AMPAMP | PIPE | CARET
            | EQEQ | BANGEQ | LTEQ | GTEQ | LTLT | GTGT | PERCENT | PLUSEQ | DASHEQ | STAREQ
            | SLASHEQ | PERCENTEQ | AMPEQ | PIPEEQ | CARETEQ | LTLTEQ | GTGTEQ | Move | DOT
            | PrimitiveType | Fn | SEMI => HalsteadType::Operator,
            Identifier | StringLiteral | RawStringLiteral | IntegerLiteral | FloatLiteral
            | BooleanLiteral | Zelf | CharLiteral | UNDERSCORE => HalsteadType::Operand,
            _ => HalsteadType::Unknown,
        }
    }
}

impl Getter for GoCode {
    fn get_space_kind(node: &Node) -> SpaceKind {
        use crate::languages::Go::*;
        match node.kind_id().into() {
            FunctionDeclaration | MethodDeclaration | FuncLiteral => SpaceKind::Function,
            SourceFile => SpaceKind::Unit,
            _ => SpaceKind::Unknown,
        }
    }

    fn get_op_type(node: &Node) -> HalsteadType {
        use crate::languages::Go::*;
        match node.kind_id().into() {
            // Operators: keywords and control flow
            // Note: Go::Go is the `go` keyword for goroutines
            Func | Go | Defer | Return | If | Else | For | Range | Switch | Select
            | Case | Default | Break | Continue | Goto | Fallthrough | Chan | Map | Struct
            | Interface | Type | Var | Const | Package | Import
            // Operators: punctuation
            | DOT | COMMA | SEMI | COLON | COLONEQ | EQ
            | PLUSEQ | DASHEQ | STAREQ | SLASHEQ | PERCENTEQ
            | AMPEQ | PIPEEQ | CARETEQ | LTLTEQ | GTGTEQ | AMPCARETEQ
            // Operators: arithmetic/logic
            | PLUS | DASH | STAR | SLASH | PERCENT | AMP | PIPE | CARET | LTLT | GTGT
            | AMPAMP | PIPEPIPE | AMPCARET | PLUSPLUS | DASHDASH | LTDASH | TILDE
            | EQEQ | BANGEQ | LT | LTEQ | GT | GTEQ | BANG
            | LPAREN | LBRACK | LBRACE | DOTDOTDOT => HalsteadType::Operator,
            // Operands
            Identifier | Identifier2 | Identifier3 | BlankIdentifier | FieldIdentifier
            | LabelName | PackageIdentifier | TypeIdentifier | IntLiteral | FloatLiteral
            | ImaginaryLiteral | RuneLiteral | RawStringLiteral | InterpretedStringLiteral | True
            | False | Nil | Iota => HalsteadType::Operand,
            _ => HalsteadType::Unknown,
        }
    }
}

impl Getter for KotlinCode {
    fn get_space_kind(node: &Node) -> SpaceKind {
        use Kotlin::*;
        match node.kind_id().into() {
            FunctionDeclaration | AnonymousFunction | LambdaLiteral | SecondaryConstructor
            | Getter | Setter => SpaceKind::Function,
            // tree-sitter-kotlin uses a single `class_declaration` node for
            // both `class` and `interface`; the only distinguishing signal
            // is the leading keyword child. Route interfaces to
            // `SpaceKind::Interface` so class-vs-interface metrics (WMC,
            // NPM, NPA) aggregate correctly at the enclosing space.
            ClassDeclaration => {
                if node.children().any(|c| c.kind_id() == Kotlin::Interface) {
                    SpaceKind::Interface
                } else {
                    SpaceKind::Class
                }
            }
            ObjectDeclaration | CompanionObject => SpaceKind::Class,
            SourceFile => SpaceKind::Unit,
            _ => SpaceKind::Unknown,
        }
    }

    fn get_func_space_name<'a>(node: &Node, code: &'a [u8]) -> Option<&'a str> {
        if let Some(name) = node.child_by_field_name("name") {
            let bytes = &code[name.start_byte()..name.end_byte()];
            return std::str::from_utf8(bytes).ok();
        }
        // Kotlin class/interface/object/fun declarations tag the name as a
        // plain child (simple_identifier/type_identifier) rather than via a
        // `name` field.
        for child in node.children() {
            if matches!(
                child.kind_id().into(),
                Kotlin::SimpleIdentifier | Kotlin::TypeIdentifier | Kotlin::Identifier
            ) {
                let bytes = &code[child.start_byte()..child.end_byte()];
                return std::str::from_utf8(bytes).ok();
            }
        }
        Some("<anonymous>")
    }

    fn get_op_type(node: &Node) -> HalsteadType {
        use Kotlin::*;

        match node.kind_id().into() {
            // Keywords and control flow.
            Fun | Val | Var | Class | Interface | Object | Enum | Data | Sealed | Open
            | Abstract | Final | Override | Private | Public | Protected | Internal | Inner
            | Companion | Init | Constructor | Typealias | Import | Package | If | Else | When
            | Try | Catch | Finally | Throw | Return | Continue | Break | For | While | Do
            | In | Is | As | AsQMARK | By | Where | Suspend | Inline | Infix | Operator
            | Tailrec | External | Lateinit | Noinline | Crossinline | Vararg | Out | Get | Set
            // Assignment / augmented assignment.
            | EQ | PLUSEQ | DASHEQ | STAREQ | SLASHEQ | PERCENTEQ
            // Comparison / arithmetic / logical operators.
            | PLUS | DASH | STAR | SLASH | PERCENT
            | AMPAMP | PIPEPIPE | BANG | BANGBANG
            | LT | GT | LTEQ | GTEQ | EQEQ | BANGEQ | EQEQEQ | BANGEQEQ
            | BANGin | BANGis
            | QMARKCOLON | QMARKDOT
            // Structural punctuation.
            | LPAREN | LBRACE | LBRACK
            | DOT | COMMA | SEMI | COLON | COLONCOLON
            | DASHGT | DOTDOT
            | PLUSPLUS | DASHDASH => HalsteadType::Operator,

            // Operands: identifiers, literals, this/super, null.
            SimpleIdentifier | Identifier | TypeIdentifier | IntegerLiteral | HexLiteral
            | BinLiteral | LongLiteral | RealLiteral | UnsignedLiteral | CharacterLiteral
            | StringLiteral | True | False | BooleanLiteral | NullLiteral | This
            | ThisExpression | Super | SuperExpression | Field => HalsteadType::Operand,
            _ => HalsteadType::Unknown,
        }
    }
}

impl Getter for RubyCode {
    fn get_space_kind(node: &Node) -> SpaceKind {
        match node.kind_id().into() {
            Ruby::Method | Ruby::SingletonMethod | Ruby::Lambda | Ruby::Block | Ruby::DoBlock => {
                SpaceKind::Function
            }
            Ruby::Class | Ruby::Module | Ruby::SingletonClass => SpaceKind::Class,
            Ruby::Program => SpaceKind::Unit,
            _ => SpaceKind::Unknown,
        }
    }

    fn get_func_space_name<'a>(node: &Node, code: &'a [u8]) -> Option<&'a str> {
        // class / module / method / singleton_method have a `name` field
        if let Some(name) = node.child_by_field_name("name") {
            let code = &code[name.start_byte()..name.end_byte()];
            return std::str::from_utf8(code).ok();
        }
        Some("<anonymous>")
    }

    fn get_op_type(node: &Node) -> HalsteadType {
        use Ruby::*;

        match node.kind_id().into() {
            // Keywords and structural/control-flow operators.
            // The enum generator splits duplicate ts_names into numbered variants
            // (Class2/Module2/If2/...). We include both the named wrappers and
            // the raw keyword tokens so every surface form is classified.
            Def | Class2 | Module2 | If2 | Elsif2 | Else2 | Unless2 | While2 | Until2 | For2
            | In2 | Do2 | Case2 | When2 | Then2 | Begin2 | Ensure2 | Rescue2 | Return3
            | Yield3 | Break3 | Next3 | Redo2 | Retry2 | Alias2 | Undef2 | BEGIN | END
            | And | Or | Not | DefinedQMARK | Super
            // Assignment and arithmetic / comparison / bitwise operators.
            | EQ | EQ2 | PLUSEQ | DASHEQ | STAREQ | STARSTAREQ | SLASHEQ | PERCENTEQ
            | PIPEPIPEEQ | AMPAMPEQ | PIPEEQ | AMPEQ | GTGTEQ | LTLTEQ | CARETEQ
            | PIPEPIPE | AMPAMP | PLUS | DASH | STAR | STARSTAR | SLASH | PERCENT
            | LTLT | GTGT | AMP | PIPE | CARET | TILDE
            | LT | GT | LTEQ | GTEQ | EQEQ | BANGEQ | EQEQEQ | LTEQGT | EQTILDE | BANGTILDE
            | BANG
            // Structural punctuation.
            | LPAREN | LBRACE | LBRACK
            | DOT | AMPDOT | COLONCOLON | COLONCOLON2
            | COMMA | SEMI | QMARK | COLON | COLON2 | EQGT | DASHGT
            | DOTDOT | DOTDOTDOT
            // String/interpolation delimiters act as operators.
            | DQUOTE | HASHLBRACE | BQUOTE | BQUOTE2 => HalsteadType::Operator,

            // Operands: identifiers, literals and keyword literals.
            Identifier | Constant | InstanceVariable | ClassVariable | GlobalVariable
            | Integer | Float | Rational | Complex | Character
            | String | ChainedString | SimpleSymbol | DelimitedSymbol | HeredocBeginning
            | True | False | Nil | Nil2 | Zelf | Line | File | Encoding => HalsteadType::Operand,

            _ => HalsteadType::Unknown,
        }
    }
}

impl Getter for PowershellCode {
    fn get_space_kind(node: &Node) -> SpaceKind {
        use Powershell::*;
        match node.kind_id().into() {
            FunctionStatement | ClassMethodDefinition | ScriptBlockExpression => {
                SpaceKind::Function
            }
            ClassStatement => SpaceKind::Class,
            Program => SpaceKind::Unit,
            _ => SpaceKind::Unknown,
        }
    }

    fn get_func_space_name<'a>(node: &Node, code: &'a [u8]) -> Option<&'a str> {
        // In the tree-sitter-pwsh grammar, neither `function_statement` nor
        // `class_method_definition` tags the name via a `name` field. Walk
        // the children to find the first identifier-shaped child.
        match node.kind_id().into() {
            Powershell::FunctionStatement => {
                for child in node.children() {
                    if child.kind_id() == Powershell::FunctionName {
                        let bytes = &code[child.start_byte()..child.end_byte()];
                        return std::str::from_utf8(bytes).ok();
                    }
                }
                Some("<anonymous>")
            }
            Powershell::ClassMethodDefinition | Powershell::ClassStatement => {
                for child in node.children() {
                    if child.kind_id() == Powershell::SimpleName {
                        let bytes = &code[child.start_byte()..child.end_byte()];
                        return std::str::from_utf8(bytes).ok();
                    }
                }
                Some("<anonymous>")
            }
            _ => Some("<anonymous>"),
        }
    }

    fn get_op_type(node: &Node) -> HalsteadType {
        use Powershell::*;

        match node.kind_id().into() {
            // Keywords and structural/control-flow markers act as operators.
            Function | Filter | Workflow | If | Elseif | Else | Switch | For
            | Foreach | In | While | Do | Until | Break | Continue | Return | Throw | Exit
            | Try | Catch | Finally | Trap | Param | Using | Namespace | Module | Assembly
            | Static | This | Base | Begin | Process | End2 | Clean | Dynamicparam | Data
            | Inlinescript | Parallel | Sequence
            // Punctuation-like operators.
            | LPAREN | LBRACE | LBRACK | COMMA | SEMI | DOT | DOT2 | COLON | COLONCOLON
            | ATLPAREN | ATLBRACE | DOLLARLPAREN
            // Assignment family.
            | EQ | PLUSEQ | DASHEQ | STAREQ | SLASHEQ | PERCENTEQ | QMARKQMARKEQ
            // Arithmetic / bitwise / unary.
            | PLUS | DASH | STAR | SLASH | PERCENT | BSLASH | DOTDOT
            | PLUSPLUS | DASHDASH | BANG
            // Short-circuit / null-coalesce / ternary.
            | AMPAMP | PIPEPIPE | QMARK | QMARKQMARK
            // Pipeline / invocation / redirection tokens.
            | PIPE | AMP
            // PowerShell's word-form logical / comparison / typing operators
            // (dash-prefixed). The grammar exposes each as its own anonymous
            // token; we classify them all as operators.
            | DASHand | DASHor | DASHxor | DASHnot | DASHband | DASHbor | DASHbxor | DASHbnot
            | DASHas | DASHis | DASHisnot | DASHf | DASHjoin
            | DASHshl | DASHshr | DASHsplit | DASHisplit | DASHcsplit
            | DASHreplace | DASHireplace | DASHcreplace
            | DASHmatch | DASHimatch | DASHcmatch | DASHnotmatch | DASHinotmatch | DASHcnotmatch
            | DASHlike | DASHilike | DASHclike | DASHnotlike | DASHinotlike | DASHcnotlike
            | DASHcontains | DASHicontains | DASHccontains
            | DASHnotcontains | DASHinotcontains | DASHcnotcontains
            | DASHin | DASHnotin
            | DASHeq | DASHieq | DASHceq | DASHne | DASHine | DASHcne
            | DASHlt | DASHilt | DASHclt | DASHle | DASHile | DASHcle
            | DASHgt | DASHigt | DASHcgt | DASHge | DASHige | DASHcge
            | LT | GT
            // File / merging redirection leaf tokens (e.g. `2>`, `2>>`,
            // `2>&1`, `*>`, `3>&2`). The grammar wraps each under a
            // `file_redirection_operator` / `merging_redirection_operator`
            // rule node, but we must classify the *leaf tokens* here —
            // the wrapper rule kinds are intentionally excluded below to
            // avoid double-counting (see comment at the bottom of the
            // match).
            | GTGT | STARGT | STARGTGT | STARGTAMP1 | STARGTAMP2
            | N2GT | N2GTGT | N2GTAMP1
            | N3GT | N3GTGT | N3GTAMP1 | N3GTAMP2
            | N4GT | N4GTGT | N4GTAMP1 | N4GTAMP2
            | N5GT | N5GTGT | N5GTAMP1 | N5GTAMP2
            | N6GT | N6GTGT | N6GTAMP1 | N6GTAMP2
            | N1GTAMP2 => HalsteadType::Operator,

            // Wrapper rule kinds (`assignment_operator`, `comparison_operator`,
            // `format_operator`, `file_redirection_operator`,
            // `merging_redirection_operator`) are NOT classified here:
            // tree-sitter-pwsh nests the individual operator leaf token
            // inside its matching wrapper rule, so
            // `T::Halstead::compute`, which visits every named and
            // anonymous child exhaustively (see spaces.rs), would
            // otherwise count each operator twice — once for the wrapper
            // and once for the leaf. Classifying only the leaves gives
            // the correct Halstead N1.

            // Operands: identifiers, variables, literals, and the name
            // identifiers that drive function declarations and command
            // invocations.
            //
            // `function_name`, `command_name`, and `path_command_name_token`
            // are the leaf identifier nodes emitted by tree-sitter-pwsh
            // for `function Greet { … }` / `Get-Item /tmp` /
            // `./build.sh arg` respectively. They carry the actual
            // declared-name or invoked-name text, so they are the
            // natural operands for a PowerShell script (normal programs
            // are dominated by command calls; omitting them suppresses
            // Halstead N2 and any downstream metric that depends on it,
            // like volume and MI).
            //
            // The named *wrappers* `command_name_expr` (the choice over
            // `command_name` / `path_command_name` / `_primary_expression`)
            // and `path_command_name` (which holds one or more
            // `path_command_name_token` leaves) are intentionally NOT
            // classified here: tree-sitter-pwsh nests the leaves inside
            // these wrappers, and `T::Halstead::compute` walks every
            // node exhaustively, so matching both wrapper and leaf would
            // double-count. Same rule the operator classification uses
            // for `assignment_operator` / `comparison_operator`.
            SimpleName | TypeIdentifier | Variable | Variable2 | BracedVariable | GenericToken
            | GenericToken2 | GenericToken3 | GenericToken4 | GenericToken5
            | DecimalIntegerLiteral | HexadecimalIntegerLiteral | RealLiteral
            | VerbatimStringCharacters | VerbatimStringCharacters2
            | VerbatimHereStringCharacters
            // Expandable (double-quoted) string literals are wrappers in
            // the grammar: their text content is embedded in the
            // wrapper's byte range rather than in a separate content
            // leaf (unlike `verbatim_string_characters`). Classifying
            // the wrapper itself is therefore the only way to count
            // `""`, `"plain"`, and interpolated `"hello $name"` as
            // operands. The wrapper's interpolation children
            // (`variable`, `sub_expression`) are classified separately
            // as operands / operator territory on their own merits, so
            // the wrapper only contributes one count per literal.
            | ExpandableStringLiteral | ExpandableHereStringLiteral
            | FunctionName | CommandName | PathCommandNameToken
            | CommandParameter => HalsteadType::Operand,

            _ => HalsteadType::Unknown,
        }
    }
}

impl Getter for CCode {
    fn get_space_kind(node: &Node) -> SpaceKind {
        match node.kind_id().into() {
            C::FunctionDefinition | C::FunctionDefinition2 => SpaceKind::Function,
            C::TranslationUnit => SpaceKind::Unit,
            _ => SpaceKind::Unknown,
        }
    }

    fn get_func_space_name<'a>(node: &Node, code: &'a [u8]) -> Option<&'a str> {
        // `function_definition` tags the declarator via a `declarator` field.
        // The declarator is usually a `function_declarator` whose own
        // `declarator` child is the name (a `_field_identifier` /
        // `identifier`). Pointer or parenthesized declarators wrap that
        // identifier, so we walk inward via the underlying tree-sitter node
        // (whose `child_by_field_name` preserves the tree lifetime) until we
        // find the identifier-shaped node.
        let mut cur = node.0.child_by_field_name("declarator");
        while let Some(current) = cur {
            match C::from(current.kind_id()) {
                C::Identifier | C::FieldIdentifier | C::TypeIdentifier => {
                    let bytes = &code[current.start_byte()..current.end_byte()];
                    return std::str::from_utf8(bytes).ok();
                }
                _ => {
                    cur = current.child_by_field_name("declarator");
                }
            }
        }
        Some("<anonymous>")
    }

    fn get_op_type(node: &Node) -> HalsteadType {
        use C::*;
        match node.kind_id().into() {
            // Keywords and control flow.
            If | Else | Switch | Case | Default | While | Do | For | Return | Break | Continue
            | Goto | Sizeof | Alignof | Alignof2 | Alignof3 | Alignof4 | Alignof5 | Offsetof
            | Typedef | Extern | Static | Auto | Register | Inline | Inline2 | Inline3
            | Forceinline | ThreadLocal | Thread | Const | Constexpr | Volatile | Volatile2
            | Restrict | Restrict2 | Atomic | Noreturn | Noreturn2 | Nonnull | Alignas
            | Alignas2 | Signed | Unsigned | Long | Short | Enum | Struct | Union
            // Punctuation.
            | LPAREN | LPAREN2 | RPAREN | LBRACE | RBRACE | LBRACK | RBRACK
            | COMMA | SEMI | COLON | QMARK | DOT | DASHGT
            // Arithmetic / bitwise / logical / comparison operators.
            | PLUS | DASH | STAR | SLASH | PERCENT | AMP | PIPE | CARET | TILDE | BANG
            | LTLT | GTGT | AMPAMP | PIPEPIPE
            | EQ | EQEQ | BANGEQ | LT | LTEQ | GT | GTEQ
            | PLUSEQ | DASHEQ | STAREQ | SLASHEQ | PERCENTEQ
            | AMPEQ | PIPEEQ | CARETEQ | LTLTEQ | GTGTEQ
            | PLUSPLUS | DASHDASH
            // Preprocessor directives count as operators.
            | HASHinclude | HASHdefine | HASHif | HASHifdef | HASHifndef
            | HASHelse | HASHelif | HASHelifdef | HASHelifndef | HASHendif => HalsteadType::Operator,

            // Operands: identifiers, type identifiers, literals.
            Identifier | FieldIdentifier | TypeIdentifier | StatementIdentifier
            | PrimitiveType | NumberLiteral | CharLiteral | StringLiteral | ConcatenatedString
            | True | False | NULL | Nullptr | SystemLibString => HalsteadType::Operand,

            _ => HalsteadType::Unknown,
        }
    }
}

impl Getter for PhpCode {
    fn get_space_kind(node: &Node) -> SpaceKind {
        use Php::*;
        match node.kind_id().into() {
            FunctionDefinition | MethodDeclaration | AnonymousFunction | ArrowFunction => {
                SpaceKind::Function
            }
            ClassDeclaration | AnonymousClass | TraitDeclaration | EnumDeclaration => {
                SpaceKind::Class
            }
            InterfaceDeclaration => SpaceKind::Interface,
            Program => SpaceKind::Unit,
            _ => SpaceKind::Unknown,
        }
    }

    fn get_func_space_name<'a>(node: &Node, code: &'a [u8]) -> Option<&'a str> {
        if let Some(name) = node.child_by_field_name("name") {
            let bytes = &code[name.start_byte()..name.end_byte()];
            return std::str::from_utf8(bytes).ok();
        }
        Some("<anonymous>")
    }

    fn get_op_type(node: &Node) -> HalsteadType {
        use Php::*;

        match node.kind_id().into() {
            // Keywords and structural / control-flow operators.
            Function | Fn | Class | Interface | Trait | Enum | Namespace | Use | As
            | Insteadof | Const | Static | Public | Protected | Private | Final | Abstract
            | Readonly | VarModifier | Extends | Implements | New | Clone | Instanceof
            | If | Else | Elseif | Endif | Switch | Case | Default | Endswitch | Match
            | While | Endwhile | Do | For | Endfor | Foreach | Endforeach | Continue
            | Break | Return | Throw | Try | Catch | Finally | Goto | Yield | Yieldfrom
            | Echo | Print | Exit | Unset | Declare | Enddeclare | Global | List | Array
            | Include | IncludeOnce | Require | RequireOnce
            | And | Or | Xor
            // Punctuation-like operators.
            | LPAREN | LPAREN2 | LBRACE | LBRACK | COMMA | SEMI | COLON | COLONCOLON
            | DOT | DASHGT | QMARKDASHGT | EQGT | DOTDOTDOT | HASHLBRACK | AT | BSLASH
            // Assignment family.
            | EQ | PLUSEQ | DASHEQ | STAREQ | STARSTAREQ | SLASHEQ | PERCENTEQ | DOTEQ
            | LTLTEQ | GTGTEQ | AMPEQ | CARETEQ | PIPEEQ | QMARKQMARKEQ
            // Arithmetic / bitwise / unary / ternary / null-coalesce.
            | PLUS | DASH | STAR | STARSTAR | SLASH | PERCENT
            | PLUSPLUS | DASHDASH | TILDE | BANG
            | AMPAMP | PIPEPIPE | QMARKQMARK | QMARK
            | AMP | PIPE | CARET | LTLT | GTGT
            // Comparison.
            | EQEQ | EQEQEQ | BANGEQ | BANGEQEQ | LTGT
            | LT | GT | LTEQ | GTEQ | LTEQGT
            // Pipe operator (PHP 8.5).
            | PIPEGT
            // String delimiters when emitted as anonymous tokens act as
            // operators; the wrapper string nodes are the operands.
            | DQUOTE | DQUOTE2 | SQUOTE | SQUOTE2 | BQUOTE | LTLTLT
            | DOLLAR => HalsteadType::Operator,

            // Operands: identifiers, variables, literals, names.
            Name | Name2 | NamespaceName | QualifiedName | RelativeName
            | VariableName | DynamicVariableName
            | Integer | Float | Boolean | Null
            | String | EncapsedString | Heredoc | Nowdoc
            | Zelf | Parent => HalsteadType::Operand,

            _ => HalsteadType::Unknown,
        }
    }
}

#[cfg(feature = "markdown")]
impl Getter for MarkdownCode {
    // Markdown uses the dedicated pipeline in `src/markdown/`; here we rely on
    // the trait's default `Getter` impls so the parser still builds a single
    // top-level unit space with no functions or classes.
}

#[cfg(test)]
mod tests {
    use crate::node::Tree;
    use crate::traits::Search;

    use super::*;

    #[test]
    fn kotlin_accessor_tokens_are_classified_for_halstead() {
        let tree = Tree::new::<KotlinCode>(
            b"class C {
                @field:JvmField
                var x: Int = 0
                    get() = field
                    set(value) { field = value }
            }",
        );
        let mut saw_get = false;
        let mut saw_set = false;
        let mut saw_field = false;

        tree.get_root()
            .act_on_node(&mut |node| match node.kind_id().into() {
                Kotlin::Get => {
                    saw_get = true;
                    assert!(matches!(
                        KotlinCode::get_op_type(node),
                        HalsteadType::Operator
                    ));
                }
                Kotlin::Set => {
                    saw_set = true;
                    assert!(matches!(
                        KotlinCode::get_op_type(node),
                        HalsteadType::Operator
                    ));
                }
                Kotlin::Field => {
                    saw_field = true;
                    assert!(matches!(
                        KotlinCode::get_op_type(node),
                        HalsteadType::Operand
                    ));
                }
                _ => {}
            });

        assert!(saw_get);
        assert!(saw_set);
        assert!(saw_field);
    }
}
