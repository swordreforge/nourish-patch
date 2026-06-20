use std::path::PathBuf;
// overlay/src/grpc_client.rs
use std::sync::Arc;
use tokio::net::UnixStream;
use tokio::runtime::Handle;
use tokio::sync::OnceCell;
use tonic::transport::{Channel, Endpoint, Uri};
use tower::service_fn;
use compositor_remote_message_client_base::bind::debug::debug_client::DebugClient;
use compositor_remote_message_client_base::bind::navigator::navigator_client::NavigatorClient;
use compositor_remote_message_client_base::bind::selection::selection_client::SelectionClient;
use compositor_remote_message_client_base::bind::{debug, navigator, selection};

#[derive(Clone)]
pub struct GrpcClient {
    inner: Arc<Inner>,
}

struct Inner {
    socket_path: PathBuf,
    handle: Handle,
    channel: OnceCell<Channel>,
}

impl GrpcClient {
    pub fn new(socket_path: impl Into<PathBuf>, handle: Handle) -> Self {
        Self {
            inner: Arc::new(Inner {
                socket_path: socket_path.into(),
                handle,
                channel: OnceCell::new(),
            }),
        }
    }

    async fn channel(&self) -> Result<Channel, tonic::transport::Error> {
        self.inner
            .channel
            .get_or_try_init(|| async {
                let path = self.inner.socket_path.clone();
                Endpoint::try_from("http://[::]")?
                    .connect_with_connector(service_fn(move |_: Uri| {
                        let path = path.clone();
                        async move {
                            let stream = UnixStream::connect(path).await?;
                            Ok::<_, std::io::Error>(hyper_util::rt::TokioIo::new(stream))
                        }
                    }))
                    .await
            })
            .await
            .cloned()
    }

    pub async fn navigator(&self) -> Result<NavigatorClient<Channel>, tonic::transport::Error> {
        Ok(NavigatorClient::new(self.channel().await?))
    }

    pub async fn selection(&self) -> Result<SelectionClient<Channel>, tonic::transport::Error> {
        Ok(SelectionClient::new(self.channel().await?))
    }
    pub async fn debug(&self) -> Result<DebugClient<Channel>, tonic::transport::Error> {
        Ok(DebugClient::new(self.channel().await?))
    }
}

impl GrpcClient {
    pub fn zoom_reset(&self) {
        let this = self.clone();
        self.inner.handle.spawn(async move {
            let req = navigator::Travel {
                action: Some(navigator::travel::Action::Reset(
                    navigator::zoom_reset::Request {},
                )),
            };

            match this.navigator().await {
                Ok(mut client) => {
                    if let Err(e) = client.travel(req).await {
                        error!("travel failed: e={e:?}");
                    }
                }
                Err(e) => error!("travel() failed: e={e:?}"),
            }
        });
    }

    pub fn view_directional(&self) {
        // let this = self.clone();
        // self.inner.handle.spawn(async move {
        //     let req = navigator::Travel {
        //         action: Some(navigator::travel::Action::ViewDirection(
        //             navigator::view_direction::Request {},
        //         )),
        //     };

        //     match this.navigator().await {
        //         Ok(mut client) => {
        //             if let Err(e) = client.travel(req).await {
        //                 tracing::error!(?e, "travel failed");
        //             }
        //         }
        //         Err(e) => tracing::error!(?e, "travel() failed"),
        //     }
        // });
    }

    pub fn debug_numeric(&self, number: u32) {
        let this = self.clone();
        self.inner.handle.spawn(async move {
            let req = debug::RequestNumeric { number };

            match this.debug().await {
                Ok(mut client) => {
                    if let Err(e) = client.numeric(req).await {
                        error!("dbg failed: e={e:?}");
                    }
                }
                Err(e) => error!("dbg() failed: e={e:?}"),
            }
        });
    }

    pub fn selection_layout(&self, action: Vec<selection::Action>) {
        let this = self.clone();
        self.inner.handle.spawn(async move {
            let req = selection::Layout { action };

            match this.selection().await {
                Ok(mut client) => {
                    if let Err(e) = client.layout(req).await {
                        error!("dbg failed: e={e:?}");
                    }
                }
                Err(e) => error!("dbg() failed: e={e:?}"),
            }
        });
    }

    pub fn scale_to_fit(&self, action: selection::FitAspect) {
        let this = self.clone();
        self.inner.handle.spawn(async move {
            match this.selection().await {
                Ok(mut client) => {
                    if let Err(e) = client.fit_aspect(action).await {
                        error!("dbg failed: e={e:?}");
                    }
                }
                Err(e) => error!("dbg() failed: e={e:?}"),
            }
        });
    }
}
