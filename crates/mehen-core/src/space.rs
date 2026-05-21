// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

use crate::analysis::MetricSet;
use crate::span::SourceSpan;

/// Identifies a `MetricSpace` within one analysis. Stable across one
/// analyzer call; not stable across runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpaceId(pub u32);

/// The kind of metric space.
///
/// `Custom(SmolStr)` keeps the enum open: declarative analyzers can publish
/// scopes such as `cloudformation.resource`, `terraform.module`, or
/// `kubernetes.object` without amending the source-code variants.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpaceKind {
    /// File-level scope. Always exactly one Unit per `LanguageAnalysis`.
    Unit,
    Function,
    Closure,
    Class,
    Interface,
    Trait,
    Impl,
    Enum,
    /// Namespaced custom scope kind for declarative analyzers.
    Custom(SmolStr),
}

impl SpaceKind {
    /// Stable name used for serialization, log lines, and snapshots.
    pub fn as_str(&self) -> &str {
        match self {
            SpaceKind::Unit => "unit",
            SpaceKind::Function => "function",
            SpaceKind::Closure => "closure",
            SpaceKind::Class => "class",
            SpaceKind::Interface => "interface",
            SpaceKind::Trait => "trait",
            SpaceKind::Impl => "impl",
            SpaceKind::Enum => "enum",
            SpaceKind::Custom(s) => s.as_str(),
        }
    }
}

/// One node in the analysis tree.
///
/// `MetricSpace` is owned data — it never borrows from a parser arena. The
/// tree is fully assembled before being handed back from
/// [`crate::LanguageAnalyzer::analyze`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetricSpace {
    pub id: SpaceId,
    pub kind: SpaceKind,
    pub name: Option<String>,
    pub span: SourceSpan,
    pub metrics: MetricSet,
    pub spaces: Vec<MetricSpace>,
}

impl MetricSpace {
    pub fn new(id: SpaceId, kind: SpaceKind, span: SourceSpan) -> Self {
        Self {
            id,
            kind,
            name: None,
            span,
            metrics: MetricSet::default(),
            spaces: Vec::new(),
        }
    }
}
