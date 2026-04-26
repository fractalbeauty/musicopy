//! Android filesystem operations.
//!
//! To open a file in a subfolder:
//! - Receive subtree URI from OPEN_DOCUMENT_TREE
//! - Recursively resolve/create dirs if they don't exist
//! - Check if file exists
//! - Create file if it doesn't exist
//! - Otherwise open file with write+truncate
//! - Provide std Write
//!
//! To do operations on files, we can open a document URI as a ParcelFileDescriptor and then wrap
//! it in a std or tokio File and use that. We don't run Drop for the File since we're only using
//! it to wrap the file descriptor, which we manually close using the ParcelFileDescriptor via JNI.

use crate::fs::{OpenMode, TreePath};
use anyhow::Context;
use jni::{
    JNIEnv, JavaVM,
    objects::{GlobalRef, JObject, JObjectArray, JValue, JValueOwned},
    strings::JNIString,
    sys::{jint, jsize},
};
use std::{borrow::Cow, collections::HashMap, mem::ManuallyDrop, ops::Deref, os::fd::FromRawFd};
use tokio::fs::File as TokioFile;
use tracing::error;

const MIME_TYPE_DIR: &str = "vnd.android.document/directory";

/// Recursively resolve or create a series of directories,
pub fn create_dir_all(path: &TreePath) -> anyhow::Result<String> {
    if path.is_empty() {
        anyhow::bail!("path is empty");
    }

    let vm = get_vm();
    let mut env = vm.attach_current_thread()?;

    let segments = path
        .path
        .components()
        .map(|s| s.as_os_str().to_string_lossy())
        .collect::<Vec<_>>();

    let tree_uri = Uri::parse(&mut env, &path.tree).context("Uri::parse failed")?;

    let result_uri = resolve_dirs(&mut env, &tree_uri, segments, true)?;

    // TODO probably shouldnt return string
    result_uri
        .to_string(&mut env)
        .context("Uri::to_string failed")
}

