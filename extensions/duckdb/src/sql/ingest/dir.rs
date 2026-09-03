//! Ingest directory layout, hardening, checksums, and external-error helpers.

use crate::{DbError, DbResult, DuckDb};
use std::path::{Path, PathBuf};

/// Compute the ingest data directory from a DB path (appends `.ingest`).
///
/// READ-5 / TASK-1867: `DuckDb::open_in_memory` stores the `DuckDB`
/// connection string `:memory:` as its path. Appending `.ingest` to that
/// sentinel yielded the *relative* path `:memory:.ingest`, which the ingest
/// pipeline then created — with staged JSON inside it — in whatever the
/// process working directory happened to be: the user's project root under
/// `ops`, or the crate directory under `cargo test` (where the debris was
/// once committed to this repository). An in-memory handle has no staging
/// area, so it is rejected instead of silently redirected.
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
/// SEC-25 / TASK-0787: the ingest dir holds workspace-root sidecars and
/// JSON staging files that the database trusts on load. On Unix we create
/// it with mode 0o700 (and re-stamp the mode when the dir pre-exists with
/// a more permissive default umask) so a co-tenant on a multi-user system
/// cannot tamper with staged data between collect and load. Non-Unix
/// platforms have no portable mode to stamp, so they get the rejection half
/// only: a pre-existing symlink or reparse point at `data_dir` is refused
/// there too (see [`reject_untrusted_ingest_dir`]), and a fresh dir is
/// created with `create_dir_all` at the platform default.
///
/// SEC-25 / TASK-1000: only the **leaf** ingest dir is hardened to 0o700.
/// `DirBuilder::recursive(true).mode(0o700)` would also stamp every
/// intermediate parent created during the call (e.g. `target/`,
/// `target/ops/`) with 0o700, breaking cargo / build-system convention
/// (target/ is canonically 0o755) and producing an asymmetry between
/// fresh workspaces and ones where `target/` already exists. Create the
/// parents first at the platform-default umask, then build the leaf
/// alone with the restrictive mode.
///
/// SEC-25 / TASK-1857: a pre-existing `data_dir` is *not* trusted. `mkdir`
/// reporting `AlreadyExists` says only that the name is taken — it may be a
/// symlink an attacker planted, in which case a path-based `chmod` would
/// follow it and stamp 0o700 on the attacker's chosen target while every
/// subsequent staged write landed inside it. We therefore `lstat` the path,
/// reject anything that is not a real directory, and then do the mode stamp
/// through an **open handle** whose `(dev, ino)` is checked against the
/// `lstat` result, so the check and the act refer to the same inode. The
/// intermediate parents are created with `create_dir_all` at the platform
/// default umask (TASK-1000) and are deliberately *not* hardened; the
/// co-tenant guarantee this function makes is about the leaf ingest dir
/// only.
///
/// # SEC / TASK-2039 + TASK-2054: closing the verify-then-write TOCTOU window
///
/// Everything above verifies the ingest dir through an **open handle** and
/// then drops it, so on its own it leaves a window: a principal who can create
/// names in the ingest dir's **parent** could swap the verified directory for
/// a symlink between the check and each staged write, and the JSON the
/// database later trusts on load would land wherever they point.
///
/// TASK-2039 weighed two answers and took the cheaper one: [`harden_ingest_parent`]
/// removes the swap *capability*, making the staging parent unwritable to
/// every principal but its owner. TASK-2054 then added the structural half it
/// deferred — [`IngestDir`] keeps the verified descriptor open and every
/// staged write, read, rename and unlink resolves against it via `*at(2)`, so
/// the pipeline no longer re-resolves the directory by name at all.
///
/// The two are complementary, not redundant. Parent hardening is what stops an
/// attacker planting a name *before* [`IngestDir::open`] takes the handle; the
/// anchor is what covers the cases hardening cannot — a shared-writable but
/// sticky staging parent, and an attacker running as the same uid, whom no
/// directory mode binds.
///
/// The parent is tightened by clearing the group/other **write** bits only
/// (`0o775` → `0o755`), not stamped to `0o700`: `target/ops` is conventionally
/// readable, and TASK-1000's rule that intermediate parents keep the platform
/// default still holds for everything above the immediate parent. A parent
/// that is shared-writable but **sticky** (`/tmp`-style) is accepted as is —
/// the sticky bit already forbids other principals renaming or deleting a
/// name they do not own, which is exactly the swap being defended against.
/// If the write bits cannot be cleared (we do not own the directory), staging
/// is refused rather than performed into a directory another principal
/// controls.
///
/// Non-Unix keeps the rejection half only, as above: there is no portable
/// mode to inspect or stamp.
pub(super) fn create_ingest_dir(data_dir: &Path) -> std::io::Result<()> {
    if let Some(parent) = data_dir.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
            #[cfg(unix)]
            harden_ingest_parent(parent)?;
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
            Err(e) => return Err(e),
        }
        harden_existing_ingest_dir(data_dir)
    }
    #[cfg(not(unix))]
    {
        // SEC-25 / TASK-1857: the "a pre-existing `data_dir` is not trusted"
        // rule is not Unix-specific. `create_dir_all` succeeds silently when
        // the name is already taken by a symlink or a directory junction, so
        // without this check every staged write would land wherever the
        // reparse point points. There is no portable mode to stamp, so this
        // branch keeps the *rejection* half of the Unix behaviour and drops
        // only the `fchmod`.
        match reject_untrusted_ingest_dir(data_dir) {
            Ok(Some(_)) => Ok(()),
            Ok(None) => std::fs::create_dir_all(data_dir),
            Err(e) => Err(e),
        }
    }
}

