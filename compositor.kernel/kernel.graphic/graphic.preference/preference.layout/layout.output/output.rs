//! Global-space output placement. Backends produce outputs; the compositor
//! places them. Single-output era: everything at the origin.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutputPosition(pub i32, pub i32);

pub fn position_for(_identity: Option<&str>, _index: usize) -> OutputPosition {
    OutputPosition(0, 0)
}
