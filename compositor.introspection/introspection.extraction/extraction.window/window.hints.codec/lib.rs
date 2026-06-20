//! A name-keyed JSON codec registry for type-erased hint values, so the whole
//! `InferredHints`/`Preferences` set can be persisted and rebuilt without static
//! type info at the call site (the persistence layer only knows attribute names).
pub mod codec;
