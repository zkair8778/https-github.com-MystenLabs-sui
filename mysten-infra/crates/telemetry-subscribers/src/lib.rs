// Copyright (c) 2022, Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! # Telemetry-subscribers
//!
//! This is a library for common telemetry functionality, especially subscribers for [Tokio tracing](https://github.com/tokio-rs/tracing)
//! libraries.  Here we simply package many common subscribers, such as writing trace data to Jaeger, distributed tracing,
//! common logs and metrics destinations, etc.  into a easy to configure common package.  There are also
//! some unique layers such as one to automatically create Prometheus latency histograms for spans.
//!
//! We also purposely separate out logging levels from span creation.  This is often needed by production apps
//! as normally it is not desired to log at very high levels, but still desirable to gather sampled span data
//! all the way down to TRACE level spans.
//!
//! Getting started is easy.  In your app:
//!
//! ```rust
//!   use telemetry_subscribers::TelemetryConfig;
//!   let (_guard, _handle) = TelemetryConfig::new("my_app").init();
//! ```
//!
//! It is important to retain the guard until the end of the program.  Assign it in the main fn and keep it,
//! for once it drops then log output will stop.
//!
//! There is a builder API available: just do `TelemetryConfig::new()...` Another convenient initialization method
//! is `TelemetryConfig::new().with_env()` to populate the config from environment vars.
//!
//! You can also run the example and see output in ANSI color:
//!
//! ```bash
//!     cargo run --example easy-init
//! ```
//!
//! ## Features
//! - `jaeger` - this feature is enabled by default as it enables jaeger tracing
//! - `json` - Bunyan formatter - JSON log output, optional
//! - `tokio-console` - [Tokio-console](https://github.com/tokio-rs/console) subscriber, optional
//!
//! ### Stdout vs file output
//!
//! By default, logs (but not spans) are formatted for human readability and output to stdout, with key-value tags at the end of every line.
//! `RUST_LOG` can be configured for custom logging output, including filtering.
//!
//! By setting `log_file` in the config, one can write log output to a daily-rotated file.
//!
//! ### Tracing and span output
//!
//! Detailed span start and end logs can be generated by defining the `json_log_output` config variable.  Note that this causes all output to be in JSON format, which is not as human-readable, so it is not enabled by default.
//! This output can easily be fed to backends such as ElasticSearch for indexing, alerts, aggregation, and analysis.
//!
//! NOTE: JSON output requires the `json` crate feature to be enabled.
//!
//! ### Jaeger (seeing distributed traces)
//!
//! To see nested spans visualized with [Jaeger](https://www.jaegertracing.io), do the following:
//!
//! 1. Run this to get a local Jaeger container: `docker run -d -p6831:6831/udp -p6832:6832/udp -p16686:16686 jaegertracing/all-in-one:latest`
//! 2. Set `enable_jaeger` config setting to true or set `TOKIO_JAEGER` env var
//! 3. Run your app
//! 4. Browse to `http://localhost:16686/` and select the service you configured using `service_name`
//!
//! NOTE: separate spans (which are not nested) are not connected as a single trace for now.
//!
//! Jaeger subscriber is enabled by default but is protected by the jaeger feature flag.  If you'd like to leave
//! out the Jaeger dependencies, you can turn off the default-features in your dependency:
//!
//! ```toml
//!     telemetry = { url = "...", default-features = false }
//! ```
//!
//! ### Automatic Prometheus span latencies
//!
//! Included in this library is a tracing-subscriber layer named `PrometheusSpanLatencyLayer`.  It will create
//! a Prometheus histogram to track latencies for every span in your app, which is super convenient for tracking
//! span performance in production apps.
//!
//! Enabling this layer can only be done programmatically, by passing in a Prometheus registry to `TelemetryConfig`.
//!
//! ### Span levels vs log levels
//!
//! What spans are included for Jaeger output, automatic span latencies, etc.?  These are controlled by
//! the `span_level` config attribute, or the `TS_SPAN_LEVEL` environment variable.  Note that this is
//! separate from `RUST_LOG`, so that you can separately control the logging verbosity from the level of
//! spans that are to be recorded and traced.
//!
//! ### Live async inspection / Tokio Console
//!
//! [Tokio-console](https://github.com/tokio-rs/console) is an awesome CLI tool designed to analyze and help debug Rust apps using Tokio, in real time!  It relies on a special subscriber.
//!
//! 1. Build your app using a special flag: `RUSTFLAGS="--cfg tokio_unstable" cargo build`
//! 2. Enable the `tokio-console` feature for this crate.
//! 2. Set the `tokio_console` config setting when running your app (or set TOKIO_CONSOLE env var if using config `with_env()` method)
//! 3. Clone the console repo and `cargo run` to launch the console
//!
//! NOTE: setting tokio TRACE logs is NOT necessary.  It says that in the docs but there's no need to change Tokio logging levels at all.  The console subscriber has a special filter enabled taking care of that.
//!
//! By default, Tokio console listens on port 6669.  To change this setting as well as other setting such as
//! the retention policy, please see the [configuration](https://docs.rs/console-subscriber/latest/console_subscriber/struct.Builder.html#configuration) guide.
//!
//! ### Custom panic hook
//!
//! This library installs a custom panic hook which records a log (event) at ERROR level using the tracing
//! crate.  This allows span information from the panic to be properly recorded as well.
//!
//! To exit the process on panic, set the `CRASH_ON_PANIC` environment variable.

