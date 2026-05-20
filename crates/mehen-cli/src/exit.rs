//! Exit code contract for the 1.0 CLI (rewrite plan §4.1).

#[derive(Debug, Clone, Copy)]
pub enum ExitCode {
    Success = 0,
    /// Setup, IO, git, parser fatal, unsupported-language, or invalid-state
    /// error. Also covers "analysis errors" diagnostics on `mehen metrics`.
    SetupError = 1,
    /// Threshold or policy failure. Reserved for `mehen diff` and
    /// `mehen top-offenders`.
    ThresholdFailure = 2,
    /// Invalid machine-output serialization state.
    SerializationError = 3,
}

impl From<ExitCode> for i32 {
    fn from(value: ExitCode) -> Self {
        value as i32
    }
}
