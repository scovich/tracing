#[tracing::instrument(drop)]
fn sync_fn() {}

#[tracing::instrument(drop)]
fn returns_ready_future() -> impl std::future::Future<Output = ()> {
    std::future::ready(())
}

fn main() {}
