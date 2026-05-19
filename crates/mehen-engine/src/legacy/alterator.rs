use crate::legacy::checker::Checker;
use crate::legacy::langs::CCode;
#[cfg(feature = "markdown")]
use crate::legacy::langs::MarkdownCode;

/// Marker trait for language implementations used by `Parser`.
pub(crate) trait Alterator: Checker {}

impl Alterator for CCode {}
#[cfg(feature = "markdown")]
impl Alterator for MarkdownCode {}
