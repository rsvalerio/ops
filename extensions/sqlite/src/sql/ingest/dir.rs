//! Ingest directory layout, hardening, checksums, and external-error helpers.

use crate::error::io_context;
use crate::{DbError, DbResult, Sqlite};
use std::path::{Path, PathBuf};

/// Compute the ingest data directory from a DB path (appends `.ingest`).
///
/// An in-memory handle has no staging area: its path is the `SQLite`
/// connection string `:memory:`, not a filesystem path, and appending
/// `.ingest` to it would name a relative directory inside the process working
/// directory. It is rejected rather than redirected.
///
/// # Errors
///
/// [`DbError::NotFileBacked`] if `db_path` is the in-memory sentinel.
pub fn data_dir_for_db(db_path: &Path) -> DbResult<PathBuf> {
    if db_path == Path::new(crate::connection::IN_MEMORY_PATH) {
        return Err(DbError::NotFileBacked(db_path.to_path_buf()));
    }
    let mut path = db_path.as_os_str().to_os_string();
    path.push(".ingest");
    Ok(PathBuf::from(path))
}

/// Create the ingest data directory with restrictive permissions.
///
/// The ingest dir holds workspace-root sidecars and JSON staging files that
/// the database trusts on load, so on Unix the leaf is created with mode
/// `0o700` — and re-stamped to it when it already exists — keeping other
/// local users from tampering with staged data between collect and load.
///
/// Only the leaf belongs to ops. Its parent is the database's own directory,
/// which may be the workspace root or any directory the user configured, so
/// the parent chain is created at the platform default when missing and its
/// mode is never changed nor used as a reason to refuse staging. The defence
/// against a principal who can create names in that parent is [`IngestDir`],
/// which verifies and anchors the leaf itself.
///
/// A pre-existing `data_dir` is not trusted: `mkdir` reporting
/// `AlreadyExists` says only that the name is taken. The path is `lstat`ed,
/// anything that is not a real directory is refused, and the mode is stamped
/// through an open handle whose `(dev, ino)` matches the `lstat`, so a planted
/// symlink cannot have its target chmodded. A symlink at the immediate parent
/// is refused the same way, before the leaf is created inside its target.
///
/// Non-Unix platforms have no portable mode to stamp, so they keep the
/// rejection half only: a symlink or reparse point at `data_dir` is refused
/// (see [`reject_untrusted_ingest_dir`]) and a fresh dir is created at the
/// platform default.
///
/// # Errors
///
/// If a directory cannot be created, inspected or restricted, or if
/// `data_dir` or its immediate parent is a symlink or not a directory. Every
/// error names the path it concerns.
pub(super) fn create_ingest_dir(data_dir: &Path) -> std::io::Result<()> {
    if let Some(parent) = data_dir.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| {
                io_context(
                    format!("creating ingest dir parent {}", parent.display()),
                    e,
                )
            })?;
            #[cfg(unix)]
            if reject_untrusted_ingest_dir(parent)?.is_none() {
                return Err(vanished(parent));
            }
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        match std::fs::DirBuilder::new()
            .recursive(false)
            .mode(0o700)
            .create(data_dir)
        {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => {
                return Err(io_context(
                    format!("creating ingest dir {}", data_dir.display()),
                    e,
                ))
            }
        }
        harden_existing_ingest_dir(data_dir)
    }
    #[cfg(not(unix))]
    {
        // `create_dir_all` succeeds silently when the name is already taken
        // by a symlink or a directory junction, so the pre-existing path is
        // inspected first; otherwise every staged write would land wherever
        // the reparse point points.
        match reject_untrusted_ingest_dir(data_dir) {
            Ok(Some(_)) => Ok(()),
            Ok(None) => std::fs::create_dir_all(data_dir)
                .map_err(|e| io_context(format!("creating ingest dir {}", data_dir.display()), e)),
            Err(e) => Err(e),
        }
    }
}

/// The error for a directory that disappeared between two steps.
#[cfg(unix)]
fn vanished(dir: &Path) -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!(
            "ingest dir {} vanished before it could be verified",
            dir.display()
        ),
    )
}

/// The error for a directory whose inode changed between `lstat` and open.
#[cfg(unix)]
fn changed_identity(dir: &Path) -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        format!(
            "ingest dir {} changed identity between inspection and open",
            dir.display()
        ),
    )
}

/// `lstat` `data_dir` and refuse anything that is not a real directory.
///
/// Returns `Ok(Some(lstat))` when the path exists and is a plain directory,
/// `Ok(None)` when nothing is there, and an error when the name is taken by
/// a symlink (or, on Windows, any other reparse point — `FileType::is_symlink`
/// covers junctions too), or by a non-directory. Following whatever already
/// holds the name is the whole attack this refuses.
fn reject_untrusted_ingest_dir(data_dir: &Path) -> std::io::Result<Option<std::fs::Metadata>> {
    use std::io::{Error, ErrorKind};

    let lstat = match std::fs::symlink_metadata(data_dir) {
        Ok(m) => m,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(io_context(
                format!("inspecting ingest dir {}", data_dir.display()),
                e,
            ))
        }
    };
    let file_type = lstat.file_type();
    if file_type.is_symlink() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!(
                "ingest dir {} is a symlink; refusing to stage data through it",
                data_dir.display()
            ),
        ));
    }
    if !file_type.is_dir() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!(
                "ingest dir {} exists but is not a directory",
                data_dir.display()
            ),
        ));
    }
    Ok(Some(lstat))
}

