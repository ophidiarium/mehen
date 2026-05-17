#![allow(clippy::enum_variant_names)]

pub mod language_python;
pub use language_python::*;

pub mod language_rust;
pub use language_rust::*;

pub mod language_tsx;
pub use language_tsx::*;

pub mod language_typescript;
pub use language_typescript::*;

pub mod language_go;
pub use language_go::*;

pub mod language_ruby;
pub use language_ruby::Ruby;

pub mod language_kotlin;
pub use language_kotlin::*;

pub mod language_powershell;
pub use language_powershell::*;

pub mod language_c;
pub use language_c::*;

pub mod language_php;
pub use language_php::*;

#[cfg(feature = "markdown")]
pub mod language_markdown;
#[cfg(feature = "markdown")]
pub use language_markdown::Markdown;
