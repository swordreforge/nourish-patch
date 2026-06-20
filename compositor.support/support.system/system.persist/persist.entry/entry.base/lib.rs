// The type-erased persistence handle. Deliberately serde-free: the heavy
// serialization lives in the generated `snapshot`/`rehydrate` fns (in the owning
// crate, via `y5_persist!`), so this crate — which the `System` trait depends on —
// stays dependency-light.
pub mod base;