/// The effective uid of this process.
#[cfg(unix)]
#[expect(unsafe_code, reason = "libc::geteuid FFI; see the SAFETY comment")]
fn effective_uid() -> u32 {
    // SAFETY: `geteuid` takes no arguments, dereferences nothing, and is
    // defined to always succeed, so there are no preconditions to uphold and
    // no error case to handle.
    unsafe { libc::geteuid() }
}

/// Stamp `0o700` on an ingest dir that already exists on disk, refusing to
/// act on anything that is not a real directory.
///
/// `std::fs::set_permissions` is path-based and follows symlinks, so a planted
/// symlink would have its *target* chmodded. Instead the path is `lstat`ed,
/// symlinks and non-directories are rejected outright, and the mode is applied
/// through a handle confirmed to be the very inode that was inspected
/// (`File::set_permissions` is `fchmod`, not `chmod`).
#[cfg(unix)]
fn harden_existing_ingest_dir(data_dir: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    // Shared with the non-Unix branch so the two platforms cannot drift on
    // what counts as an untrusted pre-existing ingest dir.
    let Some(lstat) = reject_untrusted_ingest_dir(data_dir)? else {
        return Err(vanished(data_dir));
    };

    let handle = std::fs::File::open(data_dir)
        .map_err(|e| io_context(format!("opening ingest dir {}", data_dir.display()), e))?;
    let opened = handle
        .metadata()
        .map_err(|e| io_context(format!("inspecting ingest dir {}", data_dir.display()), e))?;
    if !opened.is_dir() || opened.dev() != lstat.dev() || opened.ino() != lstat.ino() {
        return Err(changed_identity(data_dir));
    }

    handle
        .set_permissions(std::fs::Permissions::from_mode(0o700))
        .map_err(|e| {
            io_context(
                format!("restricting ingest dir {} to mode 0700", data_dir.display()),
                e,
            )
        })
}

/// A verified, anchored handle on the ingest staging directory.
///
/// Owns a directory descriptor that was confirmed to be the private directory
/// [`create_ingest_dir`] hardened: the same `(dev, ino)` as an `lstat` of the
/// path, owned by this user, and closed to group and other. Every staged
/// write, read, rename and unlink goes through `*at(2)` syscalls anchored on
/// that descriptor, so replacing the directory's *name* after the handle is
/// open redirects nothing — the kernel resolves each staged entry relative to
/// the inode held here, not to the path the handle was opened from.
///
/// The owner and mode check is what makes the anchor independent of the
/// permissions of the directory the database lives in. Another principal who
/// can create names there can swap in a directory of their own before the
/// handle is taken, but cannot make it owned by this user, so the swap is
/// refused instead of staged into. A process running as the same uid is not
/// bound by any directory mode; against it the anchor guarantees only that a
/// swap *after* [`IngestDir::open`] changes nothing.
///
/// # Paths are labels
///
/// [`IngestDir::path`] and [`IngestDir::entry_path`] hand out paths for the
/// `data_sources` provenance row and log breadcrumbs, which record a name for
/// a human to find later. No staged entry is opened by name: staged JSON is
/// read through [`IngestDir::open_read`] and reaches the engine as a bound
/// parameter.
///
/// # Platform
///
/// The anchoring is Unix-only: there is no portable `*at` family, so non-Unix
/// targets resolve entries by name and keep the symlink / reparse-point
/// rejection in [`reject_untrusted_ingest_dir`].
#[derive(Debug)]
pub struct IngestDir {
    path: PathBuf,
    /// The verified directory descriptor every anchored operation resolves
    /// against. Held open for the whole staging lifetime: the guarantee lasts
    /// only as long as the descriptor does.
    #[cfg(unix)]
    handle: std::fs::File,
}

impl IngestDir {
    /// Create (and harden) the ingest directory, then open a verified handle on
    /// it.
    ///
    /// The directory is created and hardened by [`create_ingest_dir`], opened
    /// with `O_DIRECTORY | O_NOFOLLOW`, and the opened inode is checked against
    /// a fresh `lstat`, its owner and its mode, so the descriptor is provably
    /// the private directory that was just hardened rather than a name that
    /// changed in between.
    ///
    /// # Errors
    ///
    /// [`DbError::Io`] if the directory cannot be created, hardened, or opened,
    /// or if the name no longer refers to the directory that was verified.
    pub fn open(data_dir: &Path) -> DbResult<Self> {
        create_ingest_dir(data_dir)?;
        Ok(Self::open_verified(data_dir)?)
    }

    #[cfg(unix)]
    fn open_verified(data_dir: &Path) -> std::io::Result<Self> {
        use std::io::{Error, ErrorKind};
        use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};

        /// Any access for group or other.
        const SHARED_ACCESS: u32 = 0o077;

