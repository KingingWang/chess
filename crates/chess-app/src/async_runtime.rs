//! A shared Tokio runtime resource. Both the AI (CPU-bound, run via
//! `spawn_blocking`) and the network (IO-bound) live on this runtime so the
//! Bevy main/render thread is never blocked. Communication with Bevy systems
//! is via lock-free [`crossbeam_channel`]s polled each frame.

use std::sync::Arc;

use bevy::prelude::*;

#[derive(Resource, Clone)]
pub struct AsyncRuntime(pub Arc<tokio::runtime::Runtime>);

impl AsyncRuntime {
    pub fn new() -> Self {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("failed to build Tokio runtime");
        AsyncRuntime(Arc::new(rt))
    }
}

impl Default for AsyncRuntime {
    fn default() -> Self {
        Self::new()
    }
}
