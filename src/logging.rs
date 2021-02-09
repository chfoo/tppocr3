use std::sync::{Arc, Mutex};

use slog::{Drain, Level, LevelFilter};
use slog_async::{Async, OverflowStrategy};
use slog_scope::{debug, GlobalLoggerGuard};
use slog_term::{FullFormat, TermDecorator};

pub fn set_up_logging() {
    lazy_static::lazy_static! {
        static ref GLOBAL_LOGGER_GUARD: Arc<Mutex<Option<GlobalLoggerGuard>>> = Arc::new(Mutex::new(None));
    }

    let decorator = TermDecorator::new().build();
    let drain = FullFormat::new(decorator)
        .use_utc_timestamp()
        .use_original_order()
        .build()
        .fuse();
    let drain = LevelFilter::new(drain, Level::Debug).fuse();
    let drain = Async::new(drain)
        .chan_size(512)
        .overflow_strategy(OverflowStrategy::Block)
        .build()
        .fuse();

    let logger = slog::Logger::root(drain, slog::o!());
    let guard = slog_scope::set_global_logger(logger);

    let mut global_logger = GLOBAL_LOGGER_GUARD.lock().unwrap();
    *global_logger = Some(guard);

    debug!("logging initialized");
}