        let lstat = std::fs::symlink_metadata(data_dir)
            .map_err(|e| io_context(format!("inspecting ingest dir {}", data_dir.display()), e))?;
        let handle = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(data_dir)
            .map_err(|e| io_context(format!("opening ingest dir {}", data_dir.display()), e))?;
        let opened = handle
            .metadata()
            .map_err(|e| io_context(format!("inspecting ingest dir {}", data_dir.display()), e))?;
        if !opened.is_dir() || opened.dev() != lstat.dev() || opened.ino() != lstat.ino() {
            return Err(changed_identity(data_dir));
        }
        // A directory another principal swapped in after hardening passes the
        // identity check above (it is self-consistent), but it cannot be owned
        // by us, and one of ours that was never hardened is not private.
        let owner = opened.uid();
        let mode = opened.permissions().mode() & 0o7777;
        if owner != effective_uid() || mode & SHARED_ACCESS != 0 {
            return Err(Error::new(
                ErrorKind::PermissionDenied,
                format!(
                    "ingest dir {} is not a private directory of this user (owner uid {owner}, mode {mode:o}); refusing to stage data into it",
                    data_dir.display()
                ),
            ));
        }
        Ok(Self {
            path: data_dir.to_path_buf(),
            handle,
        })
    }

    #[cfg(not(unix))]
    fn open_verified(data_dir: &Path) -> std::io::Result<Self> {
        use std::io::{Error, ErrorKind};

        // No `*at` family to anchor on; keep the rejection half so a reparse
        // point at `data_dir` is still refused (same split as
        // `create_ingest_dir`).
        if reject_untrusted_ingest_dir(data_dir)?.is_none() {
            return Err(Error::new(
                ErrorKind::NotFound,
                format!(
                    "ingest dir {} vanished before it could be opened",
                    data_dir.display()
                ),
            ));
        }
        Ok(Self {
            path: data_dir.to_path_buf(),
        })
    }

    /// The directory's path, for the `data_sources` provenance row and log
    /// breadcrumbs.
    ///
    /// Never use this to open a file: resolving the directory by name again is
    /// exactly what the anchor avoids. Use [`IngestDir::write_atomic`],
    /// [`IngestDir::open_read`], [`IngestDir::rename`], or
    /// [`IngestDir::remove_file`].
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The path a staged entry *would* have, for provenance labels only —
    /// never for opening. Carries the same "labels only" contract as
    /// [`IngestDir::path`].
    #[must_use]
    pub fn entry_path(&self, name: &str) -> PathBuf {
        self.path.join(name)
    }

    /// Wrap `source` as a [`DbError::Io`] naming the operation `op` and the
    /// staged entry `name` it acted on.
    ///
    /// Public so ingestor implementations outside this crate (`ops-metadata`'s
    /// `MetadataIngestor`) can wrap anchored-entry failures with the same
    /// message shape and source chain as the walkers in this module.
    #[must_use = "return the constructed error; building it reports nothing"]
    pub fn entry_error(&self, op: &str, name: &str, source: std::io::Error) -> DbError {
        DbError::Io(io_context(
            format!("{op} staged entry {}", self.entry_path(name).display()),
            source,
        ))
    }

    /// Reject a staged entry name that is not a single path component.
    ///
    /// Anchoring is worth nothing if the name itself can escape the directory:
    /// `openat(fd, "../elsewhere")` resolves out of the anchor exactly as a
    /// path join would.
    fn check_name(name: &str) -> std::io::Result<()> {
        use std::io::{Error, ErrorKind};
        let is_component = !name.is_empty()
            && name != "."
            && name != ".."
            && !name.contains('/')
            && !name.contains('\0');
        if is_component {
            Ok(())
        } else {
            Err(Error::new(
                ErrorKind::InvalidInput,
                format!("staged entry name {name:?} is not a single path component"),
            ))
        }
    }

    /// Atomically stage `bytes` as `name` inside the anchored directory.
    ///
    /// The `ops_core::config::atomic_write` contract (sibling temp → fsync →
    /// rename → parent fsync) with every step anchored: the temp is created
    /// with `openat(O_CREAT | O_EXCL | O_NOFOLLOW)` on the verified descriptor
    /// and published with `renameat` on that same descriptor, so neither the
    /// temp nor the destination can be redirected by a name swap.
    ///
    /// # Errors
    ///
    /// [`DbError::Io`] if `name` is not a single path component, or if any of
    /// the create / write / fsync / rename steps fails.
    pub fn write_atomic(&self, name: &str, bytes: &[u8]) -> DbResult<()> {
        self.write_atomic_io(name, bytes)
            .map_err(|e| self.entry_error("writing", name, e))
    }

    #[cfg(unix)]
    fn write_atomic_io(&self, name: &str, bytes: &[u8]) -> std::io::Result<()> {
        use std::io::Write;

        Self::check_name(name)?;
        let tmp_name = Self::tmp_name(name);
        let mut tmp = self.create_exclusive(&tmp_name)?;

        let published = tmp
            .write_all(bytes)
            .and_then(|()| tmp.sync_all())
            .and_then(|()| {
                drop(tmp);
                self.rename_io(&tmp_name, name)
            });
        if published.is_err() {
            // Best-effort: a leaked temp is disk hygiene, not a correctness
            // problem, and the original error is the one worth reporting.
            drop(self.remove_file_io(&tmp_name));
            return published;
        }
        // Persist the directory entry itself, matching `atomic_write`'s
        // `sync_parent_dir`. Best-effort there, best-effort here: filesystems
        // that reject `fsync` on a directory must not fail an otherwise
        // successful stage.
        drop(self.handle.sync_all());
        Ok(())
    }

    #[cfg(not(unix))]
    fn write_atomic_io(&self, name: &str, bytes: &[u8]) -> std::io::Result<()> {
        Self::check_name(name)?;
        ops_core::config::atomic_write(&self.entry_path(name), bytes)
    }

    /// Unique-per-process temp basename for the sibling-temp write. Mirrors
    /// `ops_core::config::edit::build_tmp_basename`'s shape (leading dot,
    /// `.tmp.` infix) so the existing "no leftover temp" assertions and any
    /// operator cleanup script recognise it.
    #[cfg(unix)]
    fn tmp_name(name: &str) -> String {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
        let pid = std::process::id();
        format!(".{name}.tmp.{pid}.{seq}")
    }

    /// `openat(O_CREAT | O_EXCL | O_WRONLY | O_NOFOLLOW)` on the anchor.
    #[cfg(unix)]
    fn create_exclusive(&self, name: &str) -> std::io::Result<std::fs::File> {
        // 0o600: the ingest dir is already 0o700, but a staged file that
        // outlives a crash must not become group/world readable if the
        // directory mode is later loosened.
        const TMP_MODE: libc::c_uint = 0o600;
        self.openat(
            name,
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            TMP_MODE,
        )
    }

    /// Open a staged entry for reading, anchored on the verified descriptor.
    ///
    /// `O_NOFOLLOW` refuses a symlink planted at `name` rather than reading
    /// through it.
    ///
    /// # Errors
    ///
    /// [`DbError::Io`] if `name` is not a single path component or the entry
    /// cannot be opened.
    pub fn open_read(&self, name: &str) -> DbResult<std::fs::File> {
        self.open_read_io(name)
            .map_err(|e| self.entry_error("opening", name, e))
    }

    #[cfg(unix)]
    fn open_read_io(&self, name: &str) -> std::io::Result<std::fs::File> {
        Self::check_name(name)?;
        self.openat(name, libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC, 0)
    }

    #[cfg(not(unix))]
    fn open_read_io(&self, name: &str) -> std::io::Result<std::fs::File> {
        Self::check_name(name)?;
        std::fs::File::open(self.entry_path(name))
    }

    #[cfg(unix)]
    #[expect(
        unsafe_code,
        reason = "libc::openat FFI and fd ownership transfer; each block carries its SAFETY comment"
    )]
    fn openat(
        &self,
        name: &str,
        flags: libc::c_int,
        mode: libc::c_uint,
    ) -> std::io::Result<std::fs::File> {
        use std::os::unix::io::{AsRawFd, FromRawFd};

        let cname = std::ffi::CString::new(name)?;
        // SAFETY: `self.handle` is an open directory descriptor that outlives
        // this call (borrowed for `&self`), `cname` is a NUL-terminated C
        // string that outlives the call, and the variadic `mode` argument is
        // supplied because `flags` may contain `O_CREAT`. `openat` returns a
        // fresh descriptor or -1; nothing is dereferenced.
        let fd = unsafe { libc::openat(self.handle.as_raw_fd(), cname.as_ptr(), flags, mode) };
        if fd < 0 {
            return Err(std::io::Error::last_os_error());
        }
        // SAFETY: `fd` was just returned by a successful `openat`, is not -1,
        // and is not owned by anything else, so `File` may take exclusive
        // ownership of it.
        Ok(unsafe { std::fs::File::from_raw_fd(fd) })
    }

    /// Rename a staged entry within the anchored directory.
    ///
    /// # Errors
    ///
    /// [`DbError::Io`] if either name is not a single path component or the
    /// rename fails.
    pub fn rename(&self, from: &str, to: &str) -> DbResult<()> {
        self.rename_io(from, to).map_err(|e| {
            DbError::Io(io_context(
                format!(
                    "renaming staged entry {} to {to:?}",
                    self.entry_path(from).display()
                ),
                e,
            ))
        })
    }

    #[cfg(unix)]
    #[expect(unsafe_code, reason = "libc::renameat FFI; see the SAFETY comment")]
    fn rename_io(&self, from: &str, to: &str) -> std::io::Result<()> {
        use std::os::unix::io::AsRawFd;

        Self::check_name(from)?;
        Self::check_name(to)?;
        let cfrom = std::ffi::CString::new(from)?;
        let cto = std::ffi::CString::new(to)?;
        let fd = self.handle.as_raw_fd();
        // SAFETY: `fd` is an open directory descriptor borrowed for the call,
        // and both C strings are NUL-terminated and outlive it. `renameat`
        // returns 0 or -1 and dereferences nothing we own.
        let rc = unsafe { libc::renameat(fd, cfrom.as_ptr(), fd, cto.as_ptr()) };
        if rc < 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }

    #[cfg(not(unix))]
    fn rename_io(&self, from: &str, to: &str) -> std::io::Result<()> {
        Self::check_name(from)?;
        Self::check_name(to)?;
        std::fs::rename(self.entry_path(from), self.entry_path(to))
    }

    /// Unlink a staged entry, anchored on the verified descriptor.
    ///
    /// # Errors
    ///
    /// [`DbError::Io`] if `name` is not a single path component or the unlink
    /// fails (a missing entry surfaces as [`std::io::ErrorKind::NotFound`], as
    /// with `std::fs::remove_file`).
    pub fn remove_file(&self, name: &str) -> DbResult<()> {
        self.remove_file_io(name)
            .map_err(|e| self.entry_error("removing", name, e))
    }

    #[cfg(unix)]
    #[expect(unsafe_code, reason = "libc::unlinkat FFI; see the SAFETY comment")]
    fn remove_file_io(&self, name: &str) -> std::io::Result<()> {
        use std::os::unix::io::AsRawFd;

        Self::check_name(name)?;
        let cname = std::ffi::CString::new(name)?;
        // SAFETY: the descriptor is open and borrowed for the call, `cname` is
        // NUL-terminated and outlives it, and the flag argument is 0 (unlink a
        // non-directory). `unlinkat` returns 0 or -1.
        let rc = unsafe { libc::unlinkat(self.handle.as_raw_fd(), cname.as_ptr(), 0) };
        if rc < 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }

    #[cfg(not(unix))]
    fn remove_file_io(&self, name: &str) -> std::io::Result<()> {
        Self::check_name(name)?;
        std::fs::remove_file(self.entry_path(name))
    }

    /// SHA-256 of a staged entry, read through the anchor.
    ///
    /// # Errors
    ///
    /// [`DbError::Io`] if the entry cannot be opened or read.
    pub fn checksum(&self, name: &str) -> DbResult<String> {
        checksum_reader(self.open_read(name)?).map_err(|e| self.entry_error("reading", name, e))
    }
}

