use compositor_support_system_persist_entry_base::base::PersistError;

/// Opt a storage into persistence (impl beside the live type). Three jobs: (A)
/// versioning/migration (`CURRENT_VERSION` + `migrate`), (B) transformation
/// (`to_persisted`/`from_persisted`; the on-disk form is NOT the live type), (C)
/// rehydration (`from_persisted`). Pure data — no `Storage`, no IO; `y5_persist!`
/// wraps an impl into a type-erased [`PersistEntry`].
pub trait Persist: 'static {
    /// The live in-memory type held in a storage slot.
    type Live: 'static;
    /// The serializable persisted form. NOT `Live`. `PartialEq` drives change
    /// detection at the `buffer()` boundary; `Send + 'static` allow caching it.
    type Persisted: serde::Serialize + serde::de::DeserializeOwned + PartialEq + Send + 'static;

    /// On-disk identity; the file is `<KEY>.json`. Unique per process.
    const KEY: &'static str;
    /// Bump whenever `Persisted`'s shape changes.
    const CURRENT_VERSION: u32;

    /// (B) live -> persisted DTO.
    fn to_persisted(live: &Self::Live) -> Self::Persisted;
    /// (C) persisted DTO -> live (reconstruct; may merge into a default).
    fn from_persisted(persisted: Self::Persisted) -> Self::Live;

    /// (A) migrate an OLDER version's JSON up to `CURRENT_VERSION` so it can
    /// deserialize as `Persisted`. Operates on `serde_json::Value` so each step
    /// is a field rename/default, never a typed historical struct. The default
    /// accepts only the current version.
    fn migrate(
        from_version: u32,
        json: serde_json::Value,
    ) -> Result<serde_json::Value, PersistError> {
        if from_version == Self::CURRENT_VERSION {
            Ok(json)
        } else {
            Err(PersistError::unknown_version(
                Self::KEY,
                from_version,
                Self::CURRENT_VERSION,
            ))
        }
    }
}

/// Build a `pub static <NAME>: PersistEntry` for an `impl Persist` and its slot
/// tokens. Invoke in the crate that owns the slot's (crate-private) write token:
///
/// ```ignore
/// y5_persist!(CAMERA_PERSIST, CameraPersist, CAMERA, CAMERA_MUT);
/// // then: fn persist(&self) -> &'static [&'static PersistEntry] { &[&CAMERA_PERSIST] }
/// ```
#[macro_export]
macro_rules! y5_persist {
    ($name:ident, $ty:ty, $read:path, $write:path) => {
        pub static $name: $crate::PersistEntry = $crate::PersistEntry {
            key: <$ty as $crate::base::Persist>::KEY,
            version: <$ty as $crate::base::Persist>::CURRENT_VERSION,
            snapshot: |storage, last| {
                let live = match storage.try_get(&$read) {
                    ::core::option::Option::Some(v) => v,
                    ::core::option::Option::None => return $crate::SnapshotOutcome::Absent,
                };
                let current = <$ty as $crate::base::Persist>::to_persisted(live);
                if let ::core::option::Option::Some(prev) = last
                    .and_then(|a| a.downcast_ref::<<$ty as $crate::base::Persist>::Persisted>())
                {
                    if *prev == current {
                        return $crate::SnapshotOutcome::Unchanged;
                    }
                }
                let bytes = $crate::serde_json::to_vec(&current)
                    .expect("Persisted serializes to JSON");
                $crate::SnapshotOutcome::Changed { bytes, cache: ::std::boxed::Box::new(current) }
            },
            rehydrate: |storage, bytes, from_version| {
                let value: $crate::serde_json::Value =
                    $crate::serde_json::from_slice(bytes)
                        .map_err(|e| $crate::PersistError::parse(e.to_string()))?;
                let value =
                    <$ty as $crate::base::Persist>::migrate(from_version, value)?;
                let persisted = $crate::serde_json::from_value(value)
                    .map_err(|e| $crate::PersistError::parse(e.to_string()))?;
                *storage.get_mut(&$write) =
                    <$ty as $crate::base::Persist>::from_persisted(persisted);
                ::core::result::Result::Ok(())
            },
        };
    };
}
