use std::path::Path;
use std::sync::Arc;

use crate::legacy::alterator::Alterator;
use crate::legacy::checker::Checker;
use crate::legacy::getter::Getter;
use crate::legacy::langs::*;
use crate::legacy::metrics::abc::Abc;
use crate::legacy::metrics::cognitive::Cognitive;
use crate::legacy::metrics::cyclomatic::Cyclomatic;
use crate::legacy::metrics::exit::Exit;
use crate::legacy::metrics::halstead::Halstead;
use crate::legacy::metrics::loc::Loc;
use crate::legacy::metrics::mi::Mi;
use crate::legacy::metrics::nargs::NArgs;
use crate::legacy::metrics::nom::Nom;
use crate::legacy::metrics::npa::Npa;
use crate::legacy::metrics::npm::Npm;
use crate::legacy::metrics::wmc::Wmc;
use crate::legacy::node::Node;
use crate::legacy::preproc::PreprocResults;

/// A trait for callback functions used by the `mk_action!` macro.
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
    fn get_lang() -> LANG;
}

pub(crate) trait Search<'a> {
    /// Walk every node under `self`, depth-first, and invoke `pred` on
    /// each. Only consumed by per-language tests in `getter.rs` — clippy
    /// flags it as dead in non-test builds, hence the `cfg(test)` gate.
    #[cfg(test)]
    fn act_on_node(&self, pred: &mut dyn FnMut(&Node<'a>));
    fn act_on_child(&self, action: &mut dyn FnMut(&Node<'a>));
}
