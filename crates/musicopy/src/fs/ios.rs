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
//! Bookmark resolution and refreshing are also done in the UI layer using a
//! UniFFI trait interface. The core layer is responsible for tracking access
//! to security-scoped resources.

use super::ios_uniffi::*;
use core_foundation::{
    base::{CFRelease, TCFTypeRef},
    string::kCFStringEncodingUTF8,
    url::{
        CFURLCreateWithBytes, CFURLStartAccessingSecurityScopedResource,
        CFURLStopAccessingSecurityScopedResource,
    },
};
use dashmap::DashMap;
use std::sync::{Arc, LazyLock, OnceLock};

static IOS_BOOKMARK_RESOLVER: OnceLock<Arc<dyn IosBookmarkResolver>> = OnceLock::new();

/// Sets the global iOS bookmark resolver.
///
/// Should be called once at app startup.
pub fn set_ios_bookmark_resolver(resolver: Arc<dyn IosBookmarkResolver>) {
    let _ = IOS_BOOKMARK_RESOLVER.set(resolver);
}

/// Calls the global iOS bookmark resolver to resolve a bookmark.
pub fn resolve_bookmark(bookmark: String) -> Result<String, ResolveBookmarkError> {
    let resolver = IOS_BOOKMARK_RESOLVER
        .get()
        .ok_or_else(|| ResolveBookmarkError::Unexpected {
            reason: "ios bookmark resolver not initialized".into(),
        })?;

    resolver.resolve_bookmark(bookmark)
}

static URL_ACCESS_COUNTS: LazyLock<DashMap<String, usize>> = LazyLock::new(|| DashMap::new());

/// Increases the count for the security-scoped resource at the given URL.
///
/// If this is the first user, also starts accessing the security-scoped resource.
fn start_accessing_url(url: &str) -> anyhow::Result<()> {
    log::trace!("fs::ios: start_accessing_url called with url: {}", url);

    let mut count = URL_ACCESS_COUNTS.entry(url.to_string()).or_insert(0);
    *count += 1;

    log::trace!("fs::ios: access count increased to {} for {}", *count, url);

    if *count == 1 {
        log::trace!("fs::ios: start accessing security scoped resource: {}", url);
        start_accessing_security_scoped_resource(url)?;
    }

    Ok(())
}

/// Decreases the count for the security-scoped resource at the given URL.
///
/// If this is the last user, also stops accessing the security-scoped resource.
fn stop_accessing_url(url: &str) -> anyhow::Result<()> {
    log::trace!("fs::ios: stop_accessing_url called with url: {}", url);

    let mut count = URL_ACCESS_COUNTS
        .entry(url.to_string())
        // shouldn't happen, but preferable to call stop twice than panic
        .or_insert(1);
    *count -= 1;

    log::trace!("fs::ios: access count decreased to {} for {}", *count, url);

    if *count == 0 {
        // drop the entry reference to prevent deadlocking
        drop(count);

        URL_ACCESS_COUNTS.remove(url);

        log::trace!("fs::ios: stop accessing security scoped resource: {}", url);
        stop_accessing_security_scoped_resource(url)?;
    }

    Ok(())
}

// wrapper for CFURLStartAccessingSecurityScopedResource
fn start_accessing_security_scoped_resource(url: &str) -> anyhow::Result<()> {
    let url_bytes = url.as_bytes();

    let cf_url = unsafe {
        CFURLCreateWithBytes(
            std::ptr::null(),
            url_bytes.as_ptr(),
            url_bytes.len() as isize,
            kCFStringEncodingUTF8,
            std::ptr::null(),
        )
    };
    if cf_url.is_null() {
        anyhow::bail!("CFURLCreateWithBytes failed");
    }

    let success = unsafe { CFURLStartAccessingSecurityScopedResource(cf_url) };
    unsafe { CFRelease(cf_url.as_void_ptr()) };

    if success == 0 {
        anyhow::bail!("CFURLStartAccessingSecurityScopedResource failed");
    }

    Ok(())
}

// wrapper for CFURLStopAccessingSecurityScopedResource
fn stop_accessing_security_scoped_resource(url: &str) -> anyhow::Result<()> {
    let url_bytes = url.as_bytes();

    let cf_url = unsafe {
        CFURLCreateWithBytes(
            std::ptr::null(),
            url_bytes.as_ptr(),
            url_bytes.len() as isize,
            kCFStringEncodingUTF8,
            std::ptr::null(),
        )
    };
    if cf_url.is_null() {
        anyhow::bail!("CFURLCreateWithBytes failed");
    }

    unsafe { CFURLStopAccessingSecurityScopedResource(cf_url) };
    unsafe { CFRelease(cf_url.as_void_ptr()) };

    Ok(())
}

/// Guard that starts accessing a URL on creation and stops accessing it on drop.
#[derive(Debug)]
pub struct IosUrlAccessGuard {
    url: String,
}

impl IosUrlAccessGuard {
    pub fn new(url: String) -> Self {
        log::trace!("fs::ios::IosUrlAccessGuard::new called with url: {}", url);

        if let Err(e) = start_accessing_url(&url) {
            log::error!(
                "fs::ios::IosUrlAccessGuard failed to start accessing url {}: {}",
                url,
                e
            );
        }

        log::trace!("fs::ios::IosUrlAccessGuard started accessing url: {}", url);

        Self { url }
    }
}

// stop accessing url on drop
impl Drop for IosUrlAccessGuard {
    fn drop(&mut self) {
        if let Err(e) = stop_accessing_url(&self.url) {
            log::error!(
                "fs::ios: IosUrlAccessGuard Drop failed to stop accessing url {}: {}",
                self.url,
                e
            );
        }
    }
}

// make sure to also increase access count on clone
impl Clone for IosUrlAccessGuard {
    fn clone(&self) -> Self {
        if let Err(e) = start_accessing_url(&self.url) {
            log::error!(
                "fs::ios::IosUrlAccessGuard failed to start accessing url on clone {}: {}",
                self.url,
                e
            );
        }

        Self {
            url: self.url.clone(),
        }
    }
}
