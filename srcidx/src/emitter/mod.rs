//! Emitter trait for outputting symbol index in various formats.

pub mod ndjson;

use crate::index::SymbolIndex;
use std::io::Write;
use std::path::Path;

/// Trait for emitting symbol index in different formats.
pub trait IndexEmitter: Send + Sync {
    /// Write the symbol index to the given writer.
    fn emit(&self, index: &SymbolIndex, writer: &mut dyn Write) -> anyhow::Result<()>;

    /// Format name (e.g., "ndjson", "msgpack").
    fn format_name(&self) -> &'static str;

    /// File extension for this format.
    fn extension(&self) -> &'static str;
}

/// Registry of emitters.
pub struct EmitterRegistry {
    emitters: Vec<Box<dyn IndexEmitter>>,
}

impl EmitterRegistry {
    pub fn new() -> Self {
        Self {
            emitters: Vec::new(),
        }
    }

    pub fn register(&mut self, emitter: Box<dyn IndexEmitter>) {
        self.emitters.push(emitter);
    }

    pub fn get(&self, format: &str) -> Option<&dyn IndexEmitter> {
        self.emitters
            .iter()
            .find(|e| e.format_name() == format)
            .map(|e| e.as_ref())
    }

    pub fn emit_to_file(
        &self,
        index: &SymbolIndex,
        format: &str,
        path: &Path,
    ) -> anyhow::Result<()> {
        let emitter = self
            .get(format)
            .ok_or_else(|| anyhow::anyhow!("Unknown format: {}", format))?;
        let mut file = std::fs::File::create(path)?;
        emitter.emit(index, &mut file)
    }
}

impl Default for EmitterRegistry {
    fn default() -> Self {
        let mut registry = Self::new();
        registry.register(Box::new(crate::emitter::ndjson::NdjsonEmitter::new()));
        registry
    }
}