/// Check if a file exists at the given path.
pub fn exists(path: &TreePath) -> anyhow::Result<bool> {
    if path.is_empty() {
        anyhow::bail!("path is empty");
    }

    let vm = get_vm();
    let mut env = vm.attach_current_thread()?;

    let mut segments = path
        .path
        .components()
        .map(|s| s.as_os_str().to_string_lossy())
        .collect::<Vec<_>>();

    let Some(filename) = segments.pop() else {
        anyhow::bail!("path is empty");
    };

    let tree_uri = Uri::parse(&mut env, &path.tree).context("Uri::parse failed")?;
    let parent_uri = resolve_dirs(&mut env, &tree_uri, segments, false)
        .context("failed to resolve parent directories")?;

    let content_resolver = ContentResolver::get(&mut env).context("ContentResolver::get failed")?;

    let child_uri = content_resolver
        .find_child(&mut env, &tree_uri, &parent_uri, &filename)
        .context("ContentResolver::find_child failed")?;

    Ok(child_uri.is_some())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessMode {
    /// Attempt to open the file, failing if it doesn't exist.
    Open,
    /// Attempt to create the file, failing if it already exists.
    Create,
    /// Attempt to open the file if it exists, otherwise create it.
    OpenOrCreate,
}

pub fn open_or_create_file(
    path: &TreePath,
    access_mode: AccessMode,
    open_mode: OpenMode,
) -> anyhow::Result<FileHandle> {
    if path.is_empty() {
        anyhow::bail!("path is empty");
    }

    let vm = get_vm();
    let mut env = vm.attach_current_thread()?;

    let mut segments = path
        .path
        .components()
        .map(|s| s.as_os_str().to_string_lossy())
        .collect::<Vec<_>>();

    let Some(filename) = segments.pop() else {
        anyhow::bail!("path is empty");
    };

    let tree_uri = Uri::parse(&mut env, &path.tree).context("Uri::parse failed")?;
    let parent_uri = resolve_dirs(&mut env, &tree_uri, segments, false)
        .context("failed to resolve parent directories")?;

    let content_resolver = ContentResolver::get(&mut env).context("ContentResolver::get failed")?;

    let child_uri = content_resolver
        .find_child(&mut env, &tree_uri, &parent_uri, &filename)
        .context("ContentResolver::find_child failed")?;

    match child_uri {
        Some(child_uri) => {
            if access_mode == AccessMode::Create {
                anyhow::bail!("file already exists");
            }

            let handle = FileHandle::open(&mut env, &child_uri, open_mode)
                .context("FileHandle::open failed")?;

            Ok(handle)
        }
        None => {
            if access_mode == AccessMode::Open {
                anyhow::bail!("file not found: {:?}", path);
            }

            let mime_type = match path.extension().as_deref() {
                Some("ogg") => "audio/ogg",
                Some("opus") => "audio/ogg",
                Some("flac") => "audio/flac",
                Some("mp3") => "audio/mpeg",
                Some("txt") => "text/plain",
                _ => "application/octet-stream",
            };
            let mime_type_string = env.new_string(mime_type)?;
            let filename = env.new_string(filename)?;

            let document_uri = DocumentsContract::jni_create_document(
                &mut env,
                &content_resolver,
                &parent_uri,
                &mime_type_string,
                &filename,
            )?;

            let handle = FileHandle::open(&mut env, &document_uri, OpenMode::Write)
                .context("FileHandle::open failed")?;

            Ok(handle)
        }
    }
}

pub struct FileHandle {
    parcel: GlobalRef,
    file: ManuallyDrop<TokioFile>,
}

impl FileHandle {
    fn open<'other_local>(
        env: &mut JNIEnv,
        document_uri: &Uri<'other_local>,
        mode: OpenMode,
    ) -> anyhow::Result<Self> {
        let content_resolver = ContentResolver::get(env).context("ContentResolver::get failed")?;

        let mode_str = match mode {
            OpenMode::Read => "r",
            OpenMode::Write => "wt",
        };
        let mode_string = env.new_string(mode_str).context("new_string failed")?;

        let parcel = content_resolver.jni_open_file_descriptor(env, document_uri, &mode_string)?;
        let parcel = env.new_global_ref(parcel)?;

        let fd = env.call_method(&parcel, "getFd", "()I", &[])?.i()?;

        let file = unsafe { TokioFile::from_raw_fd(fd) };

        Ok(Self {
            parcel,
            file: ManuallyDrop::new(file),
        })
    }

    pub fn file(&self) -> &TokioFile {
        &self.file
    }

    pub fn file_mut(&mut self) -> &mut TokioFile {
        &mut self.file
    }
}

impl Drop for FileHandle {
    fn drop(&mut self) {
        // forget the file to prevent its Drop implementation from being called
        let file = unsafe { ManuallyDrop::take(&mut self.file) };
        std::mem::forget(file);

        let vm = get_vm();
        let mut env = match vm.attach_current_thread() {
            Ok(env) => env,
            Err(e) => {
                error!(
                    "android::FileHandle::Drop failed to attach current thread: {}",
                    e
                );
                return;
            }
        };

        // close the ParcelFileDescriptor
        if let Err(e) = env.call_method(&self.parcel, "close", "()V", &[]) {
            error!("android::FileHandle::Drop failed to close parcel: {}", e);
        }
    }
}

/// Retrieve the VM from `ndk_context`.
fn get_vm() -> JavaVM {
    unsafe { JavaVM::from_raw(ndk_context::android_context().vm().cast()) }
        .expect("ndk_context should be initialized")
}

