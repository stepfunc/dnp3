//! Shared CLI utilities for DNP3 examples
//!
//! This crate contains shared types and utilities for DNP3 example CLI applications.

use clap::ValueEnum;

/// Shared log level enum for CLI applications
#[derive(Debug, Clone, Copy, ValueEnum)]
#[allow(missing_docs)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl From<LogLevel> for tracing::Level {
    fn from(value: LogLevel) -> Self {
        match value {
            LogLevel::Trace => tracing::Level::TRACE,
            LogLevel::Debug => tracing::Level::DEBUG,
            LogLevel::Info => tracing::Level::INFO,
            LogLevel::Warn => tracing::Level::WARN,
            LogLevel::Error => tracing::Level::ERROR,
        }
    }
}

// Re-export the serial parameter enums and implementations
pub mod serial;