/// `lstat` `data_dir` and refuse anything that is not a real directory.
///
/// Returns `Ok(Some(lstat))` when the path exists and is a plain directory,
/// `Ok(None)` when nothing is there, and an error when the name is taken by
/// a symlink (or, on Windows, any other reparse point — `FileType::is_symlink`
/// covers junctions too), or by a non-directory.
///
/// SEC-25 / TASK-1857: `mkdir` reporting `AlreadyExists` says only that the
/// name is taken. Following whatever is there is the whole attack.
fn reject_untrusted_ingest_dir(data_dir: &Path) -> std::io::Result<Option<std::fs::Metadata>> {
    use std::io::{Error, ErrorKind};

    let lstat = match std::fs::symlink_metadata(data_dir) {
        Ok(m) => m,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
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

/// SEC / TASK-2039: remove the *capability* to swap the verified ingest dir
/// for a symlink, by making its parent directory writable only by its owner.
///
/// See the TASK-2039 section on [`create_ingest_dir`] for why this is done
/// instead of threading a directory handle through the [`crate::Ingestor`]
/// trait. Returns:
///
/// * `Ok(())` when no other principal can create names in `parent` — either
///   the group/other write bits were already clear, or the directory is
///   sticky (a name there cannot be renamed or unlinked by anyone but its
///   owner), or we cleared the bits ourselves.
/// * `Err` when the bits are set, the directory is not sticky, and the
///   `fchmod` fails — typically because the directory belongs to someone
///   else, which is precisely the situation in which staging into it is
///   unsafe.
///
/// The mode is applied through an open handle (`fchmod`), and the handle is
/// confirmed to be a directory first, so a symlink at `parent` cannot have
/// its target chmodded — the same discipline as
/// [`harden_existing_ingest_dir`].
#[cfg(unix)]
fn harden_ingest_parent(parent: &Path) -> std::io::Result<()> {
    use std::io::{Error, ErrorKind};
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    /// Write permission for group and other: the ability to create, rename,
    /// or unlink names inside the directory.
    const SHARED_WRITE: u32 = 0o022;
    /// The sticky bit (`S_ISVTX`), which restricts renaming and unlinking
    /// inside a shared-writable directory to the entry's own owner.
    const STICKY: u32 = 0o1000;

    let handle = std::fs::File::open(parent)?;
    let meta = handle.metadata()?;
    if !meta.is_dir() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!(
                "ingest staging parent {} is not a directory",
                parent.display()
            ),
        ));
    }
    // `PermissionsExt::mode` returns the raw `st_mode`, file-type bits and
    // all; keep only the permission + set-id/sticky bits `fchmod` accepts.
    let mode = meta.permissions().mode() & 0o7777;
    if mode & SHARED_WRITE == 0 {
        return Ok(());
    }
    if mode & STICKY != 0 {
        // The sticky bit binds every principal *except* the directory's own
        // owner, who can clear it, chmod the directory, or replace it
        // wholesale. Accepting on the bit alone would therefore trust an
        // attacker-owned `0o1777` directory exactly as much as `/tmp`.
        let owner = meta.uid();
        if !is_trusted_parent_owner(owner) {
            return Err(Error::new(
                ErrorKind::PermissionDenied,
                format!(
                    "ingest staging parent {} is shared-writable and sticky but owned by uid {owner}, which can clear the sticky bit or replace the directory",
                    parent.display(),
                ),
            ));
        }
        tracing::debug!(
            parent = ?parent.display(),
            mode = format!("{mode:o}"),
            owner,
            "SEC / TASK-2039: ingest staging parent is shared-writable but sticky and trusted-owned; names cannot be swapped by other principals"
        );
        return Ok(());
    }
    handle
        .set_permissions(std::fs::Permissions::from_mode(mode & !SHARED_WRITE))
        .map_err(|e| {
            Error::new(
                e.kind(),
                format!(
                    "ingest staging parent {} is writable by other local principals (mode {mode:o}) and its permissions could not be tightened: {e}",
                    parent.display(),
                ),
            )
        })
}

