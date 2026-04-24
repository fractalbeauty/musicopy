//! Protocol definitions for client and server messages.
//!
//! Messages are (de)serialized using `postcard`, which is not self-describing. In app v12 we
//! changed the ALPN to `musicopy/1` and upraded Iroh to the 0.9x series, which was a breaking
//! change. For future changes, we can detect the protocol version from the ALPN, and continue to
//! support previous protocol versions temporarily. This was not possible for v12 because of the
//! Iroh upgrade.

use crate::library::transcode::TranscodeFormat;
use iroh::EndpointId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A message sent by the server end of a connection on the control stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessageV1 {
    /// Identify the server with a friendly name.
    Identify(String),
    /// Notify the client that the connection has been accepted.
    Accepted,
    /// Inform the client of available files.
    Index(Vec<IndexItem>),
    /// Inform the client of updates to the index.
    IndexUpdate(Vec<IndexUpdateItem>),
    /// Notify the client that the statuses of jobs have changed.
    JobStatus(HashMap<u64, JobStatusItem>),
}

/// An item available for downloading from the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexItem {
    pub endpoint_id: EndpointId,
    pub root: String,
    pub path: String,

    pub file_size: FileSize,
}

/// An update to an item in the index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IndexUpdateItem {
    FileSize {
        endpoint_id: EndpointId,
        root: String,
        path: String,

        file_size: FileSize,
    },
}

/// A job that changed status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobStatusItem {
    Transcoding,
    Ready { file_size: u64 },
    Failed { error: String },
}

/// An unknown, estimated, or actual file size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileSize {
    Unknown,
    Estimated(u64),
    Actual(u64),
}

/// A message sent by the client end of a connection on the control stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessageV1 {
    /// Identify the client with a friendly name.
    Identify {
        name: String,
        /// Transcode format for transcoding, or None to transfer original files.
        transcode_format: Option<TranscodeFormat>,
    },
    /// Request to download files.
    Download(Vec<DownloadItem>),
}

/// An item requested for downloading by the client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadItem {
    pub job_id: u64,

    pub endpoint_id: EndpointId,
    pub root: String,
    pub path: String,
}