/// Retrieve the Context JObject from `ndk_context`.
fn get_context<'local>() -> JObject<'local> {
    unsafe { JObject::<'local>::from_raw(ndk_context::android_context().context().cast()) }
}

/// Recursively resolves a series of directories from the subtree root, returning a document URI.
///
/// Takes the URI of the containing subtree and a Vec of path segments.
/// Also takes a `create` flag to optionally create directories if they don't exist.
fn resolve_dirs<'local, 'other_local_1>(
    env: &mut JNIEnv<'local>,
    tree_uri: &Uri<'other_local_1>,
    segments: Vec<Cow<'_, str>>,
    create: bool,
) -> anyhow::Result<Uri<'local>> {
    let mime_type_dir_string = env.new_string(MIME_TYPE_DIR)?;

    let content_resolver = ContentResolver::get(env).context("ContentResolver::get failed")?;

    let tree_document_id = DocumentsContract::jni_get_tree_document_id(env, tree_uri)
        .context("jni_get_tree_document_id failed")?;
    let tree_document_uri =
        DocumentsContract::jni_build_document_uri_using_tree(env, tree_uri, &tree_document_id)
            .context("jni_build_document_uri_using_tree failed")?;

    // TODO: maybe root uri is not necessarily tree uri?
    let mut curr_uri = tree_document_uri.new_local_ref(env)?;

    'outer: for segment in &segments {
        let curr_document_id = DocumentsContract::jni_get_document_id(env, &curr_uri)?;
        let curr_children_uri = DocumentsContract::jni_build_child_documents_uri_using_tree(
            env,
            tree_uri,
            &curr_document_id,
        )?;

        for row in content_resolver.query(
            env,
            &curr_children_uri,
            &[
                DocumentsContract::COLUMN_DOCUMENT_ID,
                DocumentsContract::COLUMN_DISPLAY_NAME,
            ],
        )? {
            let child_document_id = row.document_id()?;
            let child_display_name = row.display_name()?;

            // TODO: check for dir mime

            if let (Some(child_document_id), Some(child_display_name)) =
                (child_document_id, child_display_name)
            {
                let child_display_name_std: String =
                    env.get_string(child_display_name.into())?.into();

                if child_display_name_std == *segment {
                    let child_uri = DocumentsContract::jni_build_document_uri_using_tree(
                        env,
                        tree_uri,
                        &child_document_id,
                    )?;

                    curr_uri = child_uri;
                    continue 'outer;
                }
            }
        }

        // not found
        if create {
            let display_name_string = env.new_string(segment)?;

            let child_uri = DocumentsContract::jni_create_document(
                env,
                &content_resolver,
                &curr_uri,
                &mime_type_dir_string,
                &display_name_string,
            )
            .context("jni_create_document failed")?;

            curr_uri = child_uri;
        } else {
            anyhow::bail!("segment {:?} not found in path {:?}", segment, segments);
        }
    }

    Ok(curr_uri)
}

/// Newtype for ContentResolver JObjects.
struct ContentResolver<'local>(JObject<'local>);

impl<'local> ContentResolver<'local> {
    /// https://developer.android.com/reference/android/content/Context#getContentResolver()
    fn get(env: &mut JNIEnv<'local>) -> anyhow::Result<Self> {
        let context = get_context();
        let content_resolver = env
            .call_method(
                context,
                "getContentResolver",
                "()Landroid/content/ContentResolver;",
                &[],
            )?
            .l()?;
        anyhow::ensure!(
            !content_resolver.is_null(),
            "Context#getContentResolver returned null"
        );
        Ok(Self(content_resolver))
    }

    fn query<'other_local_1, 'other_local_2>(
        &self,
        env: &mut JNIEnv<'other_local_1>,
        uri: &Uri<'other_local_2>,
        projections: &[&str],
    ) -> anyhow::Result<Vec<Row<'other_local_1>>> {
        let projection_strings = projections
            .iter()
            .map(|s| env.new_string(s))
            .collect::<Result<Vec<_>, _>>()?;

        let projections_array = env.new_object_array(
            projections.len() as jsize,
            "java/lang/String",
            JObject::null(),
        )?;
        for (i, string) in projection_strings.iter().enumerate() {
            env.set_object_array_element(&projections_array, i as jsize, string)?;
        }

