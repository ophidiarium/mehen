use crate::langs::{GoCode, KotlinCode, PythonCode, RubyCode, RustCode, TsxCode, TypescriptCode};
use crate::languages::{Kotlin, Python, Ruby, Rust, Tsx, Typescript};
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
            | Tailrec | External | Lateinit | Noinline | Crossinline | Vararg | Out
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
            | ThisExpression | Super | SuperExpression => HalsteadType::Operand,
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