use span_latency_prom::PrometheusSpanLatencyLayer;
use std::{
    env,
    io::{stderr, Write},
    str::FromStr,
};
use tracing::metadata::LevelFilter;
use tracing::Level;
use tracing_appender::non_blocking::{NonBlocking, WorkerGuard};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_subscriber::{
    filter,
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    reload,
    util::SubscriberInitExt,
    EnvFilter, Layer, Registry,
};

use crossterm::tty::IsTty;

pub mod span_latency_prom;

/// Alias for a type-erased error type.
pub type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

/// Configuration for different logging/tracing options
/// ===
/// - json_log_output: Output JSON logs to stdout only.
/// - log_file: If defined, write output to a file starting with this name, ex app.log
/// - log_level: error/warn/info/debug/trace, defaults to info
/// - service_name:
#[derive(Default, Clone, Debug)]
pub struct TelemetryConfig {
    /// The name of the service for Jaeger and Bunyan
    pub service_name: String,

    pub enable_jaeger: bool,
    /// Enables Tokio Console debugging on port 6669
    pub tokio_console: bool,
    /// Output JSON logs.
    pub json_log_output: bool,
    /// Write chrome trace output, which can be loaded from chrome://tracing
    pub chrome_trace_output: bool,
    /// If defined, write output to a file starting with this name, ex app.log
    pub log_file: Option<String>,
    /// Log level to set, defaults to info
    pub log_string: Option<String>,
    /// Span level - what level of spans should be created.  Note this is not same as logging level
    /// If set to None, then defaults to INFO
    pub span_level: Option<Level>,
    /// Set a panic hook
    pub panic_hook: bool,
    /// Crash on panic
    pub crash_on_panic: bool,
    /// Optional Prometheus registry - if present, all enabled span latencies are measured
    pub prom_registry: Option<prometheus::Registry>,
}

#[must_use]
#[allow(dead_code)]
pub struct TelemetryGuards {
    worker_guard: WorkerGuard,

    #[cfg(feature = "chrome")]
    chrome_guard: Option<tracing_chrome::FlushGuard>,
}

#[derive(Clone, Debug)]
pub struct FilterHandle(reload::Handle<EnvFilter, Registry>);

impl FilterHandle {
    pub fn update<S: AsRef<str>>(&self, directives: S) -> Result<(), BoxError> {
        let filter = EnvFilter::try_new(directives)?;
        self.0.reload(filter)?;
        Ok(())
    }

    pub fn get(&self) -> Result<String, BoxError> {
        self.0
            .with_current(|filter| filter.to_string())
            .map_err(Into::into)
    }
}

fn get_output(log_file: Option<String>) -> (NonBlocking, WorkerGuard) {
    if let Some(logfile_prefix) = log_file {
        let file_appender = tracing_appender::rolling::daily("", logfile_prefix);
        tracing_appender::non_blocking(file_appender)
    } else {
        tracing_appender::non_blocking(stderr())
    }
}

// NOTE: this function is copied from tracing's panic_hook example
fn set_panic_hook(crash_on_panic: bool) {
    let default_panic_handler = std::panic::take_hook();

    // Set a panic hook that records the panic as a `tracing` event at the
    // `ERROR` verbosity level.
    //
    // If we are currently in a span when the panic occurred, the logged event
    // will include the current span, allowing the context in which the panic
    // occurred to be recorded.
    std::panic::set_hook(Box::new(move |panic| {
        // If the panic has a source location, record it as structured fields.
        if let Some(location) = panic.location() {
            // On nightly Rust, where the `PanicInfo` type also exposes a
            // `message()` method returning just the message, we could record
            // just the message instead of the entire `fmt::Display`
            // implementation, avoiding the duplicated location
            tracing::error!(
                message = %panic,
                panic.file = location.file(),
                panic.line = location.line(),
                panic.column = location.column(),
            );
        } else {
            tracing::error!(message = %panic);
        }

        default_panic_handler(panic);

        // We're panicking so we can't do anything about the flush failing
        let _ = std::io::stderr().flush();
        let _ = std::io::stdout().flush();

        if crash_on_panic {
            // Kill the process
            std::process::exit(12);
        }
    }));
}

