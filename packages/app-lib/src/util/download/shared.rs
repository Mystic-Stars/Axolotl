#![allow(dead_code)]
//! Shared download helpers used by both the legacy and XMCL-compatible
//! engines.
//!
//! Route resolution, integrity verification, `.part` handling, atomic
//! rename, and Windows file-lock retry logic will move here as the
//! legacy engine is split out of `fetch.rs`.

pub const SEGMENTED_DOWNLOAD_THRESHOLD: u64 = 512 * 1024;
pub const XMCL_RANGE_CONCURRENCY: usize = 4;
pub const XMCL_BMCL_CONCURRENCY: usize = 16;
pub const XMCL_OTHER_CONCURRENCY: usize = 16;
