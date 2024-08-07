use std::time::Duration;

use anyhow::anyhow;
use futures_util::{
    future::{select, Either},
    FutureExt, StreamExt,
};
use tokio::{signal::ctrl_c, time::sleep, try_join};
use zbus::Connection;
use zbus_systemd::login1::ManagerProxy;

pub trait ExternalDeviceManager {
    async fn turn_on(&mut self) -> anyhow::Result<()>;
    async fn turn_off(&mut self) -> anyhow::Result<()>;
    async fn zbus_integration(&mut self) -> anyhow::Result<()> {
        let connection = Connection::system().await?;
        let manager = ManagerProxy::new(&connection).await?;
        let get_fd = || async {
            manager
                .inhibit(
                    "sleep".into(),
                    "External Device Manager".into(),
                    "Turn off TV".into(),
                    "delay".into(),
                )
                .await
        };
        let mut _fd = Some(get_fd().await?);
        let ctrl_c_future = ctrl_c().map(|_| ()).shared();
        try_join!(
            async {
                let mut signal_stream = manager.receive_prepare_for_sleep().await?;
                loop {
                    self.turn_on().await?;
                    match select(Box::pin(signal_stream.next()), ctrl_c_future.clone()).await {
                        Either::Left((prepare_for_sleep, _ctrl_c_future)) => {
                            assert_eq!(
                                prepare_for_sleep.ok_or(anyhow!("No data"))?.args()?.start,
                                true
                            );
                            match select(Box::pin(self.turn_off()), ctrl_c_future.clone()).await {
                                Either::Left(_) => {
                                    _fd = None;
                                    let prepare_for_sleep =
                                        signal_stream.next().await.ok_or(anyhow!("No data"))?;
                                    assert_eq!(prepare_for_sleep.args()?.start, false);
                                    _fd = Some(get_fd().await?);
                                }
                                Either::Right((_, turn_off_future)) => {
                                    turn_off_future.await?;
                                    _fd = None;
                                    break;
                                }
                            }
                        }
                        Either::Right(_) => {
                            self.turn_off().await?;
                            _fd = None;
                            break;
                        }
                    }
                }
                Ok::<_, anyhow::Error>(())
            },
            async {
                ctrl_c_future.clone().await;
                Ok(())
            },
        )?;
        Ok(())
    }
}
