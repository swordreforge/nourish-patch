//! End-to-end smoke test of the persistence framework with a dummy storage:
//! transformation (B) drops a non-persisted field, versioning/migration (A) lifts
//! an old file, and rehydration (C) writes the reconstructed value through the
//! slot. No real compositor state is touched.

use compositor_support_system_persist_envelope_base::base as envelope;
use compositor_support_system_persist_trait_base::base::Persist;
use compositor_support_system_persist_trait_base::{y5_persist, PersistError, SnapshotOutcome};
use compositor_support_system_storage_slot_base::base::Storage;
use compositor_support_system_storage_token_base::y5_storage;

#[derive(Clone, PartialEq, Debug)]
struct Camera {
    x: f64,
    y: f64,
    zoom: f64,
    /// Live-only scratch — must NOT survive a round-trip (not part of `Persisted`).
    scratch: u32,
}

y5_storage!(pub CAMERA, CAMERA_MUT: Camera);

#[derive(serde::Serialize, serde::Deserialize, PartialEq)]
struct CamSnap {
    x: f64,
    y: f64,
    zoom: f64,
}

struct CamPersist;

impl Persist for CamPersist {
    type Live = Camera;
    type Persisted = CamSnap;
    const KEY: &'static str = "camera";
    const CURRENT_VERSION: u32 = 2;

    fn to_persisted(c: &Camera) -> CamSnap {
        CamSnap { x: c.x, y: c.y, zoom: c.zoom }
    }
    fn from_persisted(s: CamSnap) -> Camera {
        // Reconstruct: scratch is reset, not restored.
        Camera { x: s.x, y: s.y, zoom: s.zoom, scratch: 0 }
    }
    fn migrate(from: u32, mut v: serde_json::Value) -> Result<serde_json::Value, PersistError> {
        match from {
            1 => {
                // v1 had no `zoom`; default it.
                v["zoom"] = serde_json::json!(1.0);
                Ok(v)
            }
            2 => Ok(v),
            _ => Err(PersistError::unknown_version("camera", from, 2)),
        }
    }
}

y5_persist!(CAMERA_ENTRY, CamPersist, CAMERA, CAMERA_MUT);

#[test]
fn roundtrip_through_envelope_drops_non_persisted_field() {
    let mut a = Storage::new();
    a.insert(&CAMERA, Camera { x: 1.0, y: 2.0, zoom: 3.0, scratch: 99 });

    // Snapshot (vs no cache = changed) -> envelope -> bytes -> envelope -> data.
    let (data, cache) = match (CAMERA_ENTRY.snapshot)(&a, None) {
        SnapshotOutcome::Changed { bytes, cache } => (bytes, cache),
        _ => panic!("first snapshot must be Changed"),
    };
    // Snapshotting again against the cache is Unchanged (PartialEq, no write).
    assert!(matches!(
        (CAMERA_ENTRY.snapshot)(&a, Some(cache.as_ref())),
        SnapshotOutcome::Unchanged
    ));
    let file = envelope::wrap(CAMERA_ENTRY.key, CAMERA_ENTRY.version, &data).unwrap();
    let (version, data2) = envelope::unwrap(&file).unwrap();
    assert_eq!(version, 2);

    // Rehydrate into a fresh storage seeded with a different default.
    let mut b = Storage::new();
    b.insert(&CAMERA, Camera { x: 0.0, y: 0.0, zoom: 0.0, scratch: 7 });
    (CAMERA_ENTRY.rehydrate)(&mut b, &data2, version).unwrap();

    let got = b.get(&CAMERA);
    assert_eq!((got.x, got.y, got.zoom), (1.0, 2.0, 3.0));
    assert_eq!(got.scratch, 0, "scratch must not be persisted/restored");
}

#[test]
fn migrates_v1_file_to_current() {
    let mut s = Storage::new();
    s.insert(&CAMERA, Camera { x: 0.0, y: 0.0, zoom: 0.0, scratch: 0 });

    // A v1 payload: no `zoom`.
    let v1 = serde_json::to_vec(&serde_json::json!({ "x": 5.0, "y": 6.0 })).unwrap();
    (CAMERA_ENTRY.rehydrate)(&mut s, &v1, 1).unwrap();

    let got = s.get(&CAMERA);
    assert_eq!((got.x, got.y, got.zoom), (5.0, 6.0, 1.0));
}

#[test]
fn rejects_unknown_version() {
    let mut s = Storage::new();
    s.insert(&CAMERA, Camera { x: 0.0, y: 0.0, zoom: 0.0, scratch: 0 });
    let data = serde_json::to_vec(&serde_json::json!({ "x": 1.0, "y": 1.0, "zoom": 1.0 })).unwrap();
    assert!((CAMERA_ENTRY.rehydrate)(&mut s, &data, 99).is_err());
}