        let cursor = self.jni_query(
            env,
            uri,
            &projections_array,
            &JObject::null(),
            &JObject::null(),
        )?;

        let mut rows = Vec::new();

        while cursor.jni_move_to_next(env)? {
            let mut column_values = HashMap::new();

            for (i, column_name) in projections.iter().enumerate() {
                let column_index = cursor
                    .jni_get_column_index(env, &projection_strings[i])?
                    .ok_or_else(|| anyhow::anyhow!("column does not exist"))?;

                // TODO: switch method by expected type
                let value = cursor.jni_get_string(env, column_index)?;
                if !value.is_null() {
                    column_values.insert(column_name.to_string(), JValueOwned::Object(value));
                }
            }

            if !column_values.is_empty() {
                rows.push(Row(column_values));
            }
        }

        cursor.jni_close(env)?;

        Ok(rows)
    }

    /// Get the URI of a child by name if it exists.
    ///
    /// Takes the URI of the parent, and the URI of the containing subtree.
    fn find_child<'other_local_1, 'other_local_2, 'other_local_3>(
        &self,
        env: &mut JNIEnv<'other_local_1>,
        tree_uri: &Uri<'other_local_2>,
        parent_uri: &Uri<'other_local_3>,
        name: &str,
    ) -> anyhow::Result<Option<Uri<'other_local_1>>> {
        let parent_document_id = DocumentsContract::jni_get_document_id(env, parent_uri)?;
        let children_uri = DocumentsContract::jni_build_child_documents_uri_using_tree(
            env,
            tree_uri,
            &parent_document_id,
        )?;

        for row in self.query(
            env,
            &children_uri,
            &[
                DocumentsContract::COLUMN_DOCUMENT_ID,
                DocumentsContract::COLUMN_DISPLAY_NAME,
            ],
        )? {
            let child_document_id = row.document_id()?;
            let child_display_name = row.display_name()?;

            if let (Some(child_document_id), Some(child_display_name)) =
                (child_document_id, child_display_name)
            {
                let child_display_name_std: String =
                    env.get_string(child_display_name.into())?.into();

                if child_display_name_std == name {
                    let child_uri = DocumentsContract::jni_build_document_uri_using_tree(
                        env,
                        tree_uri,
                        &child_document_id,
                    )?;
                    return Ok(Some(child_uri));
                }
            }
        }

        Ok(None)
    }

    fn jni_open_file_descriptor<'other_local_1, 'other_local_2, 'other_local_3>(
        &self,
        env: &mut JNIEnv<'other_local_1>,
        uri: &Uri<'other_local_2>,
        mode: &JObject<'other_local_3>,
    ) -> anyhow::Result<JObject<'other_local_1>> {
        let fd = env
            .call_method(
                &self.0,
                "openFileDescriptor",
                "(Landroid/net/Uri;Ljava/lang/String;)Landroid/os/ParcelFileDescriptor;",
                &[JValue::Object(uri), JValue::Object(mode)],
            )?
            .l()?;
        anyhow::ensure!(
            !fd.is_null(),
            "ContentResolver#openFileDescriptor returned null"
        );
        Ok(fd)
    }

    /// https://developer.android.com/reference/android/content/ContentProvider#query(android.net.Uri,%20java.lang.String[],%20android.os.Bundle,%20android.os.CancellationSignal)
    fn jni_query<'other_local_1, 'other_local_2, 'other_local_3, 'other_local_4, 'other_local_5>(
        &self,
        env: &mut JNIEnv<'other_local_1>,
        uri: &Uri<'other_local_2>,
        projection: &JObjectArray<'other_local_3>,
        query_args: &JObject<'other_local_4>,
        cancellation_signal: &JObject<'other_local_5>,
    ) -> anyhow::Result<Cursor<'other_local_1>> {
        let cursor = env.call_method(
            &self.0,
            "query",
            "(Landroid/net/Uri;[Ljava/lang/String;Landroid/os/Bundle;Landroid/os/CancellationSignal;)Landroid/database/Cursor;",
            &[
                JValue::Object(uri),
                JValue::Object(projection),
                JValue::Object(query_args),
                JValue::Object(cancellation_signal),
            ],
        )?.l()?;
        anyhow::ensure!(!cursor.is_null(), "ContentResolver#query returned null");
        Ok(Cursor(cursor))
    }
}