/// Default DB path for a workspace root (using default `DataConfig`).
#[must_use]
pub fn default_db_path(workspace_root: &Path) -> PathBuf {
    Sqlite::resolve_path(&ops_core::config::DataConfig::default(), workspace_root)
}

/// Convert a non-IO external error into [`DbError::External`].
///
/// For callers that return `anyhow::Error` (`collect_tokei`,
/// `collect_coverage`, `check_metadata_output`, …): their failures are parse
/// errors, missing tools or timeouts, so reporting them as [`DbError::Io`]
/// would send an operator looking for a filesystem problem.
///
/// The `anyhow::Error` becomes the variant's source, so its whole context
/// chain stays reachable — for chain-walking printers and for callers that
/// downcast to a typed cause.
#[must_use]
pub const fn external_err(e: anyhow::Error) -> DbError {
    DbError::External(e)
}

/// Streaming SHA-256 of `source`, as lowercase hex.
///
/// Private on purpose: [`IngestDir::checksum`] is the only entry point, so a
/// checksum is always taken over a file opened through the anchor and never
/// over a path resolved by name.
///
/// Reads straight into one 64 KiB buffer, so a multi-megabyte ingest
/// (coverage, tokei) is hashed without a file-sized allocation. There is no
/// `BufReader`: it bypasses its own buffer whenever the caller's slice is at
/// least as large, so wrapping `source` would only add a second allocation.
fn checksum_reader<R: std::io::Read>(mut source: R) -> std::io::Result<String> {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = source.read(&mut buf)?;
        if n == 0 {
            break;
        }
        // A `Read` impl never reports more bytes than the buffer holds; surface a
        // violation as an I/O error instead of panicking on the slice.
        let chunk = buf.get(..n).ok_or_else(|| {
            std::io::Error::other("read reported more bytes than the buffer holds")
        })?;
        hasher.update(chunk);
    }
    let digest = hasher.finalize();
    Ok(hex::encode(digest.as_slice()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// SEC-25 / TASK-0787: ingest dir must be 0o700 on Unix on both fresh
    /// create and pre-existing dir paths.
    #[cfg(unix)]
    #[test]
    fn create_ingest_dir_uses_restricted_mode_on_unix() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = tmp.path().join("data.db.ingest");
        create_ingest_dir(&dir).expect("create");
        let mode = std::fs::metadata(&dir).expect("meta").permissions().mode();
        assert_eq!(
            mode & 0o777,
            0o700,
            "fresh-created ingest dir must be 0o700; got {:o}",
            mode & 0o777,
        );
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).expect("relax");
        create_ingest_dir(&dir).expect("recreate");
        let mode = std::fs::metadata(&dir).expect("meta").permissions().mode();
        assert_eq!(
            mode & 0o777,
            0o700,
            "pre-existing ingest dir must be re-stamped to 0o700; got {:o}",
            mode & 0o777,
        );
    }

    /// SEC-25 / TASK-1000: only the leaf ingest dir is 0o700.
    #[cfg(unix)]
    #[test]
    fn create_ingest_dir_does_not_lock_down_intermediate_parents() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().expect("tempdir");
        let leaf = tmp.path().join("a/b/data.db.ingest");
        create_ingest_dir(&leaf).expect("create");

        let leaf_mode = std::fs::metadata(&leaf)
            .expect("leaf meta")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(leaf_mode, 0o700, "leaf must be 0o700; got {leaf_mode:o}");

        for parent in [tmp.path().join("a"), tmp.path().join("a/b")] {
            let mode = std::fs::metadata(&parent)
                .expect("parent meta")
                .permissions()
                .mode()
                & 0o777;
            assert_ne!(
                mode,
                0o700,
                "intermediate parent {} was stamped 0o700; expected umask default",
                parent.display()
            );
        }
    }

    /// SEC-25: the ingest dir's parent is the database's own directory —
    /// possibly the workspace root — so staging must neither change its mode
    /// nor refuse it for being group- or world-writable.
    #[cfg(unix)]
    #[test]
    fn ingest_leaves_a_group_writable_database_directory_untouched() {
        use std::os::unix::fs::PermissionsExt;
        for shared_mode in [0o775, 0o777] {
            let tmp = tempfile::tempdir().expect("tempdir");
            let parent = tmp.path().join("project");
            std::fs::create_dir(&parent).expect("parent");
            std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(shared_mode))
                .expect("make parent shared-writable");

            let dir = IngestDir::open(&parent.join("data.db.ingest"))
                .expect("ingest must not fail under a shared-writable parent");
            dir.write_atomic("staged.json", b"{}").expect("stage");

            let mode = std::fs::metadata(&parent)
                .expect("meta")
                .permissions()
                .mode()
                & 0o7777;
            assert_eq!(
                mode, shared_mode,
                "the database directory must keep its mode; got {mode:o}"
            );
        }
    }

    /// SEC-25: the anchor does not lean on the parent's mode, so it must
    /// itself refuse a directory that is not private — the shape another
    /// principal's swapped-in directory, or one of ours that was never
    /// hardened, would have.
    #[cfg(unix)]
    #[test]
    fn the_anchor_refuses_a_directory_that_is_not_private() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().expect("tempdir");
        let staging = tmp.path().join("data.db.ingest");
        std::fs::create_dir(&staging).expect("create");
        std::fs::set_permissions(&staging, std::fs::Permissions::from_mode(0o755)).expect("loosen");

        let err = IngestDir::open_verified(&staging)
            .expect_err("a group/other-accessible ingest dir must be refused");
        assert_eq!(err.kind(), std::io::ErrorKind::PermissionDenied);
        assert!(
            err.to_string().contains(&staging.display().to_string()),
            "error should name the directory: {err}"
        );
    }

    /// SEC-25: `ops` must not chmod a `/tmp`-style sticky directory it
    /// happens to stage under.
    #[cfg(unix)]
    #[test]
    fn create_ingest_dir_leaves_a_sticky_shared_parent_alone() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().expect("tempdir");
        let parent = tmp.path().join("sticky");
        std::fs::create_dir(&parent).expect("parent");
        std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o1777))
            .expect("make parent sticky and shared-writable");

        create_ingest_dir(&parent.join("data.db.ingest")).expect("create");

        let mode = std::fs::metadata(&parent)
            .expect("meta")
            .permissions()
            .mode()
            & 0o7777;
        assert_eq!(
            mode, 0o1777,
            "a sticky shared parent must keep its mode; got {mode:o}"
        );
    }

    /// SEC-25: a symlink planted at the ingest-dir path must be rejected, and
    /// the symlink's target must keep the mode it had.
    #[cfg(unix)]
    #[test]
    fn create_ingest_dir_rejects_a_planted_symlink_and_leaves_target_mode() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().expect("tempdir");
        let target = tmp.path().join("attacker-owned");
        std::fs::create_dir(&target).expect("target");
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o755)).expect("mode");

        let link = tmp.path().join("data.db.ingest");
        std::os::unix::fs::symlink(&target, &link).expect("symlink");

        let err = create_ingest_dir(&link).expect_err("symlinked ingest dir must be rejected");
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
        assert!(
            err.to_string().contains("symlink"),
            "error should name the symlink: {err}"
        );

        let target_mode = std::fs::metadata(&target)
            .expect("target meta")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(
            target_mode, 0o755,
            "symlink target must keep its mode; got {target_mode:o}"
        );
        assert!(
            std::fs::symlink_metadata(&link)
                .expect("link meta")
                .file_type()
                .is_symlink(),
            "the planted symlink must be left in place, not replaced"
        );
    }

    /// SEC-25: a symlink planted at the staging *parent* must be rejected
    /// before the leaf is created, so the staging area cannot be relocated
    /// into the symlink's target.
    #[cfg(unix)]
    #[test]
    fn create_ingest_dir_rejects_a_symlinked_staging_parent() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().expect("tempdir");
        let target = tmp.path().join("attacker-owned");
        std::fs::create_dir(&target).expect("target");
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o777)).expect("mode");

        let link = tmp.path().join("parent");
        std::os::unix::fs::symlink(&target, &link).expect("symlink");
        let data_dir = link.join("data.db.ingest");

        let err = create_ingest_dir(&data_dir).expect_err("symlinked parent must be rejected");
        assert!(
            err.to_string().contains("symlink"),
            "error should name the symlink: {err}"
        );

        let target_mode = std::fs::metadata(&target)
            .expect("target meta")
            .permissions()
            .mode()
            & 0o7777;
        assert_eq!(
            target_mode, 0o777,
            "the symlink's target must keep its mode; got {target_mode:o}"
        );
        assert!(
            !target.join("data.db.ingest").exists(),
            "no leaf ingest dir may be created inside the target"
        );
        assert!(
            std::fs::symlink_metadata(&link)
                .expect("link meta")
                .file_type()
                .is_symlink(),
            "the planted symlink must be left in place, not replaced"
        );
    }

    /// SEC-25: the pre-existing-path policy is shared by both platform
    /// branches, so pin it directly. The non-Unix branch of
    /// `create_ingest_dir` cannot be exercised on this host, but it calls
    /// exactly this function, so a regression here breaks both.
    #[test]
    fn reject_untrusted_ingest_dir_accepts_only_a_real_directory() {
        let tmp = tempfile::tempdir().expect("tempdir");

        let missing = tmp.path().join("absent.ingest");
        assert!(
            reject_untrusted_ingest_dir(&missing)
                .expect("absent path is not an error")
                .is_none(),
            "an absent path must report 'nothing here', not a rejection"
        );

        let real = tmp.path().join("real.ingest");
        std::fs::create_dir(&real).expect("mkdir");
        assert!(reject_untrusted_ingest_dir(&real)
            .expect("real dir accepted")
            .is_some());

        let file = tmp.path().join("file.ingest");
        std::fs::write(&file, b"x").expect("write");
        let err = reject_untrusted_ingest_dir(&file).expect_err("a file must be rejected");
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
        assert!(
            err.to_string().contains("not a directory"),
            "error should say the path is not a directory: {err}"
        );

        #[cfg(unix)]
        {
            let link = tmp.path().join("link.ingest");
            std::os::unix::fs::symlink(&real, &link).expect("symlink");
            let err = reject_untrusted_ingest_dir(&link).expect_err("a symlink must be rejected");
            assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
            assert!(
                err.to_string().contains("symlink"),
                "error should name the symlink: {err}"
            );
        }
    }

    /// SEC-25 / TASK-1857: a plain file occupying the ingest-dir path is a
    /// hard error rather than something we chmod and write into.
    #[cfg(unix)]
    #[test]
    fn create_ingest_dir_rejects_a_non_directory() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("data.db.ingest");
        std::fs::write(&path, b"not a dir").expect("write");
        let err = create_ingest_dir(&path).expect_err("file at ingest path must be rejected");
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
        assert!(
            err.to_string().contains("not a directory"),
            "error should say the path is not a directory: {err}"
        );
    }

    #[test]
    fn data_dir_for_db_appends_ingest() {
        let path = PathBuf::from("/home/proj/target/ops/data.db");
        let result = data_dir_for_db(&path).expect("file-backed path");
        assert_eq!(
            result,
            PathBuf::from("/home/proj/target/ops/data.db.ingest")
        );
    }

    /// The `:memory:` sentinel is a connection string, not a path; deriving
    /// `:memory:.ingest` from it would name a junk directory in the process
    /// working directory.
    #[test]
    fn data_dir_for_db_rejects_the_in_memory_sentinel() {
        let err = data_dir_for_db(Path::new(":memory:")).expect_err("sentinel must be rejected");
        assert!(
            matches!(err, DbError::NotFileBacked(ref p) if p == Path::new(":memory:")),
            "expected NotFileBacked, got: {err:?}"
        );
    }

    #[test]
    fn default_db_path_uses_target_dir() {
        let root = PathBuf::from("/home/proj");
        let path = default_db_path(&root);
        assert_eq!(path, PathBuf::from("/home/proj/target/ops/data.db"));
    }

    #[test]
    fn external_err_keeps_the_cause_in_the_chain() {
        let err = external_err(anyhow::anyhow!("test error message"));
        let msg = format!("{:#}", anyhow::Error::new(err));
        assert!(msg.contains("test error message"), "got: {msg}");
    }

    /// The whole anyhow context chain survives the wrap, each link once.
    #[test]
    fn external_err_preserves_anyhow_context_chain() {
        use anyhow::Context;
        let leaf = anyhow::Error::msg("leaf cause");
        let chained: anyhow::Error = Err::<(), _>(leaf)
            .context("wrap one")
            .context("wrap two")
            .unwrap_err();
        let err = external_err(chained);
        let msg = format!("{:#}", anyhow::Error::new(err));
        for link in ["wrap two", "wrap one", "leaf cause"] {
            assert_eq!(
                msg.matches(link).count(),
                1,
                "`{link}` must appear exactly once: {msg}"
            );
        }
    }

    /// Walking `std::error::Error::source()` on the resulting
    /// `DbError::External` recovers the wrapped `anyhow::Error` chain.
    #[test]
    fn external_err_preserves_error_source_chain() {
        use anyhow::Context;
        use std::error::Error as _;

        let chained: anyhow::Error = Err::<(), _>(anyhow::Error::msg("leaf cause"))
            .context("wrap one")
            .context("wrap two")
            .unwrap_err();
        let err = external_err(chained);

        // First source = the wrapped anyhow::Error itself; subsequent calls
        // walk the anyhow context chain down to the leaf cause.
        let mut messages = Vec::new();
        let mut current: Option<&dyn std::error::Error> = err.source();
        while let Some(e) = current {
            messages.push(e.to_string());
            current = e.source();
        }
        let joined = messages.join(" | ");
        assert!(
            joined.contains("leaf cause"),
            "expected leaf cause in source chain, got: {joined}"
        );
    }

    /// The checksum tests below drive the anchored [`IngestDir::checksum`],
    /// the only surface the streaming implementation is reachable through.
    fn staged_dir(tmp: &tempfile::TempDir) -> IngestDir {
        IngestDir::open(&tmp.path().join("data.db.ingest")).expect("open")
    }

    #[test]
    fn checksum_returns_sha256_hex() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = staged_dir(&tmp);
        dir.write_atomic("test.json", br#"{"test": "data"}"#)
            .expect("stage");
        let checksum = dir.checksum("test.json").expect("checksum");
        assert_eq!(checksum.len(), 64, "SHA-256 hex should be 64 chars");
        assert!(checksum.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn checksum_fails_when_missing() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = staged_dir(&tmp);
        let result = dir.checksum("nonexistent.json");
        assert!(result.is_err(), "should fail for missing file");
    }

    /// SEC-25 / TASK-2054 (the finding's second acceptance criterion): once the
    /// ingest dir is verified and its handle held, **replacing the directory's
    /// name cannot redirect a staged write** — and nothing here depends on the
    /// staging parent's mode.
    ///
    /// The attack this models is the same-uid one that no directory mode binds
    /// (a compromised build script, another tool in the same session): the test
    /// process itself performs the swap, renaming the verified dir aside and
    /// putting an attacker-controlled directory at the very path the pipeline
    /// was given. The parent stays writable throughout.
    #[cfg(unix)]
    #[test]
    fn staged_write_is_not_redirected_by_swapping_the_ingest_dir_name() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let staging = tmp.path().join("data.db.ingest");
        let dir = IngestDir::open(&staging).expect("open verified ingest dir");

        // Swap: move the verified directory aside and plant an attacker-owned
        // one under the name every by-path write would resolve.
        let moved_aside = tmp.path().join("real-ingest-dir");
        std::fs::rename(&staging, &moved_aside).expect("move the verified dir aside");
        let attacker = tmp.path().join("attacker-dir");
        std::fs::create_dir(&attacker).expect("create attacker dir");
        std::os::unix::fs::symlink(&attacker, &staging).expect("plant symlink at the ingest path");

        dir.write_atomic("staged.json", b"secret")
            .expect("staged write through the anchor");

        assert_eq!(
            std::fs::read(moved_aside.join("staged.json")).expect("read from the verified dir"),
            b"secret",
            "the staged write must land in the directory that was verified"
        );
        assert!(
            !attacker.join("staged.json").exists(),
            "the staged write must not follow the swapped name into the attacker's directory"
        );
        // And the anchored read path agrees: it still sees the file it wrote,
        // not whatever the swapped name now resolves to.
        assert_eq!(
            dir.checksum("staged.json")
                .expect("checksum through the anchor"),
            checksum_reader(
                std::fs::File::open(moved_aside.join("staged.json")).expect("open by path")
            )
            .expect("checksum by path"),
        );
    }

    /// SEC-25 / TASK-2054: anchoring is worthless if the *entry name* can walk
    /// out of the directory, so a name that is not a single path component is
    /// refused before it reaches `openat`.
    #[test]
    fn anchored_entry_names_must_be_single_path_components() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = IngestDir::open(&tmp.path().join("data.db.ingest")).expect("open");
        for bad in ["..", ".", "", "../escape.json", "sub/escape.json"] {
            let err = dir
                .write_atomic(bad, b"x")
                .expect_err("escaping entry name must be refused");
            match err {
                DbError::Io(e) => assert_eq!(
                    e.kind(),
                    std::io::ErrorKind::InvalidInput,
                    "expected InvalidInput for {bad:?}, got {e:?}"
                ),
                other => panic!("expected DbError::Io for {bad:?}, got {other:?}"),
            }
        }
    }

    /// SEC-25 / TASK-2054: `IngestDir::open` refuses a symlink at the ingest
    /// path outright, so the anchor is never taken on an attacker-chosen
    /// directory in the first place.
    #[cfg(unix)]
    #[test]
    fn opening_a_symlinked_ingest_dir_is_refused() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let elsewhere = tmp.path().join("elsewhere");
        std::fs::create_dir(&elsewhere).expect("create target");
        let staging = tmp.path().join("data.db.ingest");
        std::os::unix::fs::symlink(&elsewhere, &staging).expect("plant symlink");

        let err = IngestDir::open(&staging).expect_err("a symlinked ingest dir must be refused");
        assert!(
            matches!(err, DbError::Io(_)),
            "expected DbError::Io, got {err:?}"
        );
    }

    /// SEC-25 / TASK-2054: the anchored rename/unlink pair used by
    /// `cleanup_artifacts` round-trips, and the write is atomic (no `.tmp.`
    /// sibling survives a successful stage).
    #[test]
    fn anchored_write_rename_and_unlink_round_trip() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = IngestDir::open(&tmp.path().join("data.db.ingest")).expect("open");
        dir.write_atomic("data.json", b"payload").expect("write");

        let leftover = std::fs::read_dir(dir.path())
            .expect("readdir")
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .find(|name| name.contains(".tmp."));
        assert!(
            leftover.is_none(),
            "anchored write left a temp: {leftover:?}"
        );

        dir.rename("data.json", "data.json.done").expect("rename");
        assert!(!dir.entry_path("data.json").exists());
        assert!(dir.entry_path("data.json.done").exists());

        dir.remove_file("data.json.done").expect("unlink");
        assert!(!dir.entry_path("data.json.done").exists());

        let err = dir
            .remove_file("data.json.done")
            .expect_err("a second unlink must report NotFound");
        match err {
            DbError::Io(e) => assert_eq!(e.kind(), std::io::ErrorKind::NotFound),
            other => panic!("expected DbError::Io, got {other:?}"),
        }
    }

    #[test]
    fn checksum_streaming_matches_in_memory_for_large_input() {
        use sha2::{Digest, Sha256};
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = staged_dir(&tmp);
        // Same byte sequence as `|i| i % 256`, built without a cast: 200 KiB
        // is an exact multiple of 256, so the cycle ends on a full period.
        let data: Vec<u8> = (0..=u8::MAX).cycle().take(200 * 1024).collect();
        dir.write_atomic("big.bin", &data).expect("stage");

        let streamed = dir.checksum("big.bin").expect("stream");
        let mut hasher = Sha256::new();
        hasher.update(&data);
        let in_memory = hex::encode(hasher.finalize().as_slice());
        assert_eq!(streamed, in_memory);
    }

    #[test]
    fn checksum_is_deterministic() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = staged_dir(&tmp);
        dir.write_atomic("test.json", b"test data").expect("stage");
        let c1 = dir.checksum("test.json").expect("checksum1");
        let c2 = dir.checksum("test.json").expect("checksum2");
        assert_eq!(c1, c2, "checksum should be deterministic");
    }

    /// ERR-13: an IO failure on a staged entry names the entry's path and
    /// keeps the OS error as its source.
    #[test]
    fn staged_entry_io_errors_name_the_path_and_keep_the_os_error() {
        use std::error::Error as _;

        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = staged_dir(&tmp);
        let missing = dir.entry_path("absent.json").display().to_string();

        for err in [
            dir.open_read("absent.json").map(drop),
            dir.checksum("absent.json").map(drop),
            dir.remove_file("absent.json"),
            dir.rename("absent.json", "absent.json.done"),
        ] {
            let DbError::Io(io) = err.expect_err("a missing entry must fail") else {
                panic!("expected DbError::Io");
            };
            assert_eq!(io.kind(), std::io::ErrorKind::NotFound);
            assert!(
                io.to_string().contains(&missing),
                "error must name the staged entry: {io}"
            );
            assert!(
                io.source()
                    .and_then(|s| s.downcast_ref::<std::io::Error>())
                    .is_some(),
                "the OS error must stay reachable as the source: {io:?}"
            );
        }
    }

    /// ERR-13: a failure creating the ingest dir names the directory.
    #[test]
    fn create_ingest_dir_errors_name_the_directory() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let blocker = tmp.path().join("not-a-dir");
        std::fs::write(&blocker, b"x").expect("write");
        let data_dir = blocker.join("nested").join("data.db.ingest");

        let err = create_ingest_dir(&data_dir).expect_err("a file in the parent chain must fail");
        assert!(
            err.to_string().contains(&blocker.display().to_string()),
            "error must name the directory it could not create: {err}"
        );
        assert!(
            std::error::Error::source(&err).is_some(),
            "the OS error must stay reachable as the source: {err:?}"
        );
    }
}
