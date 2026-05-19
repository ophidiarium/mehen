#[cfg(feature = "markdown")]
use crate::legacy::langs::MarkdownCode;
use crate::legacy::langs::{CCode, KotlinCode};
use crate::legacy::languages::{C, Kotlin};
use crate::legacy::metrics::halstead::HalsteadType;
use crate::legacy::node::Node;
use crate::legacy::spaces::SpaceKind;

pub(crate) trait Getter {
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

#[cfg(feature = "markdown")]
impl Getter for MarkdownCode {
    // Markdown uses the dedicated pipeline in `src/markdown/`; here we rely on
    // the trait's default `Getter` impls so the parser still builds a single
    // top-level unit space with no functions or classes.
}

#[cfg(test)]
mod tests {
    use crate::legacy::node::Tree;
    use crate::legacy::traits::Search;

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