impl<'local> Deref for ContentResolver<'local> {
    type Target = JObject<'local>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Struct representing a query row.
struct Row<'local>(HashMap<String, JValueOwned<'local>>);

impl<'local> Row<'local> {
    fn document_id(&'local self) -> anyhow::Result<Option<DocumentId<'local>>> {
        Ok(self
            .0
            .get(DocumentsContract::COLUMN_DOCUMENT_ID)
            .map(|v| v.borrow().l().map(DocumentId::Borrowed))
            .transpose()?)
    }

    fn display_name(&'local self) -> anyhow::Result<Option<&'local JObject<'local>>> {
        Ok(self
            .0
            .get(DocumentsContract::COLUMN_DISPLAY_NAME)
            .map(|v| v.borrow().l())
            .transpose()?)
    }
}

/// Newtype for Cursor JObjects.
struct Cursor<'local>(JObject<'local>);

impl<'local> Cursor<'local> {
    /// https://developer.android.com/reference/android/database/Cursor#moveToNext()
    fn jni_move_to_next<'other_local>(
        &self,
        env: &mut JNIEnv<'other_local>,
    ) -> anyhow::Result<bool> {
        Ok(env.call_method(&self.0, "moveToNext", "()Z", &[])?.z()?)
    }

    /// https://developer.android.com/reference/android/database/Cursor#getColumnIndex(java.lang.String)
    fn jni_get_column_index<'other_local_1, 'other_local_2>(
        &self,
        env: &mut JNIEnv<'other_local_1>,
        column_name: impl AsRef<JObject<'other_local_2>>,
    ) -> anyhow::Result<Option<jint>> {
        let column_index = env
            .call_method(
                &self.0,
                "getColumnIndex",
                "(Ljava/lang/String;)I",
                &[JValue::Object(column_name.as_ref())],
            )?
            .i()?;

        if column_index == -1 {
            Ok(None)
        } else {
            Ok(Some(column_index))
        }
    }

    /// https://developer.android.com/reference/android/database/Cursor#getString(int)
    fn jni_get_string<'other_local>(
        &self,
        env: &mut JNIEnv<'other_local>,
        column_index: jint,
    ) -> anyhow::Result<JObject<'other_local>> {
        Ok(env
            .call_method(
                &self.0,
                "getString",
                "(I)Ljava/lang/String;",
                &[JValue::Int(column_index)],
            )?
            .l()?)
    }

    /// https://developer.android.com/reference/android/database/Cursor#close()
    fn jni_close<'other_local>(self, env: &mut JNIEnv<'other_local>) -> anyhow::Result<()> {
        env.call_method(&self.0, "close", "()V", &[])?;
        Ok(())
    }
}

impl<'local> Deref for Cursor<'local> {
    type Target = JObject<'local>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Newtype for Uri JObjects.
struct Uri<'local>(JObject<'local>);

impl<'local> Uri<'local> {
    fn parse(env: &mut JNIEnv<'local>, uri_string: impl Into<JNIString>) -> anyhow::Result<Self> {
        let string = env.new_string(uri_string)?;
        Self::jni_parse(env, &string)
    }

