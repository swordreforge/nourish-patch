use compositor_support_library_pam_worker_base::{current_username, AuthResponse, PamWorker, SubmitError};
use smithay::reexports::calloop::{LoopHandle, RegistrationToken};
use compositor_orchestration_core_state_base::Loop;

pub fn make_pam(loop_handle: &LoopHandle<Loop>) -> Option<(PamWorker, RegistrationToken)> {
    let username = current_username().ok_or_else(|| std::io::Error::other("no current user"));
    let username = match username {
        Ok(username) => username,
        Err(err) => {
            error!("PAM worker creation error: {err}");
            return None;
        }
    };
    let mut pam = PamWorker::spawn("y5-lock", username);
    let mut pam = match pam {
        Ok(pam) => pam,
        Err(err) => {
            error!("PAM worker creation error: {err}");
            return None;
        }
    };

    let ping_source = pam.take_ping_source().unwrap_or_else(|| abort!("ping source"));

    let registration = loop_handle.insert_source(ping_source, move |_event, _, state| {
        let responses = state.inner.worlds.get_mut(compositor_y5_lock_system_base::base::LOCK_WORLD).storage_mut().get_mut(&compositor_y5_lock_system_base::base::LOCK_MUT).pam.as_ref().unwrap().0.drain_responses();
        let responses = responses.first().clone();

        let Some(responses) = responses else {
            return;
        };

        match responses {
            AuthResponse::Success => {
                info!("Auth valid");
                crate::interface::unlock(state);
                // state.dispatch_lock_message(LockMessage::AuthSucceeded);
                // state.unlock_session();
            }
            AuthResponse::Failure(reason) => {
                error!("Auth invalid: {reason}");
                crate::interface::unlock_fail(state);
            }
            AuthResponse::Error(msg) => {
                error!("PAM error: {msg}");
                crate::interface::unlock_fail(state);
                // state.dispatch_lock_message(LockMessage::AuthFailed(
                //     "Authentication unavailable".into(),
                // ));
            }
        }
    });
    let registration = match registration {
        Ok(reg) => reg,
        Err(err) => {
            error!("PAM registration error: {err}");
            return None;
        }
    };

    info!("PAM Created");
    Some((pam, registration))
}
