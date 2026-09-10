//! Feature-gated erasure trait for the `DuckDb` handle.
//!
//! Kept separate from the registry/schema surface so it carries no database
//! concerns; [`crate::context::Context`] stores the erased handle.

/// Erasure trait for the `DuckDb` handle so that extension.rs does not depend
/// on duckdb types.
///
/// # Downcast contract
///
/// The only concrete type stored behind `Arc<dyn DuckDbHandle>` in production
/// code is `ops_duckdb::DuckDb`.
///
/// **Reborrow as `&dyn DuckDbHandle` first.** The blanket impl below covers
/// every `'static + Send + Sync` type, and `Arc<dyn DuckDbHandle>` is one of
/// them — so wherever this trait is in scope, method resolution on an `Arc`
/// (or `&Arc`) receiver matches the blanket impl *for the smart pointer*
/// before it derefs, and `as_any()` hands back the erased `Arc` instead of the
/// handle. Every downcast from it then returns `None`.
///
/// Whether it misresolves depends on the *importing
/// module*, which is what makes it dangerous. A module that never names
/// `DuckDbHandle` in a `use` has no blanket-impl candidate in scope, so a bare
/// `handle.as_any()` derefs through to the trait object's own method and
/// downcasts correctly — until someone adds the import, at which point every
/// downcast in that module silently starts returning `None` with no compile
/// error and no runtime error. Do not rely on the import list. Downcast call
/// sites should:
///
/// ```text
/// let erased: &dyn DuckDbHandle = handle.as_ref();
/// let db: Option<&ops_duckdb::DuckDb> = erased
///     .as_any()
///     .downcast_ref::<ops_duckdb::DuckDb>();
/// ```
///
/// or use the typed convenience helper `ops_duckdb::get_db` which performs
/// the downcast and returns `Option<&DuckDb>`. New consumers should prefer
/// `get_db` over calling `as_any` directly to avoid coupling on the concrete
/// trait method.
///
/// # Implementing
///
/// A blanket impl provides the canonical `as_any` body for every
/// `'static + Send + Sync` type, so implementers cannot accidentally (or
/// maliciously) return a wrong reference like `&()` that would silently
/// break every downcast — the contract is compile-time-enforced, and
/// implementers do not (and cannot) supply their own `as_any` body. Any
/// `'static + Send + Sync` type automatically satisfies `DuckDbHandle`, no
/// explicit `impl` block required at the call site.
///
/// The `Arc`-receiver misresolution described above is contained at the two
/// places that matter: the mandated reborrow, and `ops_duckdb::get_db` /
/// `try_provide_from_db`, which perform the reborrow for every consumer and
/// carry regression tests that call them with the trait deliberately in
/// scope. New consumers should call those helpers rather than `as_any`
/// directly.
#[cfg(feature = "duckdb")]
pub trait DuckDbHandle: std::any::Any + Send + Sync {
    /// Return the handle as `&dyn Any` so callers can downcast to the
    /// concrete type. The blanket impl below supplies the canonical
    /// body (`self`); see trait-level docs for the supported concrete
    /// type and the preferred typed accessor.
    fn as_any(&self) -> &dyn std::any::Any;
}

#[cfg(feature = "duckdb")]
impl<T: std::any::Any + Send + Sync> DuckDbHandle for T {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