/// May `owner` be trusted to hold a shared-writable staging parent?
///
/// Only the superuser and ourselves. `/tmp` — root-owned and sticky — is the
/// shape this accepts; a co-tenant's own `0o1777` directory is the shape it
/// must not, because its owner is not bound by the sticky bit they set.
#[cfg(unix)]
fn is_trusted_parent_owner(owner: u32) -> bool {
    // SAFETY: `geteuid` takes no arguments, dereferences nothing, and is
    // defined to always succeed, so there are no preconditions to uphold and
    // no error case to handle.
    let euid = unsafe { libc::geteuid() };
    owner == 0 || owner == euid
}

/// Stamp `0o700` on an ingest dir that already exists on disk, refusing to
/// act on anything that is not a real directory.
///
/// SEC-25 / TASK-1857: `std::fs::set_permissions` is path-based and follows
/// symlinks, so it cannot be used here — a planted symlink would have its
/// *target* chmodded. Instead we `lstat` the path, reject symlinks and
/// non-directories outright, then open a handle and confirm it resolves to
/// the very inode we inspected before applying the mode through that handle
/// (`File::set_permissions` is `fchmod`, not `chmod`).
#[cfg(unix)]
fn harden_existing_ingest_dir(data_dir: &Path) -> std::io::Result<()> {
    use std::io::{Error, ErrorKind};
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    // Shared with the non-Unix branch so the two platforms cannot drift on
    // what counts as an untrusted pre-existing ingest dir.
    let Some(lstat) = reject_untrusted_ingest_dir(data_dir)? else {
        return Err(Error::new(
            ErrorKind::NotFound,
            format!(
                "ingest dir {} vanished before it could be hardened",
                data_dir.display()
            ),
        ));
    };

    let handle = std::fs::File::open(data_dir)?;
    let opened = handle.metadata()?;
    if !opened.is_dir() || opened.dev() != lstat.dev() || opened.ino() != lstat.ino() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!(
                "ingest dir {} changed identity between inspection and open",
                data_dir.display()
            ),
        ));
    }

    handle.set_permissions(std::fs::Permissions::from_mode(0o700))
}

