pub mod anti_slop;

pub use anti_slop::{
    audit_anti_slop, audit_project_anti_slop, AntiSlopViolation, AntiSlopViolationKind,
};

pub mod doc_contracts;

pub use doc_contracts::{
    audit_doc_contracts, audit_project_docstrings, DocContractViolation,
    DocContractViolationKind,
};
