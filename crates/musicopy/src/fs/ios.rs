//! iOS filesystem operations.
//!
//! To access files:
//! - Present a UIDocumentPickerViewController
//! - Receive a URL
//! - Create a bookmark and store it
//! - When writing, resolve the bookmark
//! - If stale, refresh it and store the new bookmark
//! - Start accessing security-scoped resources for the URL
//! - Append path components and access files normally
//! - When done, stop accessing security-scoped resources for the URL
//!
//! Document picking, bookmark creation, and storage are done in the UI layer.
//! The core layer is responsible for bookmark resolution and tracking access
//! to security-scoped resources.

use anyhow::Context;
use base64::{Engine, prelude::BASE64_STANDARD};
use objc2_core_foundation::{CFData, CFRetained, CFURL, CFURLBookmarkResolutionOptions};
use std::path::PathBuf;

/// Resolves a bookmark base64 string, returning the file path and an access
/// guard. The guard should be held until access is no longer needed.
pub fn resolve_bookmark(bookmark: String) -> anyhow::Result<(PathBuf, IosUrlAccessGuard)> {
    // decode bookmark
    let bookmark_bytes = BASE64_STANDARD
        .decode(bookmark)
        .context("failed to decode bookmark string")?;

    let bookmark_data = CFData::from_bytes(&bookmark_bytes);

    // resolve bookmark
    let mut is_stale: u8 = 0;
    let Some(bookmark_url) = (unsafe {
        CFURL::new_by_resolving_bookmark_data(
            None,
            Some(&bookmark_data),
            CFURLBookmarkResolutionOptions::CFURLBookmarkResolutionWithSecurityScope,
            None,
            None,
            &mut is_stale,
            std::ptr::null_mut(),
        )
    }) else {
        return Err(anyhow::anyhow!("failed to resolve bookmark data"));
    };

    if is_stale == 1 {
        log::warn!("resolved bookmark is stale");

        // TODO: handle refreshing the bookmark. annoying bc we should update
        // references to the stale bookmark in the database, and the bookmark
        // is currently stored in the ui layer
    }

    // start accessing security-scoped resource
    let guard = IosUrlAccessGuard::new(bookmark_url.clone());

    // get file path
    let path = bookmark_url
        .to_file_path()
        .ok_or_else(|| anyhow::anyhow!("failed to get file path from URL"))?;

    Ok((path, guard))
}

/// Guard that starts accessing a URL on creation and stops accessing it on drop.
#[derive(Debug)]
pub struct IosUrlAccessGuard {
    url: CFRetained<CFURL>,
}

impl IosUrlAccessGuard {
    pub fn new(url: CFRetained<CFURL>) -> Self {
        let success = unsafe { url.start_accessing_security_scoped_resource() };
        if !success {
            log::error!(
                "IosUrlAccessGuard::new: CFURLStartAccessingSecurityScopedResource returned false"
            );
        }

        Self { url }
    }
}

// stop accessing url on drop
impl Drop for IosUrlAccessGuard {
    fn drop(&mut self) {
        unsafe { self.url.stop_accessing_security_scoped_resource() };
    }
}

// make sure to start accessing another time on clone
impl Clone for IosUrlAccessGuard {
    fn clone(&self) -> Self {
        let success = unsafe { self.url.start_accessing_security_scoped_resource() };
        if !success {
            log::error!(
                "IosUrlAccessGuard::clone: CFURLStartAccessingSecurityScopedResource returned false"
            );
        }

        Self {
            url: self.url.clone(),
        }
    }
}
