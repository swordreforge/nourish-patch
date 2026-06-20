use std::path::PathBuf;
// overlay/src/grpc_client.rs
use std::sync::Arc;
use tokio::net::UnixStream;
use tokio::runtime::Handle;
use tokio::sync::OnceCell;
use tonic::transport::{Channel, Endpoint, Uri};
use tower::service_fn;

use crate::bind::bind::navigator::{self, navigator_client::NavigatorClient};

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
                        println!("travel failed {:?}", e);
                    }
                }
                Err(e) => {
                    println!("travel() failed {:?}", e)
                }
            }
        });
    }

    pub fn view_directional(&self, direction: navigator::view_direction::Direction) {
        let this = self.clone();
        self.inner.handle.spawn(async move {
            let req = navigator::Travel {
                action: Some(navigator::travel::Action::ViewDirection(
                    navigator::view_direction::Request {
                        alternative: true,
                        direction: Some(direction),
                    },
                )),
            };

            match this.navigator().await {
                Ok(mut client) => {
                    if let Err(e) = client.travel(req).await {
                        println!("travel failed {:?}", e);
                    }
                }
                Err(e) => println!("travel() failed {:?}", e),
            }
        });
    }
}