impl TelemetryConfig {
    pub fn new(service_name: &str) -> Self {
        Self {
            service_name: service_name.to_owned(),
            enable_jaeger: false,
            tokio_console: false,
            json_log_output: false,
            chrome_trace_output: false,
            log_file: None,
            log_string: None,
            span_level: None,
            panic_hook: true,
            crash_on_panic: false,
            prom_registry: None,
        }
    }

    pub fn with_log_level(mut self, log_string: &str) -> Self {
        self.log_string = Some(log_string.to_owned());
        self
    }

    pub fn with_span_level(mut self, span_level: Level) -> Self {
        self.span_level = Some(span_level);
        self
    }

    pub fn with_log_file(mut self, filename: &str) -> Self {
        self.log_file = Some(filename.to_owned());
        self
    }

    pub fn with_prom_registry(mut self, registry: &prometheus::Registry) -> Self {
        self.prom_registry = Some(registry.clone());
        self
    }

    pub fn with_env(mut self) -> Self {
        if env::var("CRASH_ON_PANIC").is_ok() {
            self.crash_on_panic = true
        }

        if env::var("TOKIO_JAEGER").is_ok() {
            self.enable_jaeger = true
        }

        if env::var("TOKIO_CHROME").is_ok() {
            self.chrome_trace_output = true;
        }

        if env::var("ENABLE_JSON_LOGS").is_ok() {
            self.json_log_output = true;
        }

        if env::var("TOKIO_CONSOLE").is_ok() {
            self.tokio_console = true;
        }

        if let Ok(span_level) = env::var("TOKIO_SPAN_LEVEL") {
            self.span_level =
                Some(Level::from_str(&span_level).expect("Cannot parse TOKIO_SPAN_LEVEL"));
        }

        if let Ok(filepath) = env::var("RUST_LOG_FILE") {
            self.log_file = Some(filepath);
        }

        self
    }

    pub fn init(self) -> (TelemetryGuards, FilterHandle) {
        let config = self;

        // Setup an EnvFilter for filtering logging output layers.
        // NOTE: we don't want to use this to filter all layers.  That causes problems for layers with
        // different filtering needs, including tokio-console/console-subscriber, and it also doesn't
        // fit with the span creation needs for distributed tracing and other span-based tools.
        let log_level = config.log_string.unwrap_or_else(|| "info".into());
        let env_filter =
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level));
        let (log_filter, reload_handle) = reload::Layer::new(env_filter);
        let filter_handle = FilterHandle(reload_handle);

        // Separate span level filter.
        // This is a dumb filter for now - allows all spans that are below a given level.
        // TODO: implement a sampling filter
        let span_level = config.span_level.unwrap_or(Level::INFO);
        let span_filter = filter::filter_fn(move |metadata| {
            metadata.is_span() && *metadata.level() <= span_level
        });

        let mut layers = Vec::new();

        // tokio-console layer
        // Please see https://docs.rs/console-subscriber/latest/console_subscriber/struct.Builder.html#configuration
        // for environment vars/config options
        #[cfg(feature = "tokio-console")]
        if config.tokio_console {
            layers.push(console_subscriber::spawn().boxed());
        }

        #[cfg(feature = "chrome")]
        let chrome_guard = if config.chrome_trace_output {
            let (chrome_layer, guard) = tracing_chrome::ChromeLayerBuilder::new().build();
            layers.push(chrome_layer.boxed());
            Some(guard)
        } else {
            None
        };

        if let Some(registry) = config.prom_registry {
            let span_lat_layer = PrometheusSpanLatencyLayer::try_new(&registry, 15)
                .expect("Could not initialize span latency layer");
            layers.push(span_lat_layer.with_filter(span_filter.clone()).boxed());
        }

        #[cfg(feature = "jaeger")]
        if config.enable_jaeger {
            // Install a tracer to send traces to Jaeger.  Batching for better performance.
            let tracer = opentelemetry_jaeger::new_agent_pipeline()
                .with_service_name(&config.service_name)
                .with_max_packet_size(9216) // Default max UDP packet size on OSX
                .with_auto_split_batch(true) // Auto split batches so they fit under packet size
                .install_batch(opentelemetry::runtime::Tokio)
                .expect("Could not create async Tracer");

            // Create a tracing subscriber with the configured tracer
            let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

            // Enable Trace Contexts for tying spans together
            opentelemetry::global::set_text_map_propagator(
                opentelemetry::sdk::propagation::TraceContextPropagator::new(),
            );

            layers.push(telemetry.with_filter(span_filter.clone()).boxed());
        }

        let (nb_output, worker_guard) = get_output(config.log_file.clone());
        if config.json_log_output {
            // See https://www.lpalmieri.com/posts/2020-09-27-zero-to-production-4-are-we-observable-yet/#5-7-tracing-bunyan-formatter
            // Also Bunyan layer addes JSON logging for tracing spans with duration information
            let json_layer = JsonStorageLayer
                .and_then(
                    BunyanFormattingLayer::new(config.service_name, nb_output)
                        .with_filter(log_filter),
                )
                .boxed();
            layers.push(json_layer);
        } else {
            // Output to file or to stderr with ANSI colors
            let fmt_layer = fmt::layer()
                .with_ansi(config.log_file.is_none() && stderr().is_tty())
                .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
                .with_writer(nb_output)
                .with_filter(log_filter)
                .boxed();
            layers.push(fmt_layer);
        }

        tracing_subscriber::registry().with(layers).init();

        if config.panic_hook {
            set_panic_hook(config.crash_on_panic);
        }

        // The guard must be returned and kept in the main fn of the app, as when it's dropped then the output
        // gets flushed and closed. If this is dropped too early then no output will appear!
        let guards = TelemetryGuards {
            worker_guard,
            #[cfg(feature = "chrome")]
            chrome_guard,
        };

        (guards, filter_handle)
    }
}

