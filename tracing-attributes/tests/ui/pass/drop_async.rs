#[tracing::instrument(drop)]
async fn drop_plain() {}

#[tracing::instrument(drop(level = tracing::Level::DEBUG))]
async fn drop_level() {}

#[tracing::instrument(drop, ret, err)]
async fn drop_with_ret_err() -> Result<(), &'static str> {
    Ok(())
}

#[tracing::instrument(drop(after_first_poll))]
async fn drop_after_first_poll(arg: &str) {
    let _ = arg.len();
}

fn main() {
    let _ = drop_plain();
    let _ = drop_level();
    let _ = drop_with_ret_err();
    let _ = drop_after_first_poll("arg");
}
