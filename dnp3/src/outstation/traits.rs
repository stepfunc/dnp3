/// Enumeration returned for cold/warm restart
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum RestartDelay {
    /// corresponds to g51v1
    Seconds(u16),
    /// corresponds to g52v2
    Milliseconds(u16),
}

/// dynamic information required by the outstation from the user application
pub trait OutstationApplication: Sync + Send + 'static {
    /// The value returned by this method is used in conjunction with the `Delay Measurement`
    /// function code and returned in a g52v2 time delay object as part of a non-LAN time
    /// synchronization procedure.
    ///
    /// It represents the processing delay from receiving the request to sending the response.
    /// This parameter should almost always use the default value of zero as only an RTOS
    /// or bare metal system would have access to this level of timing. Modern hardware
    /// can almost always respond in less than 1 millisecond anyway.
    ///
    /// For more information, see IEEE-1815 2012, pg. 64
    fn get_processing_delay_ms(&self) -> u16 {
        0
    }

    /// Request that the outstation perform a cold restart (IEEE-1815 2012, pg. 58)
    ///
    /// If supported, return Some(RestartDelay) indicating how long the restart
    /// will take to complete
    ///
    /// returning None, will cause the outstation to return IIN.2 FUNCTION_NOT_SUPPORTED
    ///
    /// The outstation will not automatically restart. It is the responsibility of the user
    /// application to handle this request and take the appropriate acton.
    fn cold_restart(&mut self) -> Option<RestartDelay> {
        None
    }

    /// Request that the outstation perform a warm restart (IEEE-1815 2012, pg. 58)
    ///
    /// If supported, return Some(RestartDelay) indicating how long the restart
    /// will take to complete
    ///
    /// returning None, will cause the outstation to return IIN.2 FUNCTION_NOT_SUPPORTED
    ///
    /// The outstation will not automatically restart. It is the responsibility of the user
    /// application to handle this request and take the appropriate acton.
    fn warm_restart(&mut self) -> Option<RestartDelay> {
        None
    }
}

#[derive(Copy, Clone)]
pub struct NullHandler;

impl NullHandler {
    pub fn create() -> Box<dyn OutstationApplication> {
        Box::new(NullHandler)
    }
}

impl OutstationApplication for NullHandler {}
