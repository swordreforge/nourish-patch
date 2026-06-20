use compositor_orchestration_core_state_base::Loop;
use compositor_remote_message_client_base::bind::debug::{
    Request, RequestNumeric, Response, ResponseNumeric,
};
use compositor_remote_message_client_base::bind::navigator::{Travel, TravelResponse};
use compositor_remote_message_client_base::bind::selection::{
    FitAspect, FitAspectResponse, Layout, LayoutResponse,
};
use compositor_remote_message_client_base::{Message, Service};

pub struct Handle {}

// In cases where reply needs to be separated from the event, return None from the individual handlers and use the reply from the enum
pub fn execute(_loop: &mut Loop, message: Message) {
    match message.Value {
        Service::Selection(a) => a.execute(&mut Handle {}, _loop),
        Service::Navigator(a) => a.execute(&mut Handle {}, _loop),
        Service::Debug(a) => a.execute(&mut Handle {}, _loop),
    }
}

impl compositor_remote_message_client_base::NavigatorService<Loop> for Handle {
    fn travel(&mut self, request: Travel, state: &mut Loop) -> TravelResponse {
        compositor_remote_client_handle_navigator::travel(request, state)
    }
}

impl compositor_remote_message_client_base::SelectionService<Loop> for Handle {
    fn layout(&mut self, request: Layout, state: &mut Loop) -> LayoutResponse {
        compositor_remote_client_handle_selection::layout(request, state)
    }

    fn fit_aspect(&mut self, request: FitAspect, state: &mut Loop) -> FitAspectResponse {
        compositor_remote_client_handle_aspect::fit_aspect(request, state)
    }
}

impl compositor_remote_message_client_base::DebugService<Loop> for Handle {
    fn debug(&mut self, request: Request, state: &mut Loop) -> Response {
        Response {}
    }

    fn numeric(&mut self, request: RequestNumeric, state: &mut Loop) -> ResponseNumeric {
        compositor_remote_client_handle_debug::numeric(request, state)
    }
}
