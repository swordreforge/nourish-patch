use compositor_support_library_pam_worker_base::SubmitError;
use smithay::{
    backend::renderer::gles::GlesRenderer,
    utils::{Physical, Size},
};
use compositor_orchestration_core_state_base::Loop;
use compositor_y5_lock_interface_surface::message::LockMessage;

pub fn handle(
    state: &mut Loop,
    renderer: &mut GlesRenderer,
    size: Size<i32, Physical>,
    message: LockMessage,
) {
    let Some((pam, _)) = state.inner.worlds.get_mut(compositor_y5_lock_system_base::base::LOCK_WORLD).storage_mut().get_mut(&compositor_y5_lock_system_base::base::LOCK_MUT).pam.as_ref() else {
        return;
    };

    match message {
        LockMessage::Attempt(pin) => match pam.try_submit(pin) {
            Ok(()) => {
                info!("pam submitting..");
                // self.lock_surface.mark_submitting();
            }
            Err(SubmitError::Busy) => {
                error!("pam busy.. attempt aborted");
                // self.lock_surface.fail("Please wait...");
            }
            Err(SubmitError::WorkerDead) => {
                error!("pam busy.. no worker");
                // tracing::error!("PAM worker died");
                // self.respawn_pam_worker();
            }
        },
        _ => {
            return;
        }
    }
}
