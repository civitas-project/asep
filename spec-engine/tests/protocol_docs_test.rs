use std::fs;
use std::path::Path;

// @test:FR-SPEC-DOCS-01
#[test]
fn test_engineering_specs_zero_antislop_and_puml_templates() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let specs_dir = root.join("engineering-specs");

    // 1. Verify zero "antislop" mentions in engineering-specs
    for entry in walkdir(specs_dir.clone()) {
        if entry.is_file() {
            let content = fs::read_to_string(&entry).unwrap_or_default();
            assert!(
                !content.to_lowercase().contains("antislop"),
                "Found prohibited external mention 'antislop' in {}",
                entry.display()
            );
        }
    }

    // 2. Verify all 4 PlantUML templates exist and are valid PlantUML
    let templates_dir = specs_dir.join("templates");
    let required_pumls = [
        "use_case_template.puml",
        "activity_template.puml",
        "sequence_bce_template.puml",
        "class_diagram_template.puml",
    ];

    for puml_name in &required_pumls {
        let puml_path = templates_dir.join(puml_name);
        assert!(
            puml_path.exists(),
            "PlantUML template {} must exist",
            puml_name
        );
        let content = fs::read_to_string(&puml_path).unwrap();
        assert!(
            content.contains("@startuml"),
            "{} missing @startuml",
            puml_name
        );
        assert!(content.contains("@enduml"), "{} missing @enduml", puml_name);
    }

    // 3. Verify Skeletons contain the required diagram sections
    let prd_skeleton = fs::read_to_string(specs_dir.join("skeletons").join("PRD_SKELETON.md"))
        .expect("PRD_SKELETON.md must exist");
    assert!(
        prd_skeleton.contains("Use Case Diagram"),
        "PRD must contain Use Case Diagram"
    );
    assert!(
        prd_skeleton.contains("Activity Diagram"),
        "PRD must contain Activity Diagram"
    );

    let tdd_skeleton = fs::read_to_string(specs_dir.join("skeletons").join("TDD_SKELETON.md"))
        .expect("TDD_SKELETON.md must exist");
    assert!(
        tdd_skeleton.contains("Sequence Diagram"),
        "TDD must contain Sequence Diagram"
    );
    assert!(
        tdd_skeleton.contains("Class Diagram"),
        "TDD must contain Class Diagram"
    );
}

// @test:FR-CONTRACTS-01
#[test]
fn test_code_contracts_multilang_and_zero_antislop() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let contracts_dir = root.join("code-contracts");

    // 1. Verify zero "antislop" mentions in code-contracts
    for entry in walkdir(contracts_dir.clone()) {
        if entry.is_file() {
            let content = fs::read_to_string(&entry).unwrap_or_default();
            assert!(
                !content.to_lowercase().contains("antislop"),
                "Found prohibited external mention 'antislop' in {}",
                entry.display()
            );
        }
    }

    // 2. Verify all 4 multi-language contract templates exist
    let templates_dir = contracts_dir.join("templates");
    let required_templates = [
        ("ts_contract.ts", "@remarks"),
        ("py_contract.py", "Invariants:"),
        ("go_contract.go", "Err"),
        ("cpp_contract.hpp", "@invariant"),
    ];

    for (file_name, expected_keyword) in &required_templates {
        let file_path = templates_dir.join(file_name);
        assert!(
            file_path.exists(),
            "Contract template {} must exist",
            file_name
        );
        let content = fs::read_to_string(&file_path).unwrap();
        assert!(
            content.contains(expected_keyword),
            "{} must contain '{}'",
            file_name,
            expected_keyword
        );
    }

    // 3. Verify code-contracts.md contains SWEBOK 4.4 Design by Contract and language sections
    let doc_content = fs::read_to_string(contracts_dir.join("code-contracts.md"))
        .expect("code-contracts.md must exist");
    assert!(
        doc_content.contains("SWEBOK") || doc_content.contains("Design by Contract"),
        "code-contracts.md must cite SWEBOK / Design by Contract"
    );
    assert!(doc_content.contains("TypeScript"), "must cover TypeScript");
    assert!(doc_content.contains("Python"), "must cover Python");
    assert!(doc_content.contains("Go"), "must cover Go");
    assert!(doc_content.contains("C++"), "must cover C++");
}

// @test:FR-GOV-DOCS-01
#[test]
fn test_repository_governance_zero_antislop_and_readme_template() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let gov_dir = root.join("repository-governance");

    // 1. Verify zero "antislop" mentions in repository-governance
    for entry in walkdir(gov_dir.clone()) {
        if entry.is_file() {
            let content = fs::read_to_string(&entry).unwrap_or_default();
            assert!(
                !content.to_lowercase().contains("antislop"),
                "Found prohibited external mention 'antislop' in {}",
                entry.display()
            );
        }
    }

    // 2. Verify README_TEMPLATE.md exists and contains canonical SWEBOK v4 sections
    let readme_template_path = gov_dir.join("templates").join("README_TEMPLATE.md");
    assert!(
        readme_template_path.exists(),
        "README_TEMPLATE.md must exist in repository-governance/templates/"
    );
    let readme_content = fs::read_to_string(&readme_template_path).unwrap();
    let required_sections = [
        "Architectural Highlights",
        "Target Platform & Prerequisites",
        "Quantitative Verification",
        "Known Limitations",
        "Workspace Layout",
        "Governance, Provenance & Security",
    ];
    for section in &required_sections {
        assert!(
            readme_content.contains(section),
            "README_TEMPLATE.md missing section: {}",
            section
        );
    }

    // 3. Verify repository-governance.md contains SWEBOK v4 SCM and Security enhancements
    let gov_doc = fs::read_to_string(gov_dir.join("repository-governance.md"))
        .expect("repository-governance.md must exist");
    assert!(
        gov_doc.contains("SWEBOK"),
        "governance doc must cite SWEBOK"
    );
    assert!(
        gov_doc.contains("CVSS") || gov_doc.contains("Vulnerability"),
        "governance doc must detail security vulnerability handling"
    );
}

fn walkdir(dir: std::path::PathBuf) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(walkdir(path));
            } else {
                files.push(path);
            }
        }
    }
    files
}
