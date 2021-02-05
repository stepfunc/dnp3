use crate::ffi;
use dnp3::config::{AppDecodeLevel, LinkDecodeLevel, TransportDecodeLevel};
use dnp3::prelude::master::*;
use std::ffi::CString;
use tracing::span::{Attributes, Record};
use tracing::{Event, Id, Metadata};
use tracing_subscriber::fmt::time::{ChronoUtc, SystemTime};
use tracing_subscriber::fmt::MakeWriter;

thread_local! {
   pub static LOG_BUFFER: std::cell::RefCell<Vec<u8>> = std::cell::RefCell::new(Vec::new());
}

pub fn configure_logging(config: ffi::LoggingConfiguration, handler: ffi::Logger) {
    tracing::subscriber::set_global_default(adapter(config, handler))
        .expect("unable to install tracing subscriber");
}

struct ThreadLocalBufferWriter;

struct ThreadLocalMakeWriter;

impl MakeWriter for ThreadLocalMakeWriter {
    type Writer = ThreadLocalBufferWriter;

    fn make_writer(&self) -> Self::Writer {
        ThreadLocalBufferWriter
    }
}

impl std::io::Write for ThreadLocalBufferWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        LOG_BUFFER.with(|vec| vec.borrow_mut().extend_from_slice(buf));
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn adapter(
    config: ffi::LoggingConfiguration,
    handler: ffi::Logger,
) -> impl tracing::Subscriber + Send + Sync + 'static {
    Adapter {
        handler,
        inner: config.build(),
    }
}

impl ffi::LoggingConfiguration {
    fn build(&self) -> Box<dyn tracing::Subscriber + Send + Sync> {
        let level: tracing::Level = self.level().into();

        let builder = tracing_subscriber::fmt()
            .with_max_level(level)
            .with_level(self.print_level)
            .with_target(self.print_module_info)
            .with_writer(ThreadLocalMakeWriter);

        match self.time_format() {
            ffi::TimeFormat::None => {
                let builder = builder.without_time();
                match self.output_format() {
                    ffi::LogOutputFormat::Text => Box::new(builder.finish()),
                    ffi::LogOutputFormat::JSON => Box::new(builder.json().finish()),
                }
            }
            ffi::TimeFormat::RFC3339 => {
                let builder = builder.with_timer(ChronoUtc::default());
                match self.output_format() {
                    ffi::LogOutputFormat::Text => Box::new(builder.finish()),
                    ffi::LogOutputFormat::JSON => Box::new(builder.json().finish()),
                }
            }
            ffi::TimeFormat::System => {
                let builder = builder.with_timer(SystemTime::default());
                match self.output_format() {
                    ffi::LogOutputFormat::Text => Box::new(builder.finish()),
                    ffi::LogOutputFormat::JSON => Box::new(builder.json().finish()),
                }
            }
        }
    }
}

struct Adapter {
    handler: ffi::Logger,
    inner: Box<dyn tracing::Subscriber + Send + Sync + 'static>,
}

impl tracing::Subscriber for Adapter {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        self.inner.enabled(metadata)
    }

    fn new_span(&self, span: &Attributes<'_>) -> Id {
        self.inner.new_span(span)
    }

    fn record(&self, span: &Id, values: &Record<'_>) {
        self.inner.record(span, values)
    }

    fn record_follows_from(&self, span: &Id, follows: &Id) {
        self.inner.record_follows_from(span, follows)
    }

    fn event(&self, event: &Event<'_>) {
        self.inner.event(event);
        if let Ok(string) = LOG_BUFFER.with(|vec| CString::new(vec.borrow().as_slice())) {
            self.handler
                .on_message((*event.metadata().level()).into(), &string);
        }
        LOG_BUFFER.with(|vec| vec.borrow_mut().clear())
    }

    fn enter(&self, span: &Id) {
        self.inner.enter(span)
    }

    fn exit(&self, span: &Id) {
        self.inner.exit(span)
    }
}

