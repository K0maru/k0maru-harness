pub mod error;
pub mod hook;
pub mod memory;
pub mod pipeline;
pub mod probe;

pub use error::HarnessError;
pub use hook::{install_pre_commit_hook, PRE_COMMIT_HOOK_CONTENT};
pub use memory::{jit_activate, JitActivationResult, JitHit};
pub use pipeline::{run_pipeline, PipelineOptions, PipelineResult};
pub use probe::{find_binary, ComponentProbe, ComponentStatus, HarnessHealthReport};