    fn to_string<'other_local>(&self, env: &mut JNIEnv<'other_local>) -> anyhow::Result<String> {
        let string = self.jni_to_string(env)?;
        Ok(env.get_string(&string.into())?.into())
    }

    fn new_local_ref<'other_local>(
        &'other_local self,
        env: &mut JNIEnv<'local>,
    ) -> anyhow::Result<Self> {
        Ok(Self(env.new_local_ref(&self.0)?))
    }

    /// https://developer.android.com/reference/android/net/Uri#parse(java.lang.String)
    fn jni_parse<'other_local>(
        env: &mut JNIEnv<'local>,
        uri_string: &JObject<'other_local>,
    ) -> anyhow::Result<Self> {
        let uri = env
            .call_static_method(
                "android/net/Uri",
                "parse",
                "(Ljava/lang/String;)Landroid/net/Uri;",
                &[JValue::Object(uri_string)],
            )?
            .l()?;
        anyhow::ensure!(!uri.is_null(), "Uri#parse returned null");
        Ok(Self(uri))
    }

    /// https://developer.android.com/reference/android/net/Uri#toString()
    fn jni_to_string<'other_local>(
        &self,
        env: &mut JNIEnv<'other_local>,
    ) -> anyhow::Result<JObject<'other_local>> {
        let string = env
            .call_method(&self.0, "toString", "()Ljava/lang/String;", &[])?
            .l()?;
        anyhow::ensure!(!string.is_null(), "Uri#toString returned null");
        Ok(string)
    }
}

impl<'local> Deref for Uri<'local> {
    type Target = JObject<'local>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Newtype for document IDs wrapping owned or borrowed String JObjects.
///
/// TODO: is the enum the best way to do this? would 2 types + Deref or something be better?
enum DocumentId<'local> {
    Owned(JObject<'local>),
    Borrowed(&'local JObject<'local>),
}

impl<'local> Deref for DocumentId<'local> {
    type Target = JObject<'local>;

    fn deref(&self) -> &Self::Target {
        match self {
            DocumentId::Owned(x) => x,
            DocumentId::Borrowed(x) => x,
        }
    }
}

struct DocumentsContract;

impl DocumentsContract {
    /// https://developer.android.com/reference/android/provider/DocumentsContract.Document#COLUMN_DISPLAY_NAME
    const COLUMN_DISPLAY_NAME: &str = "_display_name";
    /// https://developer.android.com/reference/android/provider/DocumentsContract.Document#COLUMN_DOCUMENT_ID
    const COLUMN_DOCUMENT_ID: &str = "document_id";

