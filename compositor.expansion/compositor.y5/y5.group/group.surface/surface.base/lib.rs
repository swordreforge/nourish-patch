//! # group_view
//!
//! Iced UI for a *group* surface — the background panel drawn behind a group
//! of windows.
//!
//! The compositor sizes this surface from the bounding box of the group's
//! contained windows, padded 125px on every side plus an additional 125px on
//! top (250px total top). The windows are composited on top of the surface, so
//! the surface is only visible in the padding bands. The 250px top band hosts
//! the group header: a fold arrow and the (editable) group name.
//!
//! Two modes:
//! - **Show**: full padded bounding box. Header sits in the top band; the rest
//!   of the panel is empty space the windows overlay.
//! - **Collapse**: the compositor instead hands back a static 500x250 chip that
//!   shows only the centered header.
//!
//! ## Boundary with the compositor
//!
//! - The compositor constructs [`ui::GroupUi`] with a starting [`mode::Mode`]
//!   and name.
//! - The UI emits [`message::GroupMessage::Collapse`] /
//!   [`message::GroupMessage::Show`] for the compositor to re-size the surface,
//!   and [`message::GroupMessage::Renamed`] once a rename is committed so it can
//!   persist the new name.
//! - The compositor pushes [`message::GroupMessage::SetName`] to refresh the
//!   canonical name externally. (Do not push `Renamed` back in — that is the
//!   UI's outward signal and echoing it creates a rename feedback loop.)
//!
//! Name editing is keyboard-driven: the UI subscribes to keyboard events only
//! while editing (see `GroupUi::subscribe` / `event_process`).

pub mod message;
pub mod mode;
pub mod style;
pub mod ui;
pub mod view;
