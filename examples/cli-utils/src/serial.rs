//! Serial port parameters for CLI applications
use clap::ValueEnum;
use dnp3::serial::{DataBits, FlowControl, Parity, StopBits};

/// Data bits for serial communication
#[derive(Debug, Clone, Copy, ValueEnum)]
#[allow(missing_docs)]
pub enum DataBitsArg {
    Five,
    Six,
    Seven,
    Eight,
}

/// Stop bits for serial communication
#[derive(Debug, Clone, Copy, ValueEnum)]
#[allow(missing_docs)]
pub enum StopBitsArg {
    One,
    Two,
}

/// Parity for serial communication
#[derive(Debug, Clone, Copy, ValueEnum)]
#[allow(missing_docs)]
pub enum ParityArg {
    None,
    Odd,
    Even,
}

/// Flow control for serial communication
#[derive(Debug, Clone, Copy, ValueEnum)]
#[allow(missing_docs)]
pub enum FlowControlArg {
    None,
    Software,
    Hardware,
}

impl From<DataBitsArg> for DataBits {
    fn from(value: DataBitsArg) -> Self {
        match value {
            DataBitsArg::Five => DataBits::Five,
            DataBitsArg::Six => DataBits::Six,
            DataBitsArg::Seven => DataBits::Seven,
            DataBitsArg::Eight => DataBits::Eight,
        }
    }
}

impl From<StopBitsArg> for StopBits {
    fn from(value: StopBitsArg) -> Self {
        match value {
            StopBitsArg::One => StopBits::One,
            StopBitsArg::Two => StopBits::Two,
        }
    }
}

impl From<ParityArg> for Parity {
    fn from(value: ParityArg) -> Self {
        match value {
            ParityArg::None => Parity::None,
            ParityArg::Odd => Parity::Odd,
            ParityArg::Even => Parity::Even,
        }
    }
}

impl From<FlowControlArg> for FlowControl {
    fn from(value: FlowControlArg) -> Self {
        match value {
            FlowControlArg::None => FlowControl::None,
            FlowControlArg::Software => FlowControl::Software,
            FlowControlArg::Hardware => FlowControl::Hardware,
        }
    }
}