use std::convert::TryFrom;
use std::future::Future;
use std::num::TryFromIntError;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::pin::Pin;
use std::task::{Context, Poll};

use tracing::subscriber::with_default;
use tracing::Level;
use tracing_attributes::instrument;
use tracing_mock::{expect, subscriber};
use tracing_test::block_on_future;

#[instrument(drop)]
async fn drop_async() {}

#[instrument(drop)]
async fn returns_early() {
    return;
}

#[instrument(drop)]
async fn pending_then_ready(polls: usize) {
    PendingTimes(polls).await;
}

#[instrument(drop, ret)]
async fn drop_and_ret() -> i32 {
    42
}

#[instrument(drop, err)]
async fn drop_and_err() -> Result<u8, TryFromIntError> {
    u8::try_from(1234)
}

#[instrument(drop, ret, err)]
async fn drop_and_ok() -> Result<u8, TryFromIntError> {
    u8::try_from(123)
}

#[instrument(target = "my_target", drop)]
async fn drop_with_target() {}

#[instrument(level = "warn", drop)]
async fn drop_inherit_level() {}

#[instrument(drop(level = Level::DEBUG))]
async fn drop_override_level() {}

#[instrument(drop)]
async fn drop_panic() {
    panic!();
}

#[instrument(drop(after_first_poll))]
async fn drop_after_first_poll(arg: &str) {
    PendingTimes(2).await;
    let _ = arg.len();
}

fn expect_dropped(level: Level) -> tracing_mock::event::ExpectedEvent {
    expect::event()
        .with_fields(expect::field("dropped").with_value(&true).only())
        .at_level(level)
}

#[test]
fn dropped_before_first_poll() {
    let span = expect::span().named("drop_async");
    let (subscriber, handle) = subscriber::mock()
        .new_span(span.clone())
        .enter(span.clone())
        .event(expect_dropped(Level::INFO))
        .exit(span.clone())
        .drop_span(span)
        .only()
        .run_with_handle();

    with_default(subscriber, || {
        drop(drop_async());
    });
    handle.assert_finished();
}

#[test]
fn after_first_poll_unpolled_does_not_emit() {
    let (subscriber, handle) = subscriber::mock().only().run_with_handle();

    with_default(subscriber, || {
        drop(drop_after_first_poll("arg"));
    });
    handle.assert_finished();
}

#[test]
fn after_first_poll_pending_emits() {
    let span = expect::span().named("drop_after_first_poll");
    let (subscriber, handle) = subscriber::mock()
        .new_span(span.clone().with_fields(expect::field("arg")))
        .enter(span.clone())
        .exit(span.clone())
        .enter(span.clone())
        .event(expect_dropped(Level::INFO))
        .exit(span.clone())
        .drop_span(span)
        .only()
        .run_with_handle();

    with_default(subscriber, || {
        let mut task = tokio_test::task::spawn(drop_after_first_poll("arg"));
        assert!(task.poll().is_pending());
        drop(task);
    });
    handle.assert_finished();
}

#[test]
fn dropped_after_pending() {
    let span = expect::span().named("pending_then_ready");
    let (subscriber, handle) = subscriber::mock()
        .new_span(span.clone())
        .enter(span.clone())
        .exit(span.clone())
        .enter(span.clone())
        .event(expect_dropped(Level::INFO))
        .exit(span.clone())
        .drop_span(span)
        .only()
        .run_with_handle();

    with_default(subscriber, || {
        let mut task = tokio_test::task::spawn(pending_then_ready(2));
        assert!(task.poll().is_pending());
        drop(task);
    });
    handle.assert_finished();
}

#[test]
fn completed_does_not_emit_drop() {
    let span = expect::span().named("drop_async");
    let (subscriber, handle) = subscriber::mock()
        .new_span(span.clone())
        .enter(span.clone())
        .exit(span.clone())
        // `Instrumented` enters the span again while dropping the inner future.
        .enter(span.clone())
        .exit(span.clone())
        .drop_span(span)
        .only()
        .run_with_handle();

    with_default(subscriber, || {
        block_on_future(drop_async());
    });
    handle.assert_finished();
}

#[test]
fn early_return_does_not_emit_drop() {
    let span = expect::span().named("returns_early");
    let (subscriber, handle) = subscriber::mock()
        .new_span(span.clone())
        .enter(span.clone())
        .exit(span.clone())
        .enter(span.clone())
        .exit(span.clone())
        .drop_span(span)
        .only()
        .run_with_handle();

    with_default(subscriber, || {
        block_on_future(returns_early());
    });
    handle.assert_finished();
}

#[test]
fn completed_with_ret_does_not_emit_drop() {
    let span = expect::span().named("drop_and_ret");
    let (subscriber, handle) = subscriber::mock()
        .new_span(span.clone())
        .enter(span.clone())
        .event(
            expect::event()
                .with_fields(expect::field("return").with_value(&tracing::field::debug(42)))
                .at_level(Level::INFO),
        )
        .exit(span.clone())
        .enter(span.clone())
        .exit(span.clone())
        .drop_span(span)
        .only()
        .run_with_handle();

    with_default(subscriber, || {
        block_on_future(drop_and_ret());
    });
    handle.assert_finished();
}

#[test]
fn completed_with_err_does_not_emit_drop() {
    let span = expect::span().named("drop_and_err");
    let (subscriber, handle) = subscriber::mock()
        .new_span(span.clone())
        .enter(span.clone())
        .event(expect::event().at_level(Level::ERROR))
        .exit(span.clone())
        .enter(span.clone())
        .exit(span.clone())
        .drop_span(span)
        .only()
        .run_with_handle();

    with_default(subscriber, || {
        let _ = block_on_future(drop_and_err());
    });
    handle.assert_finished();
}