/// SEC-25 / TASK-2054: a **verified, anchored handle** on the ingest staging
/// directory.
///
/// # Why this type exists
///
/// [`create_ingest_dir`] verifies the staging directory through an open handle
/// and then drops it. Before this type existed, `provide_via_ingestor` handed
/// the plain `&Path` on to [`crate::DataIngestor::collect`] /
/// [`crate::DataIngestor::load`] and `sidecar.rs` joined onto it by name, so
/// **every staged write re-resolved the directory by path**. TASK-2039 shrank
/// that window by removing the swap *capability* (see
/// [`harden_ingest_parent`]), but two cases stayed open: a shared-writable but
/// *sticky* staging parent, where the reopen is still by name, and an attacker
/// running as the **same uid**, whom no directory mode binds.
///
/// `IngestDir` closes the structural half. It owns a directory descriptor that
/// was confirmed — by `(dev, ino)` against the `lstat` taken during
/// verification — to be the directory that was hardened, and every staged
/// write, read, rename and unlink goes through `*at(2)` syscalls anchored on
/// that descriptor. Replacing the *name* after the handle is open redirects
/// nothing: the kernel resolves the staged entry relative to the inode we hold,
/// not to the path we were given.
///
/// # What is still resolved by path, and why that is sound
///
/// [`IngestDir::path`] still exists and still hands out a `&Path`, for exactly
/// two uses:
///
/// * `DuckDB`'s `read_json_auto('<path>')` — the embedded engine takes a path
///   string and has no descriptor-passing API, so the *read* of the staged JSON
///   is unavoidably by name.
/// * the `data_sources` provenance row and log breadcrumbs, which record a name
///   for a human to find later.
///
/// Neither is a *write*. The finding this type answers is that a swapped
/// directory captures the data ops stages; a swapped directory on the read side
/// can at worst feed `DuckDB` attacker-chosen JSON, which the workspace-sidecar
/// and checksum checks already treat as untrusted input.
///
/// # Platform
///
/// The anchoring is Unix-only, matching the split the rest of this module
/// already makes: there is no portable `*at` family, so non-Unix targets keep
/// the by-name behaviour together with the symlink/reparse-point rejection in
/// [`reject_untrusted_ingest_dir`].
#[derive(Debug)]
pub struct IngestDir {
    path: PathBuf,
    /// The verified directory descriptor every anchored operation resolves
    /// against. Held open for the whole staging lifetime on purpose — dropping
    /// it is what reopened the window in the first place.
    #[cfg(unix)]
    handle: std::fs::File,
}

impl IngestDir {
    /// Create (and harden) the ingest directory, then open a verified handle on
    /// it.
    ///
    /// The directory is created and hardened by [`create_ingest_dir`], opened
    /// with `O_DIRECTORY | O_NOFOLLOW`, and the opened inode is checked against
    /// a fresh `lstat` so the descriptor is provably the directory that was
    /// just hardened rather than a name that changed underneath us.
    ///
    /// # Errors
    ///
    /// [`DbError::Io`] if the directory cannot be created, hardened, or opened,
    /// or if the name no longer refers to the directory that was verified.
    pub fn open(data_dir: &Path) -> DbResult<Self> {
        create_ingest_dir(data_dir).map_err(DbError::Io)?;
        Self::open_verified(data_dir).map_err(DbError::Io)
    }

