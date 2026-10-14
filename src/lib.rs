pub mod error;
pub mod hook;
pub mod memory;
pub mod pipeline;
pub mod probe;

pub use error::HarnessError;
pub use hook::{install_pre_commit_hook, PRE_COMMIT_HOOK_CONTENT};
pub use memory::{
    is_invariant_or_lesson_path, jit_activate, priority_tier, record_lesson, search_local_lessons,
    JitActivationResult, JitHit, LessonRecordResult,
};
pub use pipeline::{
    format_wire_summary, inspect_receipt_wire, run_pipeline, PipelineOptions, PipelineResult,
};
pub use probe::{find_binary, ComponentProbe, ComponentStatus, HarnessHealthReport};
