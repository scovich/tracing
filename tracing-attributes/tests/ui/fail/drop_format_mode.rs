#[tracing::instrument(drop(Debug))]
async fn drop_debug() {}

#[tracing::instrument(drop(Display))]
async fn drop_display() {}

fn main() {}