    #[cfg(unix)]
    fn open_verified(data_dir: &Path) -> std::io::Result<Self> {
        use std::io::{Error, ErrorKind};
        use std::os::unix::fs::{MetadataExt, OpenOptionsExt};

        let lstat = std::fs::symlink_metadata(data_dir)?;
        let handle = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(data_dir)?;
        let opened = handle.metadata()?;
        if !opened.is_dir() || opened.dev() != lstat.dev() || opened.ino() != lstat.ino() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!(
                    "ingest dir {} changed identity between inspection and open",
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

    /// The directory's path, for `DuckDB`'s path-only `read_json_auto`, the
    /// `data_sources` provenance row, and log breadcrumbs.
    ///
    /// Never use this to open a file for writing — that is precisely the
    /// re-resolution this type exists to remove. Use [`IngestDir::write_atomic`],
    /// [`IngestDir::open_read`], [`IngestDir::rename`], or
    /// [`IngestDir::remove_file`].
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The path a staged entry *would* have, for `read_json_auto` and
    /// provenance. Carries the same "reads and labels only" contract as
    /// [`IngestDir::path`].
    #[must_use]
    pub fn entry_path(&self, name: &str) -> PathBuf {
        self.path.join(name)
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
        self.write_atomic_io(name, bytes).map_err(DbError::Io)
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
        self.open_read_io(name).map_err(DbError::Io)
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
        self.rename_io(from, to).map_err(DbError::Io)
    }

    #[cfg(unix)]
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
        self.remove_file_io(name).map_err(DbError::Io)
    }

    #[cfg(unix)]
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
        checksum_reader(self.open_read(name)?)
    }

    /// SEC-25 / TASK-2067: assert that [`IngestDir::entry_path`] and the
    /// anchor still name the same inode.
    ///
    /// The one staged access this type cannot anchor is the `DuckDB` engine's
    /// own read: `read_json_auto('<path>')` takes a path string and the
    /// embedded engine offers no descriptor-passing API, so that read resolves
    /// the ingest directory by name (see `create_table_from_json_sql`). Call
    /// this immediately before handing the path over: it opens the entry
    /// through the verified descriptor and compares its `(dev, ino)` against
    /// what the *path* resolves to, so a directory swapped between the
    /// anchored write and the engine's read is refused rather than silently
    /// feeding the database an attacker's JSON.
    ///
    /// This **shrinks** the window; it does not close it. The path is still
    /// resolved a second time inside `DuckDB`, and nothing prevents a swap
    /// between this check and that resolution. Closing it needs either a
    /// descriptor-passing read in the engine or staging the JSON somewhere
    /// unreachable by name, neither of which is available here.
    ///
    /// Non-Unix has no `(dev, ino)` pair to compare and no anchored open to
    /// compare it against, so the check is a no-op there — the same split
    /// [`IngestDir::open`] and [`create_ingest_dir`] already make.
    ///
    /// # Errors
    ///
    /// [`DbError::Io`] if `name` is not a single path component, the entry
    /// cannot be opened through the anchor or resolved by path, or the two
    /// resolve to different inodes.
    pub fn verify_entry_identity(&self, name: &str) -> DbResult<()> {
        self.verify_entry_identity_io(name).map_err(DbError::Io)
    }

    #[cfg(unix)]
    fn verify_entry_identity_io(&self, name: &str) -> std::io::Result<()> {
        use std::io::{Error, ErrorKind};
        use std::os::unix::fs::MetadataExt;

        // Both opens can fail with a bare `ENOENT` that names nothing, and
        // this check now runs *before* `read_json_auto` would have reported
        // the missing file itself — so re-attach the entry name, or the
        // operator loses which staged file went missing.
        let named = |e: std::io::Error| {
            Error::new(
                e.kind(),
                format!("staged entry {name:?} in {}: {e}", self.path.display()),
            )
        };
        let anchored = self
            .open_read_io(name)
            .and_then(|f| f.metadata())
            .map_err(named)?;
        // Resolve exactly as the engine will: by path, following symlinks.
        let by_path = std::fs::metadata(self.entry_path(name)).map_err(named)?;
        if anchored.dev() != by_path.dev() || anchored.ino() != by_path.ino() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!(
                    "staged entry {name:?} resolves to a different inode by path than through \
                     the verified ingest directory; refusing to hand the path to DuckDB"
                ),
            ));
        }
        Ok(())
    }

    #[cfg(not(unix))]
    fn verify_entry_identity_io(&self, name: &str) -> std::io::Result<()> {
        Self::check_name(name)
    }
}