    /// Build URI representing the children of the target directory URI.
    ///
    /// `tree_uri` should be the subtree URI returned by ACTION_OPEN_DOCUMENT_TREE.
    ///
    /// `parent_document_id` is the target document and must be a descendant of `tree_uri`.
    ///
    /// https://developer.android.com/reference/android/provider/DocumentsContract#buildChildDocumentsUriUsingTree(android.net.Uri,%20java.lang.String)
    fn jni_build_child_documents_uri_using_tree<'local, 'other_local_1, 'other_local_2>(
        env: &mut JNIEnv<'local>,
        tree_uri: &Uri<'other_local_1>,
        parent_document_id: &DocumentId<'other_local_2>,
    ) -> anyhow::Result<Uri<'local>> {
        let uri = env
            .call_static_method(
                "android/provider/DocumentsContract",
                "buildChildDocumentsUriUsingTree",
                "(Landroid/net/Uri;Ljava/lang/String;)Landroid/net/Uri;",
                &[JValue::Object(tree_uri), JValue::Object(parent_document_id)],
            )?
            .l()?;
        anyhow::ensure!(
            !uri.is_null(),
            "DocumentsContract#buildChildDocumentsUriUsingTree returned null"
        );
        Ok(Uri(uri))
    }

    /// Build URI representing the document with the given ID.
    ///
    /// `tree_uri` should be the subtree URI returned by ACTION_OPEN_DOCUMENT_TREE.
    ///
    /// `document_id` is the target document and must be a descendant of `tree_uri`.
    ///
    /// https://developer.android.com/reference/android/provider/DocumentsContract#buildDocumentUriUsingTree(android.net.Uri,%20java.lang.String)
    fn jni_build_document_uri_using_tree<'local, 'other_local_1, 'other_local_2>(
        env: &mut JNIEnv<'local>,
        tree_uri: &Uri<'other_local_1>,
        document_id: &DocumentId<'other_local_2>,
    ) -> anyhow::Result<Uri<'local>> {
        let uri = env
            .call_static_method(
                "android/provider/DocumentsContract",
                "buildDocumentUriUsingTree",
                "(Landroid/net/Uri;Ljava/lang/String;)Landroid/net/Uri;",
                &[
                    JValue::Object(tree_uri),
                    JValue::Object(document_id.as_ref()),
                ],
            )?
            .l()?;
        anyhow::ensure!(
            !uri.is_null(),
            "DocumentsContract#buildDocumentUriUsingTree returned null"
        );
        Ok(Uri(uri))
    }

    /// Create a new document in the given directory.
    ///
    /// Takes the URI of the parent directory document, the mime type of the new document,
    /// and the display name for the new document.
    ///
    /// https://developer.android.com/reference/android/provider/DocumentsContract#createDocument(android.content.ContentResolver,%20android.net.Uri,%20java.lang.String,%20java.lang.String)
    fn jni_create_document<
        'local,
        'other_local_1,
        'other_local_2,
        'other_local_3,
        'other_local_4,
    >(
        env: &mut JNIEnv<'local>,
        content_resolver: &ContentResolver<'other_local_1>,
        parent_document_uri: &Uri<'other_local_2>,
        mime_type: &JObject<'other_local_3>,
        display_name: &JObject<'other_local_4>,
    ) -> anyhow::Result<Uri<'local>> {
        let uri = env
            .call_static_method(
                "android/provider/DocumentsContract",
                "createDocument",
                "(Landroid/content/ContentResolver;Landroid/net/Uri;Ljava/lang/String;Ljava/lang/String;)Landroid/net/Uri;",
                &[
                    JValue::Object(content_resolver),
                    JValue::Object(parent_document_uri),
                    JValue::Object(mime_type),
                    JValue::Object(display_name),
                ],
            )?
            .l()?;
        anyhow::ensure!(
            !uri.is_null(),
            "ContentResolver#createDocument returned null"
        );
        Ok(Uri(uri))
    }

    /// Extract the `Document.COLUMN_DOCUMENT_ID` from the given URI.
    ///
    /// This should be a document URI.
    ///
    /// https://developer.android.com/reference/android/provider/DocumentsContract#getDocumentId(android.net.Uri)
    fn jni_get_document_id<'local>(
        env: &mut JNIEnv<'local>,
        document_uri: &Uri,
    ) -> anyhow::Result<DocumentId<'local>> {
        let string = env
            .call_static_method(
                "android/provider/DocumentsContract",
                "getDocumentId",
                "(Landroid/net/Uri;)Ljava/lang/String;",
                &[JValue::Object(document_uri)],
            )?
            .l()?;
        anyhow::ensure!(
            !string.is_null(),
            "DocumentsContract#getDocumentId returned null"
        );
        Ok(DocumentId::Owned(string))
    }

    /// Extract the `Document.COLUMN_DOCUMENT_ID` from the given URI.
    ///
    /// This should be a tree URI, such as one returned by ACTION_OPEN_DOCUMENT_TREE.
    /// If called with a document URI, it seems to return the document ID of the containing tree.
    ///
    /// https://developer.android.com/reference/android/provider/DocumentsContract#getTreeDocumentId(android.net.Uri)
    fn jni_get_tree_document_id<'local>(
        env: &mut JNIEnv<'local>,
        document_uri: &Uri,
    ) -> anyhow::Result<DocumentId<'local>> {
        let string = env
            .call_static_method(
                "android/provider/DocumentsContract",
                "getTreeDocumentId",
                "(Landroid/net/Uri;)Ljava/lang/String;",
                &[JValue::Object(document_uri)],
            )?
            .l()?;
        anyhow::ensure!(
            !string.is_null(),
            "DocumentsContract#getTreeDocumentId returned null"
        );
        Ok(DocumentId::Owned(string))
    }
}
