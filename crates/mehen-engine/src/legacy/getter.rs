use crate::legacy::langs::CCode;
#[cfg(feature = "markdown")]
use crate::legacy::langs::MarkdownCode;
use crate::legacy::languages::C;
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
