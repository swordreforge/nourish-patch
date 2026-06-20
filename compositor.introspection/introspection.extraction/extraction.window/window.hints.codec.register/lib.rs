//! Registers JSON codecs for every built-in hint attribute, so the persistence
//! layer can serialize/rebuild a full `InferredHints`/`Preferences` set by name.
//! `DetectedHandler` (a `HandlerId`, not plain serde) is encoded as a stable
//! handler name and resolved back via the built-in handler ids.
pub mod register;
