//! Segment pipeline: resolution, concurrent execution, format template evaluation.
//!
//! Resolves which segments to run from config, executes them concurrently,
//! and evaluates the format template to produce the final prompt string.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum PipelineError {
    #[error("script error: {0}")]
    Script(#[from] crate::script::ScriptError),

    #[error("timeout: prompt render exceeded {0}ms")]
    Timeout(u64),
}

// TODO: implement pipeline
// - resolve segment list from format template
// - execute segment scripts concurrently via rayon
// - evaluate format template with s("name") lookups
// - enforce global timeout
