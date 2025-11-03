//! # Filesystem module
//! Cross-platform filesystem abstraction, mainly to support Android's document system.
//!
//! Notes:
//! - Lots of operations on Android require the URI of the tree containing a document
//! - For now we're representing paths as a combination of a tree URI and a subpath.
//! - URIs on Android are opaque, they can't be treated as paths
//!     - Need to recursively traverse using JNI calls, will provide helpers
//! - content:// urls on android, maybe file:// urls on desktop
//! - For now we're not returning URIs anywhere, mostly just files.
//!     - We might want a notion of resolved vs unresolved, so we can return a URI to a document
//!       in a deep subfolder instead of its path to not have to resolve it again.
//! - Will add support for iOS/MacOS sandboxing ("bookmarks"?) later

#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "ios")]
mod ios;
mod ios_uniffi;

use anyhow::Context;
use std::{borrow::Cow, path::PathBuf, pin::Pin};
use tokio::{
    fs::{File as TokioFile, OpenOptions},
    io::{AsyncRead, AsyncWrite, AsyncWriteExt},
};

pub enum OpenMode {
    Read,
    Write,
}

/// Struct representing a path in a subtree of the filesystem.
///
/// This is required for mobile. On Android, filesystem access is granted to
/// specific subtrees, and requires the tree URI to perform operations. On iOS,
/// persisting filesystem access requires security-scoped bookmarks.
///
/// On desktop, the standard filesystem path can be obtained by appending the
/// path to the tree.
///
/// On iOS, the root at creation should be a base64-encoded bookmark. It will
/// be resolved immediately using the global IosBookmarkResolver. It then
/// starts accessing security-scoped resources for the resolved URL. When
/// the TreePath is dropped, it eventually stops accessing the security-scoped
/// resource if it was the last user of that URL.
#[derive(Debug, Clone)]
pub struct TreePath {
    /// The URI of the tree.
    ///
    /// On Android, this is a tree URI.
    ///
    /// On iOS, this is a file URL resolved from a bookmark.
    ///
    /// Otherwise, this is a regular path.
    tree: String,
    /// The subpath within the tree.
    path: PathBuf,

    // on ios, this guard handles starting and stopping security-scoped access
    #[cfg(target_os = "ios")]
    url_access_guard: ios::IosUrlAccessGuard,
}

impl TreePath {
    /// Creates a TreePath from the given root and path.
    ///
    /// On iOS, the root should be a base64-encoded bookmark.
    pub fn new(root: String, path: PathBuf) -> Self {
        // resolve bookmark
        // TODO: handle error?
        #[cfg(target_os = "ios")]
        let root = ios::resolve_bookmark(root).expect("failed to resolve ios bookmark");

        Self {
            #[cfg(target_os = "ios")]
            url_access_guard: ios::IosUrlAccessGuard::new(root.clone()),

            tree: root,
            path,
        }
    }

    /// Creates a TreePath from the given root.
    ///
    /// On iOS, the root should be a base64-encoded bookmark.
    pub fn from_root(root: String) -> Self {
        Self::new(root, PathBuf::new())
    }

    pub fn root(&self) -> &str {
        &self.tree
    }

    pub fn path(&self) -> Cow<'_, str> {
        self.path.to_string_lossy()
    }

    pub fn push(&mut self, component: &str) {
        self.path.push(component);
    }

    pub fn join(&self, component: &str) -> Self {
        let mut new_path = self.path.clone();
        new_path.push(component);
        Self {
            tree: self.tree.clone(),
            path: new_path,

            #[cfg(target_os = "ios")]
            url_access_guard: self.url_access_guard.clone(),
        }
    }

    pub fn extension(&self) -> Option<Cow<'_, str>> {
        self.path.extension().map(|s| s.to_string_lossy())
    }

    pub fn set_extension(&mut self, extension: &str) {
        self.path.set_extension(extension);
    }

    pub fn parent(&self) -> Option<Self> {
        self.path.parent().map(|p| Self {
            tree: self.tree.clone(),
            path: p.to_path_buf(),

            #[cfg(target_os = "ios")]
            url_access_guard: self.url_access_guard.clone(),
        })
    }

    pub fn is_empty(&self) -> bool {
        self.path.as_os_str().is_empty()
    }

    // on desktop, just join the tree and path
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    pub fn resolve_path(&self) -> anyhow::Result<PathBuf> {
        let mut p = PathBuf::from(&self.tree);
        p.push(&self.path);
        Ok(p)
    }

    // on ios, we need to convert the file url to a real file path (to remove
    // the scheme and percent encoding)
    #[cfg(target_os = "ios")]
    pub fn resolve_path(&self) -> anyhow::Result<PathBuf> {
        let tree_url = url::Url::parse(&self.tree).context("failed to parse tree url")?;

        let mut path = tree_url
            .to_file_path()
            .map_err(|_| anyhow::anyhow!("failed to convert tree url to file path"))?;

        path.push(&self.path);

        Ok(path)
    }

    pub fn exists(&self) -> bool {
        #[cfg(not(target_os = "android"))]
        {
            let mut p = PathBuf::from(&self.tree);
            p.push(&self.path);
            p.exists()
        }
        #[cfg(target_os = "android")]
        {
            android::exists(self).unwrap_or(false)
        }
    }
}

