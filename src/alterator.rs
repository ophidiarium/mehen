use crate::checker::Checker;
use crate::langs::{GoCode, PythonCode, RustCode, TsxCode, TypescriptCode};

/// Marker trait for language implementations used by `Parser`.
pub(crate) trait Alterator: Checker {}

impl Alterator for PythonCode {}
impl Alterator for GoCode {}
impl Alterator for TypescriptCode {}
impl Alterator for TsxCode {}
impl Alterator for RustCode {}
