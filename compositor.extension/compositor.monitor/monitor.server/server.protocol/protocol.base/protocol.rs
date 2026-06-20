// protocol_crate/src/protocol.rs
pub mod y5_proto {

    pub mod y5_compositor_unstable_v1 {
        use wayland_server;
        use wayland_server::backend as wayland_backend;
        use wayland_server::protocol::*;

        pub mod __interfaces {
            use wayland_server::backend as wayland_backend;
            use wayland_server::protocol::__interfaces::*;
            wayland_scanner::generate_interfaces!(
                "../../../wayland-protocol/y5_compositor_wayland_unstable_v1.xml"
            );
        }
        use self::__interfaces::*;

        wayland_scanner::generate_server_code!(
            "../../../wayland-protocol/y5_compositor_wayland_unstable_v1.xml"
        );
    }

    pub mod y5_compositor_unstable_client_v1 {
        use wayland_client;
        // use wayland_server::backend as wayland_backend;
        use wayland_client::protocol::*;

        pub mod __interfaces {
            use wayland_client::backend as wayland_backend;
            use wayland_client::protocol::__interfaces::*;
            wayland_scanner::generate_interfaces!(
                "../../../wayland-protocol/y5_compositor_wayland_unstable_v1.xml"
            );
        }
        use self::__interfaces::*;

        wayland_scanner::generate_client_code!(
            "../../../wayland-protocol/y5_compositor_wayland_unstable_v1.xml"
        );
    }
}
