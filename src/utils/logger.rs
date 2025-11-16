use log::{Level, LevelFilter, SetLoggerError};
use log4rs::{
    append::console::ConsoleAppender,
    append::file::FileAppender,
    config::{Appender, Config, Root},
    encode::json::JsonEncoder,
    encode::pattern::PatternEncoder,
    Handle,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::OnceLock;

static LOG_HANDLE: OnceLock<Handle> = OnceLock::new();
static LOG_METRICS: OnceLock<Mutex<LogMetrics>> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogMetrics {
    pub total_logs: u64,
    pub error_count: u64,
    pub warning_count: u64,
    pub info_count: u64,
    pub debug_count: u64,
    pub trace_count: u64,
    pub logs_by_module: HashMap<String, u64>,
    pub logs_by_level: HashMap<String, u64>,
}

impl LogMetrics {
    fn new() -> Self {
        Self {
            total_logs: 0,
            error_count: 0,
            warning_count: 0,
            info_count: 0,
            debug_count: 0,
            trace_count: 0,
            logs_by_module: HashMap::new(),
            logs_by_level: HashMap::new(),
        }
    }
}

pub fn setup_logger() -> Result<(), SetLoggerError> {
    // Create logs directory if it doesn't exist
    let _ = std::fs::create_dir_all("logs");

    // Initialize metrics
    let _ = LOG_METRICS.set(Mutex::new(LogMetrics::new()));

    // Console appender with structured format
    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "{d(%Y-%m-%d %H:%M:%S)} {h({l})} [{T}] {M} - {m}{n}",
        )))
        .build();

    // File appender with detailed format
    let logfile = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "{d(%Y-%m-%d %H:%M:%S%.3f)} {l} [{T}] {M}:{L} - {m}{n}",
        )))
        .build("logs/erbium.log")
        .expect("Failed to create log file");

    // JSON file appender for structured logging
    let json_logfile = FileAppender::builder()
        .encoder(Box::new(JsonEncoder::new()))
        .build("logs/erbium.json")
        .expect("Failed to create JSON log file");

    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .appender(Appender::builder().build("logfile", Box::new(logfile)))
        .appender(Appender::builder().build("json_logfile", Box::new(json_logfile)))
        .build(
            Root::builder()
                .appender("stdout")
                .appender("logfile")
                .appender("json_logfile")
                .build(LevelFilter::Info),
        )
        .expect("Failed to create log configuration");

    let handle = log4rs::init_config(config)?;
    let _ = LOG_HANDLE.set(handle);

    log::info!("Logger initialized successfully with structured logging");
    log::info!("Log files: erbium.log (text), erbium.json (structured)");
    Ok(())
}

pub fn set_log_level(level: LevelFilter) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(handle) = LOG_HANDLE.get() {
        // Create new config with updated log level
        let stdout = ConsoleAppender::builder()
            .encoder(Box::new(PatternEncoder::new(
                "{d(%Y-%m-%d %H:%M:%S)} {h({l})} [{T}] {M} - {m}{n}",
            )))
            .build();

        let logfile = FileAppender::builder()
            .encoder(Box::new(PatternEncoder::new(
                "{d(%Y-%m-%d %H:%M:%S%.3f)} {l} [{T}] {M}:{L} - {m}{n}",
            )))
            .build("logs/erbium.log")
            .expect("Failed to create log file");

        let json_logfile = FileAppender::builder()
            .encoder(Box::new(JsonEncoder::new()))
            .build("logs/erbium.json")
            .expect("Failed to create JSON log file");

        let new_config = Config::builder()
            .appender(Appender::builder().build("stdout", Box::new(stdout)))
            .appender(Appender::builder().build("logfile", Box::new(logfile)))
            .appender(Appender::builder().build("json_logfile", Box::new(json_logfile)))
            .build(
                Root::builder()
                    .appender("stdout")
                    .appender("logfile")
                    .appender("json_logfile")
                    .build(level),
            )
            .expect("Failed to create log configuration");

        handle.set_config(new_config);
        log::info!("Log level changed to: {:?}", level);
    }
    Ok(())
}

/// Get current log metrics
pub fn get_log_metrics() -> Option<LogMetrics> {
    LOG_METRICS
        .get()
        .and_then(|metrics| metrics.lock().ok().map(|m| m.clone()))
}

/// Reset log metrics
pub fn reset_log_metrics() -> Result<(), Box<dyn std::error::Error>> {
    if let Some(metrics) = LOG_METRICS.get() {
        let mut m = metrics
            .lock()
            .map_err(|e| format!("Failed to lock metrics: {}", e))?;
        *m = LogMetrics::new();
        log::info!("Log metrics reset");
    }
    Ok(())
}

/// Log a structured event with additional context
pub fn log_structured(
    level: Level,
    _module: &str,
    message: &str,
    context: Option<HashMap<String, String>>,
) {
    match level {
        Level::Error => {
            if let Some(ctx) = context {
                log::error!("{} | context: {:?}", message, ctx);
            } else {
                log::error!("{}", message);
            }
        }
        Level::Warn => {
            if let Some(ctx) = context {
                log::warn!("{} | context: {:?}", message, ctx);
            } else {
                log::warn!("{}", message);
            }
        }
        Level::Info => {
            if let Some(ctx) = context {
                log::info!("{} | context: {:?}", message, ctx);
            } else {
                log::info!("{}", message);
            }
        }
        Level::Debug => {
            if let Some(ctx) = context {
                log::debug!("{} | context: {:?}", message, ctx);
            } else {
                log::debug!("{}", message);
            }
        }
        Level::Trace => {
            if let Some(ctx) = context {
                log::trace!("{} | context: {:?}", message, ctx);
            } else {
                log::trace!("{}", message);
            }
        }
    }
}

/// Log performance metrics
pub fn log_performance(operation: &str, duration_ms: f64, success: bool) {
    let mut context = HashMap::new();
    context.insert("operation".to_string(), operation.to_string());
    context.insert("duration_ms".to_string(), duration_ms.to_string());
    context.insert("success".to_string(), success.to_string());

    if success {
        log_structured(
            Level::Info,
            "performance",
            &format!(
                "Operation '{}' completed in {:.2}ms",
                operation, duration_ms
            ),
            Some(context),
        );
    } else {
        log_structured(
            Level::Warn,
            "performance",
            &format!(
                "Operation '{}' failed after {:.2}ms",
                operation, duration_ms
            ),
            Some(context),
        );
    }
}

/// Log security events
pub fn log_security_event(event_type: &str, details: &str, severity: &str) {
    let mut context = HashMap::new();
    context.insert("event_type".to_string(), event_type.to_string());
    context.insert("severity".to_string(), severity.to_string());

    match severity {
        "critical" | "high" => log_structured(Level::Error, "security", details, Some(context)),
        "medium" => log_structured(Level::Warn, "security", details, Some(context)),
        _ => log_structured(Level::Info, "security", details, Some(context)),
    }
}

/// Log blockchain events
pub fn log_blockchain_event(
    event_type: &str,
    details: &str,
    block_number: Option<u64>,
    tx_hash: Option<String>,
) {
    let mut context = HashMap::new();
    context.insert("event_type".to_string(), event_type.to_string());
    if let Some(block) = block_number {
        context.insert("block_number".to_string(), block.to_string());
    }
    if let Some(hash) = tx_hash {
        context.insert("transaction_hash".to_string(), hash);
    }

    log_structured(Level::Info, "blockchain", details, Some(context));
}