pub struct TreeFile {
    #[cfg(not(target_os = "android"))]
    file: TokioFile,

    #[cfg(target_os = "android")]
    file: android::FileHandle,
}

impl TreeFile {
    pub async fn create(path: &TreePath) -> anyhow::Result<Self> {
        #[cfg(not(target_os = "android"))]
        {
            let resolved_path = path.resolve_path()?;
            let file = TokioFile::create(&resolved_path).await?;
            Ok(Self { file })
        }
        #[cfg(target_os = "android")]
        {
            let file =
                android::open_or_create_file(path, android::AccessMode::Create, OpenMode::Write)?;
            Ok(Self { file })
        }
    }

    pub async fn open(path: &TreePath, mode: OpenMode) -> anyhow::Result<Self> {
        #[cfg(not(target_os = "android"))]
        {
            let resolved_path = path.resolve_path()?;
            let file = match mode {
                OpenMode::Read => OpenOptions::new().read(true).open(&resolved_path).await?,
                OpenMode::Write => {
                    OpenOptions::new()
                        .write(true)
                        .truncate(true)
                        .open(&resolved_path)
                        .await?
                }
            };
            Ok(Self { file })
        }
        #[cfg(target_os = "android")]
        {
            let file = android::open_or_create_file(path, android::AccessMode::Open, mode)?;
            Ok(Self { file })
        }
    }

    pub async fn open_or_create(path: &TreePath, mode: OpenMode) -> anyhow::Result<Self> {
        #[cfg(not(target_os = "android"))]
        {
            let resolved_path = path.resolve_path()?;
            let file = match mode {
                OpenMode::Read => {
                    OpenOptions::new()
                        .read(true)
                        .truncate(false)
                        .create(true)
                        .open(&resolved_path)
                        .await?
                }
                OpenMode::Write => {
                    OpenOptions::new()
                        .write(true)
                        .truncate(true)
                        .create(true)
                        .open(&resolved_path)
                        .await?
                }
            };
            Ok(Self { file })
        }
        #[cfg(target_os = "android")]
        {
            let file = android::open_or_create_file(path, android::AccessMode::OpenOrCreate, mode)?;
            Ok(Self { file })
        }
    }

    fn file(&self) -> &TokioFile {
        #[cfg(not(target_os = "android"))]
        {
            &self.file
        }
        #[cfg(target_os = "android")]
        {
            self.file.file()
        }
    }

    fn file_mut(&mut self) -> &mut TokioFile {
        #[cfg(not(target_os = "android"))]
        {
            &mut self.file
        }
        #[cfg(target_os = "android")]
        {
            self.file.file_mut()
        }
    }

    pub async fn write_all(&mut self, buf: &[u8]) -> anyhow::Result<()> {
        self.file_mut().write_all(buf).await?;
        Ok(())
    }
}

impl AsyncRead for TreeFile {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        Pin::new(self.file_mut()).poll_read(cx, buf)
    }
}

impl AsyncWrite for TreeFile {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        Pin::new(self.file_mut()).poll_write(cx, buf)
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        Pin::new(self.file_mut()).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        Pin::new(self.file_mut()).poll_shutdown(cx)
    }

    fn poll_write_vectored(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        bufs: &[std::io::IoSlice<'_>],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        Pin::new(self.file_mut()).poll_write_vectored(cx, bufs)
    }

    fn is_write_vectored(&self) -> bool {
        self.file().is_write_vectored()
    }
}

pub async fn create_dir_all(path: &TreePath) -> anyhow::Result<()> {
    #[cfg(not(target_os = "android"))]
    {
        let resolved_path = path.resolve_path()?;
        tokio::fs::create_dir_all(&resolved_path).await?;
        Ok(())
    }
    #[cfg(target_os = "android")]
    {
        android::create_dir_all(path)?;
        Ok(())
    }
}
