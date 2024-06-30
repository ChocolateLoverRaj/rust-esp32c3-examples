use std::future::Future;
use futures::{FutureExt, Stream, StreamExt};
use async_ui_web::race;
use async_ui_web::shortcut_traits::ShortcutRenderStr;

pub trait StreamRenderExt<F: Future> {
    async fn render(self);
}

impl<F: Future, S: Stream<Item=F> + Unpin> StreamRenderExt<F> for S {
    async fn render(mut self) {
        let mut value = None::<F>;
        loop {
            value = race((
                async {
                    match value {
                        Some(future) => {
                            future.await;
                        }
                        None => {
                            "".render().await;
                        }
                    }
                }.map(|_| None),
                self.next()
            )).await;
        }
    }
}
