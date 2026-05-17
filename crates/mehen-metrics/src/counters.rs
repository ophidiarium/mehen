use serde::Serialize;

/// Number of arguments accumulator (NArgs).
#[derive(Default, Clone, Debug, PartialEq, Serialize)]
pub struct NargsStats {
    pub functions: u32,
    pub closures: u32,
    pub min: u32,
    pub max: u32,
}

impl NargsStats {
    pub fn record_function_args(&mut self, count: u32) {
        self.functions = self.functions.saturating_add(count);
    }

    pub fn record_closure_args(&mut self, count: u32) {
        self.closures = self.closures.saturating_add(count);
    }

    pub fn merge(&mut self, other: &NargsStats) {
        self.functions = self.functions.saturating_add(other.functions);
        self.closures = self.closures.saturating_add(other.closures);
    }
}

/// Number of methods/functions (NOM) accumulator.
#[derive(Default, Clone, Debug, PartialEq, Serialize)]
pub struct NomStats {
    pub functions: u32,
    pub closures: u32,
    pub min: u32,
    pub max: u32,
}

impl NomStats {
    pub fn record_function(&mut self) {
        self.functions = self.functions.saturating_add(1);
    }

    pub fn record_closure(&mut self) {
        self.closures = self.closures.saturating_add(1);
    }

    pub fn merge(&mut self, other: &NomStats) {
        self.functions = self.functions.saturating_add(other.functions);
        self.closures = self.closures.saturating_add(other.closures);
    }

    pub fn total(&self) -> u32 {
        self.functions.saturating_add(self.closures)
    }
}

/// Number of exits (return/throw/raise/break/continue) accumulator.
#[derive(Default, Clone, Debug, PartialEq, Serialize)]
pub struct NexitStats {
    pub exits: u32,
    pub min: u32,
    pub max: u32,
    pub average: f64,
    pub sum: u32,
}

impl NexitStats {
    pub fn record_exit(&mut self) {
        self.exits = self.exits.saturating_add(1);
    }

    pub fn merge(&mut self, other: &NexitStats) {
        self.sum = self.sum.saturating_add(other.sum);
    }
}

/// Number of public attributes accumulator (NPA).
#[derive(Default, Clone, Debug, PartialEq, Serialize)]
pub struct NpaStats {
    pub public: u32,
    pub total: u32,
}

impl NpaStats {
    pub fn record_attribute(&mut self, public: bool) {
        self.total = self.total.saturating_add(1);
        if public {
            self.public = self.public.saturating_add(1);
        }
    }

    pub fn merge(&mut self, other: &NpaStats) {
        self.public = self.public.saturating_add(other.public);
        self.total = self.total.saturating_add(other.total);
    }
}

/// Number of public methods accumulator (NPM).
#[derive(Default, Clone, Debug, PartialEq, Serialize)]
pub struct NpmStats {
    pub public: u32,
    pub total: u32,
}

impl NpmStats {
    pub fn record_method(&mut self, public: bool) {
        self.total = self.total.saturating_add(1);
        if public {
            self.public = self.public.saturating_add(1);
        }
    }

    pub fn merge(&mut self, other: &NpmStats) {
        self.public = self.public.saturating_add(other.public);
        self.total = self.total.saturating_add(other.total);
    }
}

/// Weighted methods per class accumulator (WMC).
///
/// WMC sums the cyclomatic complexity of every method on a class. Stored
/// as a u32 so language crates can fold per-method cyclomatic into one
/// per-class counter without re-walking the tree.
#[derive(Default, Clone, Debug, PartialEq, Serialize)]
pub struct WmcStats {
    pub wmc: u32,
}

impl WmcStats {
    pub fn record_method_cyclomatic(&mut self, cyclomatic: u32) {
        self.wmc = self.wmc.saturating_add(cyclomatic);
    }

    pub fn merge(&mut self, other: &WmcStats) {
        self.wmc = self.wmc.saturating_add(other.wmc);
    }
}
