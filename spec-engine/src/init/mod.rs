use crate::error::AsepError;
use std::fs;
use std::path::Path;

// @impl:FR-CLI-INIT-01 @impl:FR-CLI-03 @impl:FR-INIT-01
/// Initializes standard documentation directories and baseline PRD specification.
pub fn init_asep_project(root: &Path) -> Result<(), AsepError> {
    let docs_dir = root.join("docs");
    let adr_dir = docs_dir.join("adr");
    let diagrams_dir = docs_dir.join("diagrams");
    let rendered_dir = diagrams_dir.join("rendered");
    let invariants_dir = docs_dir.join("invariants");
    let prd_path = docs_dir.join("PRD.md");

    fs::create_dir_all(&adr_dir)?;
    fs::create_dir_all(&rendered_dir)?;
    fs::create_dir_all(&invariants_dir)?;

    if !prd_path.exists() {
        let starter_prd = r#"# Product Requirements Document (PRD)

<!-- @req:FR-INIT-01:MUST Baseline project specification -->
## 1. Overview
Project initialized under ASEP SDLC framework.
"#;
        fs::write(&prd_path, starter_prd)?;
    }

    Ok(())
}
