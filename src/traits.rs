use std::path::Path;
use std::sync::Arc;

use crate::alterator::Alterator;
use crate::checker::Checker;
use crate::getter::Getter;
use crate::langs::*;
use crate::metrics::abc::Abc;
use crate::metrics::cognitive::Cognitive;
use crate::metrics::cyclomatic::Cyclomatic;
use crate::metrics::exit::Exit;
use crate::metrics::halstead::Halstead;
use crate::metrics::loc::Loc;
use crate::metrics::mi::Mi;
use crate::metrics::nargs::NArgs;
use crate::metrics::nom::Nom;
use crate::metrics::npa::Npa;
use crate::metrics::npm::Npm;
use crate::metrics::wmc::Wmc;
use crate::node::Node;
use crate::parser::Filter;
use crate::preproc::PreprocResults;

/// A trait for callback functions.
///
/// Allows to call a private library function, getting as result
/// its output value.
pub(crate) trait Callback {
    /// The output type returned by the callee
    type Res;
    /// The input type used by the caller to pass the arguments to the callee
    type Cfg;

    /// Calls a function inside the library and returns its value
    fn call<T: ParserTrait>(cfg: Self::Cfg, parser: &T) -> Self::Res;
}

pub(crate) trait LanguageInfo {
    type BaseLang;

    fn get_lang() -> LANG;
}

#[doc(hidden)]
pub(crate) trait ParserTrait {
    type Checker: Alterator + Checker;
    type Getter: Getter;
    type Cognitive: Cognitive;
    type Cyclomatic: Cyclomatic;
    type Halstead: Halstead;
    type Loc: Loc;
    type Nom: Nom;
    type Mi: Mi;
    type NArgs: NArgs;
    type Exit: Exit;
    type Wmc: Wmc;
    type Abc: Abc;
    type Npm: Npm;
    type Npa: Npa;

    fn new(code: Vec<u8>, path: &Path, pr: Option<Arc<PreprocResults>>) -> Self;
    fn get_root(&self) -> Node<'_>;
    fn get_code(&self) -> &[u8];
    fn get_filters(&self, filters: &[String]) -> Filter;
}

pub(crate) trait Search<'a> {
    fn act_on_node(&self, pred: &mut dyn FnMut(&Node<'a>));
    fn act_on_child(&self, action: &mut dyn FnMut(&Node<'a>));
}
