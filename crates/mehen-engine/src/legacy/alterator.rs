use crate::legacy::checker::Checker;
#[cfg(feature = "markdown")]
use crate::legacy::langs::MarkdownCode;
use crate::legacy::langs::{
    CCode, GoCode, KotlinCode, PhpCode, PowershellCode, PythonCode, RubyCode, RustCode, TsxCode,
    TypescriptCode,
};

/// Marker trait for language implementations used by `Parser`.
pub trait Alterator: Checker {}

impl Alterator for PythonCode {}
impl Alterator for GoCode {}
impl Alterator for TypescriptCode {}
impl Alterator for TsxCode {}
impl Alterator for RustCode {}
impl Alterator for RubyCode {}
impl Alterator for KotlinCode {}
impl Alterator for PowershellCode {}
impl Alterator for CCode {}
impl Alterator for PhpCode {}
#[cfg(feature = "markdown")]
impl Alterator for MarkdownCode {}
