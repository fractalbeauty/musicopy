//! UniFFI definitions for iOS filesystem operations.
//!
//! These need to be available and identical on all targets for UniFFI, but
//! are only implemented on iOS.

use std::sync::Arc;

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum ResolveBookmarkError {
    #[error("failed to resolve bookmark: {reason}")]
    Unexpected { reason: String },
}

impl From<uniffi::UnexpectedUniFFICallbackError> for ResolveBookmarkError {
    fn from(e: uniffi::UnexpectedUniFFICallbackError) -> Self {
        Self::Unexpected {
            reason: format!("UnexpectedUniFFICallbackError: {}", e.reason),
        }
    }
}

#[uniffi::export(with_foreign)]
pub trait IosBookmarkResolver: Send + Sync {
    /// Takes a bookmark as a base64 string and returns a file URL as a string.
    /// Should also refresh the bookmark if stale.
    fn resolve_bookmark(&self, bookmark: String) -> Result<String, ResolveBookmarkError>;
}

/// Sets the global iOS bookmark resolver.
///
/// Should be called once at app startup.
#[uniffi::export]
pub fn set_ios_bookmark_resolver(resolver: Arc<dyn IosBookmarkResolver>) {
    #[cfg(target_os = "ios")]
    {
        crate::fs::ios::set_ios_bookmark_resolver(resolver);
    }
}
