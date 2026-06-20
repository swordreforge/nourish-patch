//! EGL fence import (waiting on client acquire points before sampling). Seam:
//! the gles side of syncobj acquire-wait; implicit sync covers Phase 1.

use std::os::unix::io::BorrowedFd;

#[derive(Debug, thiserror::Error)]
pub enum FenceImportError {
    #[error("EGL fence import is a designated seam (implicit sync in Phase 1)")]
    NotPopulated,
}

pub fn import_acquire_fence(_fd: BorrowedFd<'_>) -> Result<(), FenceImportError> {
    Err(FenceImportError::NotPopulated)
}
