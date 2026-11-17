use crate::error::AsepError;
use std::fs;
use std::path::Path;
use std::process::Command;

// @impl:FR-CLI-RENDER-01
/// Compiles PlantUML sequence and architectural diagrams into rendered assets.
///
/// Discovers `.puml` files within `docs/diagrams/` and invokes `plantuml` to render them into `docs/diagrams/rendered/`.
pub fn render_plantuml_diagrams(root: &Path) -> Result<usize, AsepError> {
    let diagrams_dir = root.join("docs/diagrams");
    if !diagrams_dir.exists() {
        return Ok(0);
    }

    let rendered_dir = diagrams_dir.join("rendered");
    fs::create_dir_all(&rendered_dir)?;

    let mut count = 0;
    for entry in fs::read_dir(&diagrams_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("puml") {
            let output = Command::new("plantuml")
                .arg("-tsvg")
                .arg("-o")
                .arg(&rendered_dir)
                .arg(&path)
                .output();

            match output {
                Ok(out) if out.status.success() => {
                    count += 1;
                }
                _ => {
                    // Fallback to plantuml runner with output flag
                    let fallback = Command::new("plantuml")
                        .arg("-o")
                        .arg(&rendered_dir)
                        .arg(&path)
                        .output();

                    match fallback {
                        Ok(out) if out.status.success() => {
                            count += 1;
                        }
                        Ok(out) => {
                            let err_msg = String::from_utf8_lossy(&out.stderr);
                            return Err(AsepError::Diagram(format!(
                                "plantuml exited with code {:?}: {}",
                                out.status.code(),
                                err_msg.trim()
                            )));
                        }
                        Err(e) => {
                            return Err(AsepError::Diagram(format!(
                                "failed to execute plantuml: {e}"
                            )));
                        }
                    }
                }
            }
        }
    }

    Ok(count)
}