/// Globally set a tracing subscriber suitable for testing environments
pub fn init_for_testing() {
    use once_cell::sync::Lazy;

    static LOGGER: Lazy<()> = Lazy::new(|| {
        let subscriber = ::tracing_subscriber::FmtSubscriber::builder()
            .with_env_filter(
                EnvFilter::builder()
                    .with_default_directive(LevelFilter::INFO.into())
                    .from_env_lossy(),
            )
            .with_file(true)
            .with_line_number(true)
            .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
            .with_test_writer()
            .finish();
        ::tracing::subscriber::set_global_default(subscriber)
            .expect("unable to initialize logging for tests");
    });

    Lazy::force(&LOGGER);
}

#[cfg(test)]
mod tests {
    use super::*;
    use prometheus::proto::MetricType;
    use std::time::Duration;
    use tracing::{debug, debug_span, info, trace_span, warn};

    #[test]
    #[should_panic]
    fn test_telemetry_init() {
        let registry = prometheus::Registry::new();
        // Default logging level is INFO, but here we set the span level to DEBUG.  TRACE spans should be ignored.
        let config = TelemetryConfig::new("my_app")
            .with_span_level(Level::DEBUG)
            .with_prom_registry(&registry);
        let _guard = config.init();

        info!(a = 1, "This will be INFO.");
        // Spans are debug level or below, so they won't be printed out either.  However latencies
        // should be recorded for at least one span
        debug_span!("yo span yo").in_scope(|| {
            // This debug log will not print out, log level set to INFO by default
            debug!(a = 2, "This will be DEBUG.");
            std::thread::sleep(Duration::from_millis(100));
            warn!(a = 3, "This will be WARNING.");
        });

        // This span won't be enabled
        trace_span!("this span should not be created").in_scope(|| {
            info!("This log appears, but surrounding span is not created");
            std::thread::sleep(Duration::from_millis(100));
        });

        let metrics = registry.gather();
        // There should be 1 metricFamily and 1 metric
        assert_eq!(metrics.len(), 1);
        assert_eq!(metrics[0].get_name(), "tracing_span_latencies");
        assert_eq!(metrics[0].get_field_type(), MetricType::HISTOGRAM);
        let inner = metrics[0].get_metric();
        assert_eq!(inner.len(), 1);
        let labels = inner[0].get_label();
        assert_eq!(labels.len(), 1);
        assert_eq!(labels[0].get_name(), "span_name");
        assert_eq!(labels[0].get_value(), "yo span yo");

        panic!("This should cause error logs to be printed out!");
    }

    /*
    Both the following tests should be able to "race" to initialize logging without causing a
    panic
    */
    #[test]
    fn testing_logger_1() {
        init_for_testing();
    }

    #[test]
    fn testing_logger_2() {
        init_for_testing();
    }
}