/// Default DB path for a workspace root (using default `DataConfig`).
#[must_use]
pub fn default_db_path(workspace_root: &Path) -> PathBuf {
    DuckDb::resolve_path(&ops_core::config::DataConfig::default(), workspace_root)
}

/// Convert a non-IO external error into [`DbError::External`].
///
/// Callers that return `anyhow::Error` (`collect_tokei`, `collect_coverage`,
/// `check_metadata_output`, etc.) should use this instead of the old `io_err`
/// which misleadingly wrapped them as `DbError::Io`.
///
/// SEC-21 (TASK-0862): Display renders via the alternate `{:#}` flag so
/// `anyhow::Context` chains continue to surface end-to-end.
///
/// ERR-2 / TASK-1209: passes the underlying `anyhow::Error` through as
/// `#[source]` instead of flattening it via `format!`, so consumers walking
/// `Error::source()` recover the cause graph (e.g. typed retry decisions).
#[must_use]
pub const fn external_err(e: anyhow::Error) -> DbError {
    DbError::External(e)
}

/// Streaming SHA-256 core behind [`IngestDir::checksum`].
///
/// DEAD-1 / TASK-2066: the path-based `checksum_file` that used to share this
/// core is gone. TASK-2054 moved the pipeline's only two checksum call sites
/// (`SidecarIngestorConfig::persist_record` and `MetadataIngestor::load`) onto
/// the anchored [`IngestDir::checksum`], leaving a public helper whose whole
/// job was the by-path resolution the anchor exists to remove — a standing
/// invitation for a future ingestor to reach for
/// `checksum_file(&dir.entry_path(name))` and silently get the pre-TASK-2054
/// behaviour. The streaming implementation is kept here, reachable only
/// through the anchor.
///
/// Streams in 64 KiB chunks so multi-megabyte ingests (coverage, tokei) do not
/// allocate a full file-sized buffer (PERF-1).
fn checksum_reader<R: std::io::Read>(source: R) -> DbResult<String> {
    use sha2::{Digest, Sha256};
    use std::io::{BufReader, Read};
    let mut reader = BufReader::with_capacity(64 * 1024, source);
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = reader.read(&mut buf).map_err(DbError::Io)?;
        if n == 0 {
            break;
        }
        // A `Read` impl never reports more bytes than the buffer holds; surface a
        // violation as an I/O error instead of panicking on the slice.
        let chunk = buf
            .get(..n)
            .ok_or_else(|| std::io::Error::other("read reported more bytes than the buffer holds"))
            .map_err(DbError::Io)?;
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
        let dir = tmp.path().join("data.duckdb.ingest");
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
        let leaf = tmp.path().join("a/b/data.duckdb.ingest");
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

    /// SEC / TASK-2039: the swap this defends against needs the ability to
    /// create or rename names in the ingest dir's parent. After
    /// `create_ingest_dir`, a shared-writable parent must no longer grant it,
    /// so a symlink cannot be swapped in after verification and no staged
    /// write can be redirected. Starting the parent at 0o777 is the closest
    /// on-disk stand-in for a co-tenant-writable staging area.
    #[cfg(unix)]
    #[test]
    fn create_ingest_dir_removes_swap_capability_from_the_staging_parent() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().expect("tempdir");
        let parent = tmp.path().join("shared");
        std::fs::create_dir(&parent).expect("parent");
        std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o777))
            .expect("make parent shared-writable");

        create_ingest_dir(&parent.join("data.duckdb.ingest")).expect("create");

        let mode = std::fs::metadata(&parent)
            .expect("meta")
            .permissions()
            .mode()
            & 0o7777;
        assert_eq!(
            mode & 0o022,
            0,
            "no other principal may create or rename names in the staging parent; got {mode:o}"
        );
        assert_eq!(
            mode & 0o700,
            0o700,
            "the owner must keep full access to the staging parent; got {mode:o}"
        );
    }

    /// SEC / TASK-2039: a parent that is shared-writable but sticky already
    /// forbids other principals renaming or unlinking a name they do not own,
    /// so its mode is left alone rather than tightened — `ops` must not chmod
    /// a `/tmp`-style directory it happens to stage under.
    #[cfg(unix)]
    #[test]
    fn create_ingest_dir_leaves_a_sticky_shared_parent_alone() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().expect("tempdir");
        let parent = tmp.path().join("sticky");
        std::fs::create_dir(&parent).expect("parent");
        std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o1777))
            .expect("make parent sticky and shared-writable");

        create_ingest_dir(&parent.join("data.duckdb.ingest")).expect("create");

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

    /// SEC / TASK-2039: the sticky bit is only worth trusting when the
    /// directory's *owner* is. An owner who is neither root nor us can clear
    /// the bit at will, so the exemption must not extend to them.
    #[cfg(unix)]
    #[test]
    fn only_root_and_ourselves_are_trusted_with_a_sticky_shared_parent() {
        use std::os::unix::fs::MetadataExt;
        let tmp = tempfile::tempdir().expect("tempdir");
        let ours = std::fs::metadata(tmp.path()).expect("meta").uid();

        assert!(
            is_trusted_parent_owner(0),
            "root-owned /tmp must be accepted"
        );
        assert!(
            is_trusted_parent_owner(ours),
            "a parent we own ourselves must be accepted"
        );

        // Any uid that is neither root nor ours. `u32::MAX` is `nobody` on
        // most systems and is never the caller here.
        let stranger = if ours == u32::MAX { 12345 } else { u32::MAX };
        assert!(
            !is_trusted_parent_owner(stranger),
            "a sticky parent owned by uid {stranger} must be refused"
        );
    }

    /// SEC-25 / TASK-1857: a symlink planted at the ingest-dir path must be
    /// rejected, and the symlink's target must keep the mode it had — the
    /// old path-based `set_permissions` chmodded the target to 0o700.
    #[cfg(unix)]
    #[test]
    fn create_ingest_dir_rejects_a_planted_symlink_and_leaves_target_mode() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().expect("tempdir");
        let target = tmp.path().join("attacker-owned");
        std::fs::create_dir(&target).expect("target");
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o755)).expect("mode");

        let link = tmp.path().join("data.duckdb.ingest");
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
        let path = tmp.path().join("data.duckdb.ingest");
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
        let path = PathBuf::from("/home/proj/target/ops/data.duckdb");
        let result = data_dir_for_db(&path).expect("file-backed path");
        assert_eq!(
            result,
            PathBuf::from("/home/proj/target/ops/data.duckdb.ingest")
        );
    }

    /// READ-5 / TASK-1867: the `:memory:` sentinel is a connection string,
    /// not a path. Deriving `:memory:.ingest` from it created a junk
    /// directory in the process working directory.
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
        assert_eq!(path, PathBuf::from("/home/proj/target/ops/data.duckdb"));
    }

    #[test]
    fn external_err_wraps_display_error() {
        let err = external_err(anyhow::anyhow!("test error message"));
        let msg = err.to_string();
        assert!(msg.contains("test error message"));
    }

    /// SEC-21 (TASK-0862): the alternate-format wrapper must preserve the
    /// full anyhow context chain.
    #[test]
    fn external_err_preserves_anyhow_context_chain() {
        use anyhow::Context;
        let leaf = anyhow::Error::msg("leaf cause");
        let chained: anyhow::Error = Err::<(), _>(leaf)
            .context("wrap one")
            .context("wrap two")
            .unwrap_err();
        let err = external_err(chained);
        let msg = err.to_string();
        assert!(msg.contains("wrap two"), "missing outer wrap: {msg}");
        assert!(msg.contains("wrap one"), "missing middle wrap: {msg}");
        assert!(msg.contains("leaf cause"), "missing leaf cause: {msg}");
    }

    /// ERR-2 / TASK-1209: walking `std::error::Error::source()` on the
    /// resulting `DbError::External` recovers the wrapped `anyhow::Error`
    /// chain rather than the previous flattened-string leaf.
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

    /// DEAD-1 / TASK-2066: the checksum tests below drive the anchored
    /// [`IngestDir::checksum`], which is the only surface the streaming
    /// implementation is reachable through now that the path-based
    /// `checksum_file` is gone.
    fn staged_dir(tmp: &tempfile::TempDir) -> IngestDir {
        IngestDir::open(&tmp.path().join("data.duckdb.ingest")).expect("open")
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
    /// was given. The parent stays writable throughout, so the pre-existing
    /// `harden_ingest_parent` defence is deliberately not what is being
    /// exercised.
    #[cfg(unix)]
    #[test]
    fn staged_write_is_not_redirected_by_swapping_the_ingest_dir_name() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let staging = tmp.path().join("data.duckdb.ingest");
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

    /// SEC-25 / TASK-2067 AC #1: the identity re-check accepts a staged entry
    /// that the anchor and the path agree on — the ordinary case, on every
    /// load.
    #[test]
    fn entry_identity_holds_for_an_unmolested_staged_entry() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = staged_dir(&tmp);
        dir.write_atomic("staged.json", b"[]").expect("stage");
        dir.verify_entry_identity("staged.json")
            .expect("path and anchor must agree");
    }

    /// SEC-25 / TASK-2067 AC #1: the swap `create_table_from_json_sql` cannot
    /// defend against on its own. The verified directory is renamed aside and
    /// an attacker-controlled directory holding a different `staged.json` is
    /// put at the name `read_json_auto` would resolve — so the path and the
    /// anchor name different inodes, and the check refuses to hand `DuckDB`
    /// the path.
    #[cfg(unix)]
    #[test]
    fn entry_identity_refuses_a_directory_swapped_under_the_anchor() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let staging = tmp.path().join("data.duckdb.ingest");
        let dir = IngestDir::open(&staging).expect("open");
        dir.write_atomic("staged.json", b"[{\"ours\":1}]")
            .expect("stage through the anchor");

        // Swap the verified directory for an attacker's, at the same name.
        let moved_aside = tmp.path().join("moved-aside");
        std::fs::rename(&staging, &moved_aside).expect("move the verified dir aside");
        let attacker = tmp.path().join("attacker-dir");
        std::fs::create_dir(&attacker).expect("create attacker dir");
        std::fs::write(attacker.join("staged.json"), b"[{\"theirs\":1}]").expect("plant");
        std::os::unix::fs::symlink(&attacker, &staging).expect("plant symlink at the ingest path");

        let err = dir
            .verify_entry_identity("staged.json")
            .expect_err("a swapped directory must be refused");
        match err {
            DbError::Io(e) => assert_eq!(e.kind(), std::io::ErrorKind::InvalidInput),
            other => panic!("expected DbError::Io, got {other:?}"),
        }
    }

    /// SEC-25 / TASK-2067: a missing staged entry is refused too — there is
    /// nothing for `read_json_auto` to read, and the anchored open is what
    /// says so.
    #[test]
    fn entry_identity_refuses_a_missing_entry() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = staged_dir(&tmp);
        assert!(dir.verify_entry_identity("never-staged.json").is_err());
    }

    /// SEC-25 / TASK-2054: anchoring is worthless if the *entry name* can walk
    /// out of the directory, so a name that is not a single path component is
    /// refused before it reaches `openat`.
    #[test]
    fn anchored_entry_names_must_be_single_path_components() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = IngestDir::open(&tmp.path().join("data.duckdb.ingest")).expect("open");
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
        let staging = tmp.path().join("data.duckdb.ingest");
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
        let dir = IngestDir::open(&tmp.path().join("data.duckdb.ingest")).expect("open");
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
}
