use mehen_core::{Language, LanguageAnalyzer};

/// Registry that maps a `Language` to its analyzer.
///
/// Per the rewrite plan §4.6, analyzers are constructed per worker (or per
/// analyze call) — they never share parser instances across threads. The
/// registry holds *factory* trait objects so each `analyzer_for` call hands
/// the caller a fresh analyzer struct to drive a single source file.
///
/// In 1.0 the analyzer structs are stateless (Phase 1 tree-sitter
/// placeholders); Phase 7+ may switch them to arena-backed parsers, at
/// which point the same registry shape continues to work because
/// `LanguageAnalysis` is owned and `Send + 'static`.
pub struct AnalyzerRegistry {
    entries: Vec<RegistryEntry>,
}

struct RegistryEntry {
    language: Language,
    factory: AnalyzerFactory,
}

type AnalyzerFactory = Box<dyn Fn() -> Box<dyn LanguageAnalyzer> + Send + Sync>;

#[derive(Debug)]
pub enum RegistryError {
    DuplicateLanguage(Language),
}

impl AnalyzerRegistry {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn register<F>(&mut self, language: Language, factory: F) -> Result<(), RegistryError>
    where
        F: Fn() -> Box<dyn LanguageAnalyzer> + Send + Sync + 'static,
    {
        if self.entries.iter().any(|e| e.language == language) {
            return Err(RegistryError::DuplicateLanguage(language));
        }
        self.entries.push(RegistryEntry {
            language,
            factory: Box::new(factory),
        });
        Ok(())
    }

    /// Returns a freshly-constructed analyzer for `language`, or `None` if
    /// no analyzer is registered (e.g. the owning crate is feature-gated
    /// off in this build).
    pub fn analyzer_for(&self, language: Language) -> Option<Box<dyn LanguageAnalyzer>> {
        self.entries
            .iter()
            .find(|e| e.language == language)
            .map(|e| (e.factory)())
    }

    /// Default registry assembling every analyzer enabled by feature flags.
    pub fn default_set() -> Self {
        let mut registry = Self::new();
        register_default_analyzers(&mut registry);
        registry
    }
}

impl Default for AnalyzerRegistry {
    fn default() -> Self {
        Self::default_set()
    }
}

fn register_default_analyzers(registry: &mut AnalyzerRegistry) {
    #[cfg(feature = "lang-python")]
    {
        let _ = registry.register(Language::Python, || {
            Box::new(mehen_python::PythonAnalyzer::new())
        });
    }
    #[cfg(feature = "lang-typescript")]
    {
        let _ = registry.register(Language::TypeScript, || {
            Box::new(mehen_typescript::TypeScriptAnalyzer::new())
        });
        let _ = registry.register(Language::JavaScript, || {
            Box::new(mehen_typescript::JavaScriptAnalyzer::new())
        });
        let _ = registry.register(Language::Tsx, || {
            Box::new(mehen_typescript::TsxAnalyzer::new())
        });
        let _ = registry.register(Language::Jsx, || {
            Box::new(mehen_typescript::JsxAnalyzer::new())
        });
    }
    #[cfg(feature = "lang-php")]
    {
        let _ = registry.register(Language::Php, || Box::new(mehen_php::PhpAnalyzer::new()));
    }
    #[cfg(feature = "lang-ruby")]
    {
        let _ = registry.register(Language::Ruby, || Box::new(mehen_ruby::RubyAnalyzer::new()));
    }
    #[cfg(feature = "lang-rust")]
    {
        let _ = registry.register(Language::Rust, || Box::new(mehen_rust::RustAnalyzer::new()));
    }
    #[cfg(feature = "lang-go")]
    {
        let _ = registry.register(Language::Go, || Box::new(mehen_go::GoAnalyzer::new()));
    }
    #[cfg(feature = "lang-c")]
    {
        let _ = registry.register(Language::C, || Box::new(mehen_c::CAnalyzer::new()));
    }
    #[cfg(feature = "lang-kotlin")]
    {
        let _ = registry.register(Language::Kotlin, || {
            Box::new(mehen_kotlin::KotlinAnalyzer::new())
        });
    }
    #[cfg(feature = "lang-powershell")]
    {
        let _ = registry.register(Language::PowerShell, || {
            Box::new(mehen_powershell::PowerShellAnalyzer::new())
        });
    }
    #[cfg(feature = "lang-markdown")]
    {
        let _ = registry.register(Language::Markdown, || {
            Box::new(mehen_markdown::MarkdownAnalyzer::new())
        });
    }
}
