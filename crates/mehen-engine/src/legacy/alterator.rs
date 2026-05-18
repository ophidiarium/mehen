use crate::legacy::checker::Checker;
#[cfg(feature = "markdown")]
use crate::legacy::langs::MarkdownCode;
use crate::legacy::langs::{CCode, GoCode, KotlinCode, PhpCode, RubyCode};

/// Marker trait for language implementations used by `Parser`.
pub(crate) trait Alterator: Checker {}

impl Alterator for GoCode {}
impl Alterator for RubyCode {}
impl Alterator for KotlinCode {}
impl Alterator for CCode {}
impl Alterator for PhpCode {}
#[cfg(feature = "markdown")]
impl Alterator for MarkdownCode {}
