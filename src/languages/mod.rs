#![allow(clippy::enum_variant_names)]

pub(crate) mod language_python;
pub(crate) use language_python::*;

pub(crate) mod language_rust;
pub(crate) use language_rust::*;

pub(crate) mod language_tsx;
pub(crate) use language_tsx::*;

pub(crate) mod language_typescript;
pub(crate) use language_typescript::*;

pub(crate) mod language_go;
pub(crate) use language_go::*;
