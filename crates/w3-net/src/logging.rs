//! 极简 stderr 日志后端，供可执行入口（example / relay bin）初始化。
//!
//! 库代码一律用 `log` crate 输出英文结构化日志；后端由入口统一装配，
//! 避免出现「有 log 调用但无任何输出」导致人工验收无法判断结果。

use std::io::Write;

struct StderrLogger;

impl log::Log for StderrLogger {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        metadata.level() <= log::Level::Info
    }

    fn log(&self, record: &log::Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let mut stderr = std::io::stderr().lock();
        let _ = writeln!(
            stderr,
            "{} target={} {}",
            record.level(),
            record.target(),
            record.args()
        );
    }

    fn flush(&self) {}
}

static LOGGER: StderrLogger = StderrLogger;

/// 初始化日志后端；重复调用安全（`set_logger` 只会成功一次）。
pub fn init_logging() {
    let _ = log::set_logger(&LOGGER);
    log::set_max_level(log::LevelFilter::Info);
}
