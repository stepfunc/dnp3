use crate::app::measurement::*;
use crate::app::parse::traits::FixedSizeVariation;
use crate::app::variations::*;
use crate::outstation::database::config::*;
use crate::outstation::database::details::event::write_fn::ToVariationCto;

/// A variation of events of type `T`
pub(crate) trait EventVariation<T>: Copy {
    /// apply the encoder to the type the variation is encoded as
    ///
    /// Each variation names the type once, and its group and variation, its encoding and whether
    /// its time is relative to a CTO all come from that type.
    fn encode<E>(self, encoder: E) -> E::Output
    where
        E: Encoder<T>;

    /// the variation the event is written in when this one is selected for it
    ///
    /// Binary and double-bit events with absolute time must have been recorded while synchronized
    /// (TB2017-003a 5.1.3.1.2), and events recorded without time are reported without time
    /// (IED certification procedures 3.1 rev 1, 8.14.1), whether the variation is the default or
    /// the one requested.
    fn for_event(self, _event: &T) -> Self {
        self
    }
}

/// Something done with an event, given how its variation is encoded
pub(crate) trait Encoder<T> {
    type Output;

    /// a fixed-size variation
    fn fixed<V>(self) -> Self::Output
    where
        V: FixedSizeVariation,
        T: ToVariation<V>;

    /// a fixed-size variation whose time is relative to a common time object (CTO)
    fn cto<V>(self) -> Self::Output
    where
        V: FixedSizeVariation,
        T: ToVariationCto<V>;

    /// an octet string, whose variation is its length
    fn octet_string(self) -> Self::Output
    where
        T: AsRef<[u8]>;
}

impl EventVariation<BinaryInput> for EventBinaryInputVariation {
    fn encode<E>(self, encoder: E) -> E::Output
    where
        E: Encoder<BinaryInput>,
    {
        match self {
            Self::Group2Var1 => encoder.fixed::<Group2Var1>(),
            Self::Group2Var2 => encoder.fixed::<Group2Var2>(),
            Self::Group2Var3 => encoder.cto::<Group2Var3>(),
        }
    }

    fn for_event(self, event: &BinaryInput) -> Self {
        match (self, event.time) {
            (Self::Group2Var2 | Self::Group2Var3, None) => Self::Group2Var1,
            (Self::Group2Var2, Some(Time::Unsynchronized(_))) => Self::Group2Var3,
            (Self::Group2Var1, _)
            | (Self::Group2Var2, Some(Time::Synchronized(_)))
            | (Self::Group2Var3, Some(_)) => self,
        }
    }
}

impl EventVariation<DoubleBitBinaryInput> for EventDoubleBitBinaryInputVariation {
    fn encode<E>(self, encoder: E) -> E::Output
    where
        E: Encoder<DoubleBitBinaryInput>,
    {
        match self {
            Self::Group4Var1 => encoder.fixed::<Group4Var1>(),
            Self::Group4Var2 => encoder.fixed::<Group4Var2>(),
            Self::Group4Var3 => encoder.cto::<Group4Var3>(),
        }
    }

    fn for_event(self, event: &DoubleBitBinaryInput) -> Self {
        match (self, event.time) {
            (Self::Group4Var2 | Self::Group4Var3, None) => Self::Group4Var1,
            (Self::Group4Var2, Some(Time::Unsynchronized(_))) => Self::Group4Var3,
            (Self::Group4Var1, _)
            | (Self::Group4Var2, Some(Time::Synchronized(_)))
            | (Self::Group4Var3, Some(_)) => self,
        }
    }
}

impl EventVariation<BinaryOutputStatus> for EventBinaryOutputStatusVariation {
    fn encode<E>(self, encoder: E) -> E::Output
    where
        E: Encoder<BinaryOutputStatus>,
    {
        match self {
            Self::Group11Var1 => encoder.fixed::<Group11Var1>(),
            Self::Group11Var2 => encoder.fixed::<Group11Var2>(),
        }
    }
}

impl EventVariation<Counter> for EventCounterVariation {
    fn encode<E>(self, encoder: E) -> E::Output
    where
        E: Encoder<Counter>,
    {
        match self {
            Self::Group22Var1 => encoder.fixed::<Group22Var1>(),
            Self::Group22Var2 => encoder.fixed::<Group22Var2>(),
            Self::Group22Var5 => encoder.fixed::<Group22Var5>(),
            Self::Group22Var6 => encoder.fixed::<Group22Var6>(),
        }
    }
}

impl EventVariation<FrozenCounter> for EventFrozenCounterVariation {
    fn encode<E>(self, encoder: E) -> E::Output
    where
        E: Encoder<FrozenCounter>,
    {
        match self {
            Self::Group23Var1 => encoder.fixed::<Group23Var1>(),
            Self::Group23Var2 => encoder.fixed::<Group23Var2>(),
            Self::Group23Var5 => encoder.fixed::<Group23Var5>(),
            Self::Group23Var6 => encoder.fixed::<Group23Var6>(),
        }
    }
}

impl EventVariation<AnalogInput> for EventAnalogInputVariation {
    fn encode<E>(self, encoder: E) -> E::Output
    where
        E: Encoder<AnalogInput>,
    {
        match self {
            Self::Group32Var1 => encoder.fixed::<Group32Var1>(),
            Self::Group32Var2 => encoder.fixed::<Group32Var2>(),
            Self::Group32Var3 => encoder.fixed::<Group32Var3>(),
            Self::Group32Var4 => encoder.fixed::<Group32Var4>(),
            Self::Group32Var5 => encoder.fixed::<Group32Var5>(),
            Self::Group32Var6 => encoder.fixed::<Group32Var6>(),
            Self::Group32Var7 => encoder.fixed::<Group32Var7>(),
            Self::Group32Var8 => encoder.fixed::<Group32Var8>(),
        }
    }
}

impl EventVariation<AnalogOutputStatus> for EventAnalogOutputStatusVariation {
    fn encode<E>(self, encoder: E) -> E::Output
    where
        E: Encoder<AnalogOutputStatus>,
    {
        match self {
            Self::Group42Var1 => encoder.fixed::<Group42Var1>(),
            Self::Group42Var2 => encoder.fixed::<Group42Var2>(),
            Self::Group42Var3 => encoder.fixed::<Group42Var3>(),
            Self::Group42Var4 => encoder.fixed::<Group42Var4>(),
            Self::Group42Var5 => encoder.fixed::<Group42Var5>(),
            Self::Group42Var6 => encoder.fixed::<Group42Var6>(),
            Self::Group42Var7 => encoder.fixed::<Group42Var7>(),
            Self::Group42Var8 => encoder.fixed::<Group42Var8>(),
        }
    }
}

impl EventVariation<Box<[u8]>> for EventOctetStringVariation {
    fn encode<E>(self, encoder: E) -> E::Output
    where
        E: Encoder<Box<[u8]>>,
    {
        encoder.octet_string()
    }
}
