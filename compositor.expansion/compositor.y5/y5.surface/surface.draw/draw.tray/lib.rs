#[macro_use]
extern crate compositor_developer_debug_instance_record;

use std::cell::RefCell;
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::utils::{Physical, Rectangle, Size};
use compositor_monitor_compositor_iced_base::IcedSpace;
use compositor_orchestration_core_state_base::Loop;
use compositor_orchestration_draw_layer_base::base::Layer;
use compositor_y5_surface_draw_handle::handle::load;
use compositor_y5_surface_draw_tray_ui::ui::{TrayMessage, TrayUi};

thread_local! {
    static W: RefCell<Option<Watcher>> = const { RefCell::new(None) };
    static H: RefCell<Option<compositor_monitor_compositor_iced_base::HandleId>> = const { RefCell::new(None) };
}

pub fn setup() {
    match Watcher::start() {
        Ok(w) => { info!("tray: ready"); W.with(|c| *c.borrow_mut() = Some(w)); }
        Err(e) => warn!("tray: D-Bus error: {e}"),
    }
}

pub fn per_frame(state: &mut Loop, renderer: &mut GlesRenderer, size: Size<i32, Physical>) {
    if state.inner.surface().registry.is_none() { return; }
    let items = W.with(|w| w.borrow_mut().as_mut().map(|w| w.poll()).unwrap_or_default());
    if items.is_empty() { H.with(|h| *h.borrow_mut() = None); return; }
    let rect = Rectangle::<i32, Physical>::from_loc_and_size((12,12), (size.w/3, 40));
    let ui = TrayUi { items };
    match H.with(|h| *h.borrow()) {
        None => { let h = load(state, renderer, ui, rect, IcedSpace::Screen, Layer::SCENE.bits());
                  H.with(|c| *c.borrow_mut() = Some(h.id)); }
        Some(id) => { if let Some(r) = state.inner.surface_mut().registry.as_mut() {
                        let _ = r.dispatch_message(
                            compositor_monitor_compositor_iced_base::IcedHandle::<TrayUi>::from_id(id),
                            TrayMessage::Sync); }}
    }
}

struct Watcher { conn: zbus::blocking::Connection }
impl Watcher {
    fn start() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self { conn: zbus::blocking::Connection::session()? })
    }
    fn poll(&mut self) -> Vec<compositor_y5_surface_draw_tray_ui::ui::TrayItem> {
        let Ok(p) = zbus::blocking::Proxy::new(&self.conn,
            "org.kde.StatusNotifierWatcher", "/StatusNotifierWatcher",
            "org.kde.StatusNotifierWatcher") else { return vec![] };
        let Ok(svcs) = p.get_property::<Vec<String>>("RegisteredStatusNotifierItems")
            else { return vec![] };
        svcs.iter().map(|s| self.fetch(s)).collect()
    }
    fn fetch(&self, svc: &str) -> compositor_y5_surface_draw_tray_ui::ui::TrayItem {
        let mut item = compositor_y5_surface_draw_tray_ui::ui::TrayItem {
            service: svc.into(), ..Default::default()
        };
        let Ok(p) = zbus::blocking::Proxy::new(&self.conn, svc,
            "/StatusNotifierItem", "org.kde.StatusNotifierItem") else { return item };
        if let Ok(name) = p.get_property::<String>("IconName") { item.icon_name = Some(name); }
        if let Ok(raw) = p.get_property::<Vec<(i32,i32,Vec<u8>)>>("IconPixmap") {
            if let Some((_w,_h,data)) = raw.into_iter().next() { item.icon_pixmap = Some(data); }
        }
        if let Ok(tt) = p.get_property::<(String,Vec<(i32,i32,Vec<u8>)>,String,String)>("ToolTip") {
            item.tooltip = tt.2;
        }
        item
    }
}
