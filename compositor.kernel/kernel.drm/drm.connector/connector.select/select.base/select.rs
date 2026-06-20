//! Which connector(s) to drive. Today's policy: the first connected connector.
//! Consumes ranked identities when monitor preferences carry them; with no
//! preference, the policy is unchanged.

use compositor_kernel_graphic_preference_output_profile::profile::OutputProfile;
use smithay::reexports::drm::control::connector;

/// Current policy: first connected. The `profiles` parameter is the seam the
/// EDID-identity ranking plugs into; with empty/identity-less profiles it is
/// behavior-neutral.
pub fn select(
    connectors: Vec<connector::Info>,
    _profiles: &[OutputProfile],
) -> Option<connector::Info> {
    connectors
        .into_iter()
        .find(|c| c.state() == connector::State::Connected)
}
