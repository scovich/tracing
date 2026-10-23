// Rewriting `async fn` to `fn -> impl Future<Output = T>` (needed so a
// drop guard can be a capture of an unpolled future) does not capture
// argument lifetimes the way `async fn` does.
#[tracing::instrument(drop)]
async fn my_function(arg: &str) -> usize {
    arg.len()
}

fn main() {
    let _ = my_function("arg");
}
