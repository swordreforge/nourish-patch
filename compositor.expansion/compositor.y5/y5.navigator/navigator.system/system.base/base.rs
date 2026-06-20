use compositor_support_system_buffer_token_base::y5_buffer;
use compositor_support_system_trait_system_base::base::{BufferCx, System, SystemCx, WorldBuilder};
use compositor_support_system_world_frame_base::base::FrameTick;
use compositor_y5_navigator_lock_state::state::NavigatorLock;
use compositor_y5_navigator_state_base::state::{Machine, NavRequest, NavigatorOutput, State, NAVIGATOR, NAVIGATOR_MUT, NAV_REQUEST};
use compositor_y5_navigator_tick_base::tick::travel_tick;
use compositor_y5_navigator_tick_warp::warp::apply_warp;
use std::any::Any;

enum NavCmd {
    SetState(State),
    Output(Option<NavigatorOutput>),
    // Externally-requested transitions (rim computes the target, announces NAV_REQUEST).
    Set(State),
    Lock(NavigatorLock),
    Unlock,
}
y5_buffer!(NAV_BUF: NavCmd);

/// Owns the navigator slot and runs the travel/lock easing per tick: reads
/// camera state + KernelData (pointer/screen), writes eased values into its
/// own slot's `output` (the camera system pulls them next in update order)
/// and the pointer-warp intent (the frame driver applies it).
#[derive(Default)]
pub struct NavigatorSystem;

impl System for NavigatorSystem {
    fn name(&self) -> &'static str {
        "navigator"
    }

    fn register(&mut self, builder: &mut WorldBuilder) {
        builder.storage.insert(&NAVIGATOR, Machine::default());
        builder.receive(&NAV_REQUEST, Self::on_request);
    }

    fn update(&mut self, cx: &mut SystemCx, _tick: &FrameTick) {
        match cx.storage.get(&NAVIGATOR).state() {
            State::Idle => {
                if cx.storage.get(&NAVIGATOR).output.is_some() {
                    cx.write(&NAV_BUF, NavCmd::Output(None));
                }
            }
            State::Travel(travel) => {
                let travel = travel.clone();
                let (next, output, warp) = travel_tick(cx, travel);
                cx.write(&NAV_BUF, NavCmd::Output(output));
                apply_warp(cx, warp);
                cx.write(&NAV_BUF, NavCmd::SetState(next));
            }
            State::Lock(lock) => {
                // Lock mode reuses travel easing for its pending travel; it
                // never transitions out by itself (unlock() does).
                let mut lock = lock.clone();
                let (output, warp) = match lock.pending_travel.take() {
                    Some(travel) => {
                        let (next, output, warp) = travel_tick(cx, travel);
                        if let State::Travel(updated) = next {
                            lock.pending_travel = Some(updated);
                        }
                        (output, warp)
                    }
                    None => (None, None),
                };
                cx.write(&NAV_BUF, NavCmd::Output(output));
                apply_warp(cx, warp);
                cx.write(&NAV_BUF, NavCmd::SetState(State::Lock(lock)));
            }
        }
    }

    fn buffer(&mut self, cx: &mut BufferCx, message: Box<dyn Any>) {
        let machine = cx.storage.get_mut(&NAVIGATOR_MUT);
        match *message.downcast::<NavCmd>().expect("navigator buffer type") {
            // force_set: Lock-state transitions come from this system itself.
            NavCmd::SetState(state) => machine.force_set(state),
            NavCmd::Output(output) => machine.output = output,
            // Externally-requested transitions use the normal (lock-respecting) API.
            NavCmd::Set(state) => machine.set(state),
            NavCmd::Lock(lock) => machine.lock(lock),
            NavCmd::Unlock => machine.unlock(),
        }
    }
}

impl NavigatorSystem {
    /// Channel listener: announced request -> self-buffer write (only mutation path).
    fn on_request(&mut self, cx: &mut SystemCx, req: &NavRequest) {
        let cmd = match req.clone() {
            NavRequest::Set(state) => NavCmd::Set(state),
            NavRequest::Lock(lock) => NavCmd::Lock(lock),
            NavRequest::Unlock => NavCmd::Unlock,
        };
        cx.write(&NAV_BUF, cmd);
    }
}
