//! Activation-token comparison helpers shared by matchers.

pub mod token;

pub use token::{candidate_token_from_env, token_matches, ACTIVATION_TOKEN_ENV, STARTUP_ID_ENV};