impl From<tracing::Level> for ffi::LogLevel {
    fn from(level: tracing::Level) -> Self {
        match level {
            tracing::Level::DEBUG => ffi::LogLevel::Debug,
            tracing::Level::TRACE => ffi::LogLevel::Trace,
            tracing::Level::INFO => ffi::LogLevel::Info,
            tracing::Level::WARN => ffi::LogLevel::Warn,
            tracing::Level::ERROR => ffi::LogLevel::Error,
        }
    }
}

impl From<ffi::LogLevel> for tracing::Level {
    fn from(level: ffi::LogLevel) -> Self {
        match level {
            ffi::LogLevel::Debug => tracing::Level::DEBUG,
            ffi::LogLevel::Trace => tracing::Level::TRACE,
            ffi::LogLevel::Info => tracing::Level::INFO,
            ffi::LogLevel::Warn => tracing::Level::WARN,
            ffi::LogLevel::Error => tracing::Level::ERROR,
        }
    }
}

impl From<ffi::DecodeLevel> for DecodeLevel {
    fn from(from: ffi::DecodeLevel) -> Self {
        Self {
            application: from.application().into(),
            transport: from.transport().into(),
            link: from.link().into(),
        }
    }
}

impl From<DecodeLevel> for ffi::DecodeLevel {
    fn from(from: DecodeLevel) -> Self {
        ffi::DecodeLevelFields {
            application: from.application.into(),
            transport: from.transport.into(),
            link: from.link.into(),
        }
        .into()
    }
}

impl From<ffi::AppDecodeLevel> for AppDecodeLevel {
    fn from(from: ffi::AppDecodeLevel) -> Self {
        match from {
            ffi::AppDecodeLevel::Nothing => Self::Nothing,
            ffi::AppDecodeLevel::Header => Self::Header,
            ffi::AppDecodeLevel::ObjectHeaders => Self::ObjectHeaders,
            ffi::AppDecodeLevel::ObjectValues => Self::ObjectValues,
        }
    }
}

impl From<ffi::TransportDecodeLevel> for TransportDecodeLevel {
    fn from(from: ffi::TransportDecodeLevel) -> Self {
        match from {
            ffi::TransportDecodeLevel::Nothing => Self::Nothing,
            ffi::TransportDecodeLevel::Header => Self::Header,
            ffi::TransportDecodeLevel::Payload => Self::Payload,
        }
    }
}

impl From<ffi::LinkDecodeLevel> for LinkDecodeLevel {
    fn from(from: ffi::LinkDecodeLevel) -> Self {
        match from {
            ffi::LinkDecodeLevel::Nothing => Self::Nothing,
            ffi::LinkDecodeLevel::Header => Self::Header,
            ffi::LinkDecodeLevel::Payload => Self::Payload,
        }
    }
}

impl From<AppDecodeLevel> for ffi::AppDecodeLevel {
    fn from(from: AppDecodeLevel) -> Self {
        match from {
            AppDecodeLevel::Nothing => ffi::AppDecodeLevel::Nothing,
            AppDecodeLevel::Header => ffi::AppDecodeLevel::Header,
            AppDecodeLevel::ObjectHeaders => ffi::AppDecodeLevel::ObjectHeaders,
            AppDecodeLevel::ObjectValues => ffi::AppDecodeLevel::ObjectValues,
        }
    }
}

impl From<TransportDecodeLevel> for ffi::TransportDecodeLevel {
    fn from(from: TransportDecodeLevel) -> Self {
        match from {
            TransportDecodeLevel::Nothing => ffi::TransportDecodeLevel::Nothing,
            TransportDecodeLevel::Header => ffi::TransportDecodeLevel::Header,
            TransportDecodeLevel::Payload => ffi::TransportDecodeLevel::Payload,
        }
    }
}

impl From<LinkDecodeLevel> for ffi::LinkDecodeLevel {
    fn from(from: LinkDecodeLevel) -> Self {
        match from {
            LinkDecodeLevel::Nothing => ffi::LinkDecodeLevel::Nothing,
            LinkDecodeLevel::Header => ffi::LinkDecodeLevel::Header,
            LinkDecodeLevel::Payload => ffi::LinkDecodeLevel::Payload,
        }
    }
}
