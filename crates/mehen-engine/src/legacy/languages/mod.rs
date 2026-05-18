#![allow(clippy::enum_variant_names)]

pub(crate) mod language_go;
pub(crate) use language_go::*;

pub(crate) mod language_ruby;
pub(crate) use language_ruby::Ruby;

pub(crate) mod language_kotlin;
pub(crate) use language_kotlin::*;

pub(crate) mod language_c;
pub(crate) use language_c::*;

pub(crate) mod language_php;
pub(crate) use language_php::*;

#[cfg(feature = "markdown")]
pub(crate) mod language_markdown;
#[cfg(feature = "markdown")]
pub(crate) use language_markdown::Markdown;