#[test]
fn completed_with_ret_and_err_ok_does_not_emit_drop() {
    let span = expect::span().named("drop_and_ok");
    let (subscriber, handle) = subscriber::mock()
        .new_span(span.clone())
        .enter(span.clone())
        .event(
            expect::event()
                .with_fields(expect::field("return").with_value(&tracing::field::debug(123u8)))
                .at_level(Level::INFO),
        )
        .exit(span.clone())
        .enter(span.clone())
        .exit(span.clone())
        .drop_span(span)
        .only()
        .run_with_handle();

    with_default(subscriber, || {
        let _ = block_on_future(drop_and_ok());
    });
    handle.assert_finished();
}

#[test]
fn dropped_before_poll_with_ret_emits_drop_not_ret() {
    let span = expect::span().named("drop_and_ret");
    let (subscriber, handle) = subscriber::mock()
        .new_span(span.clone())
        .enter(span.clone())
        .event(expect_dropped(Level::INFO))
        .exit(span.clone())
        .drop_span(span)
        .only()
        .run_with_handle();

    with_default(subscriber, || {
        drop(drop_and_ret());
    });
    handle.assert_finished();
}

#[test]
fn custom_target() {
    let span = expect::span()
        .named("drop_with_target")
        .with_target("my_target");
    let (subscriber, handle) = subscriber::mock()
        .new_span(span.clone())
        .enter(span.clone())
        .event(expect_dropped(Level::INFO).with_target("my_target"))
        .exit(span.clone())
        .drop_span(span)
        .only()
        .run_with_handle();

    with_default(subscriber, || {
        drop(drop_with_target());
    });
    handle.assert_finished();
}

#[test]
fn inherits_span_level() {
    let span = expect::span()
        .named("drop_inherit_level")
        .at_level(Level::WARN);
    let (subscriber, handle) = subscriber::mock()
        .new_span(span.clone())
        .enter(span.clone())
        .event(expect_dropped(Level::WARN))
        .exit(span.clone())
        .drop_span(span)
        .only()
        .run_with_handle();

    with_default(subscriber, || {
        drop(drop_inherit_level());
    });
    handle.assert_finished();
}

#[test]
fn override_level() {
    let span = expect::span().named("drop_override_level");
    let (subscriber, handle) = subscriber::mock()
        .new_span(span.clone())
        .enter(span.clone())
        .event(expect_dropped(Level::DEBUG))
        .exit(span.clone())
        .drop_span(span)
        .only()
        .run_with_handle();

    with_default(subscriber, || {
        drop(drop_override_level());
    });
    handle.assert_finished();
}

#[test]
fn unwind_does_not_emit_drop() {
    let span = expect::span().named("drop_panic");
    let (subscriber, handle) = subscriber::mock()
        .new_span(span.clone())
        .enter(span.clone())
        // The drop guard checks `thread::panicking()` and stays silent, so no
        // event is expected here. tracing-mock ignores `exit` during unwind
        // (printing a note instead), which is why no `exit` appears either,
        // just the second `enter` from `Instrumented`'s own drop glue.
        .enter(span.clone())
        .drop_span(span)
        .only()
        .run_with_handle();

    with_default(subscriber, || {
        let result = catch_unwind(AssertUnwindSafe(|| block_on_future(drop_panic())));
        assert!(result.is_err());
    });
    handle.assert_finished();
}

#[test]
fn leaked_future_does_not_emit_drop() {
    let span = expect::span().named("drop_async");
    let (subscriber, handle) = subscriber::mock().new_span(span).only().run_with_handle();

    with_default(subscriber, || {
        std::mem::forget(drop_async());
    });
    handle.assert_finished();
}

#[test]
fn async_trait_dropped_before_poll() {
    use async_trait::async_trait;

    #[async_trait]
    trait Foo {
        async fn foo(&self);
    }

    #[derive(Debug)]
    struct FooImpl;

    #[async_trait]
    impl Foo for FooImpl {
        #[instrument(drop)]
        async fn foo(&self) {}
    }

    let span = expect::span().named("foo");
    let (subscriber, handle) = subscriber::mock()
        .new_span(span.clone().with_fields(expect::field("self")))
        .enter(span.clone())
        .event(expect_dropped(Level::INFO))
        .exit(span.clone())
        .drop_span(span)
        .only()
        .run_with_handle();

    with_default(subscriber, || {
        drop(FooImpl.foo());
    });
    handle.assert_finished();
}

// Sync function that returns a future *and* matches the async-trait detector
// (`Box::pin(async move { ... })`). `drop` follows that detector, so this
// compiles and emits on unpolled drop — it is not a source-level `async fn`.
#[instrument(drop)]
fn drop_boxed_async() -> Pin<Box<dyn Future<Output = ()> + Send>> {
    Box::pin(async move {})
}

#[test]
fn async_trait_detector_accepts_sync_fn_returning_async_block() {
    let span = expect::span().named("drop_boxed_async");
    let (subscriber, handle) = subscriber::mock()
        .new_span(span.clone())
        .enter(span.clone())
        .event(expect_dropped(Level::INFO))
        .exit(span.clone())
        .drop_span(span)
        .only()
        .run_with_handle();

    with_default(subscriber, || {
        drop(drop_boxed_async());
    });
    handle.assert_finished();
}

struct PendingTimes(usize);

impl Future for PendingTimes {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.0 == 0 {
            Poll::Ready(())
        } else {
            self.0 -= 1;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}
