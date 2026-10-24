// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//  _   _         ______    _ _ _   _             _ _ _
// | \ | |       |  ____|  | (_) | (_)           | | | |
// |  \| | ___   | |__   __| |_| |_ _ _ __   __ _| | | |
// | . ` |/ _ \  |  __| / _` | | __| | '_ \ / _` | | | |
// | |\  | (_) | | |___| (_| | | |_| | | | | (_| |_|_|_|
// |_| \_|\___/  |______\__,_|_|\__|_|_| |_|\__, (_|_|_)
//                                           __/ |
//                                          |___/
//
// This file is auto-generated. Do not edit manually
//

use crate::app::gen::prefixed::PrefixedVariation;
use crate::app::gen::ranged::RangedVariation;
use crate::app::measurement::Time;
use crate::app::parse::traits::Index;
use crate::app::{QualifierCode, Variation};
use crate::master::{HeaderInfo, ReadHandler};

impl<'a> RangedVariation<'a> {
    pub(crate) fn extract_measurements_to<H: ReadHandler + ?Sized>(&self, qualifier: QualifierCode, handler: &mut H) -> bool {
        match self {
            RangedVariation::Group0Var254 => {
                false // extraction not supported
            }
            RangedVariation::Group0(attr) => {
                crate::master::handle_attribute(self.variation(), qualifier, attr, handler)
            }
            RangedVariation::Group1Var0 => {
                false // extraction not supported
            }
            RangedVariation::Group1Var1(seq) => {
                handler.handle_binary_input(
                    HeaderInfo::new(self.variation(), qualifier, false, false),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group1Var2(seq) => {
                handler.handle_binary_input(
                    HeaderInfo::new(self.variation(), qualifier, false, true),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group3Var0 => {
                false // extraction not supported
            }
            RangedVariation::Group3Var1(seq) => {
                handler.handle_double_bit_binary_input(
                    HeaderInfo::new(self.variation(), qualifier, false, false),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group3Var2(seq) => {
                handler.handle_double_bit_binary_input(
                    HeaderInfo::new(self.variation(), qualifier, false, true),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group10Var0 => {
                false // extraction not supported
            }
            RangedVariation::Group10Var1(seq) => {
                handler.handle_binary_output_status(
                    HeaderInfo::new(self.variation(), qualifier, false, false),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group10Var2(seq) => {
                handler.handle_binary_output_status(
                    HeaderInfo::new(self.variation(), qualifier, false, true),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group20Var0 => {
                false // extraction not supported
            }
            RangedVariation::Group20Var1(seq) => {
                handler.handle_counter(
                    HeaderInfo::new(self.variation(), qualifier, false, true),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group20Var2(seq) => {
                handler.handle_counter(
                    HeaderInfo::new(self.variation(), qualifier, false, true),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group20Var5(seq) => {
                handler.handle_counter(
                    HeaderInfo::new(self.variation(), qualifier, false, false),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group20Var6(seq) => {
                handler.handle_counter(
                    HeaderInfo::new(self.variation(), qualifier, false, false),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group21Var0 => {
                false // extraction not supported
            }
            RangedVariation::Group21Var1(seq) => {
                handler.handle_frozen_counter(
                    HeaderInfo::new(self.variation(), qualifier, false, true),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group21Var2(seq) => {
                handler.handle_frozen_counter(
                    HeaderInfo::new(self.variation(), qualifier, false, true),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group21Var5(seq) => {
                handler.handle_frozen_counter(
                    HeaderInfo::new(self.variation(), qualifier, false, true),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group21Var6(seq) => {
                handler.handle_frozen_counter(
                    HeaderInfo::new(self.variation(), qualifier, false, true),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group21Var9(seq) => {
                handler.handle_frozen_counter(
                    HeaderInfo::new(self.variation(), qualifier, false, false),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group21Var10(seq) => {
                handler.handle_frozen_counter(
                    HeaderInfo::new(self.variation(), qualifier, false, false),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group30Var0 => {
                false // extraction not supported
            }
            RangedVariation::Group30Var1(seq) => {
                handler.handle_analog_input(
                    HeaderInfo::new(self.variation(), qualifier, false, true),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group30Var2(seq) => {
                handler.handle_analog_input(
                    HeaderInfo::new(self.variation(), qualifier, false, true),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group30Var3(seq) => {
                handler.handle_analog_input(
                    HeaderInfo::new(self.variation(), qualifier, false, false),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group30Var4(seq) => {
                handler.handle_analog_input(
                    HeaderInfo::new(self.variation(), qualifier, false, false),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group30Var5(seq) => {
                handler.handle_analog_input(
                    HeaderInfo::new(self.variation(), qualifier, false, true),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group30Var6(seq) => {
                handler.handle_analog_input(
                    HeaderInfo::new(self.variation(), qualifier, false, true),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group31Var0 => {
                false // extraction not supported
            }
            RangedVariation::Group31Var1(seq) => {
                handler.handle_frozen_analog_input(
                    HeaderInfo::new(self.variation(), qualifier, false, true),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group31Var2(seq) => {
                handler.handle_frozen_analog_input(
                    HeaderInfo::new(self.variation(), qualifier, false, true),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group31Var3(seq) => {
                handler.handle_frozen_analog_input(
                    HeaderInfo::new(self.variation(), qualifier, false, true),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group31Var4(seq) => {
                handler.handle_frozen_analog_input(
                    HeaderInfo::new(self.variation(), qualifier, false, true),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group31Var5(seq) => {
                handler.handle_frozen_analog_input(
                    HeaderInfo::new(self.variation(), qualifier, false, false),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group31Var6(seq) => {
                handler.handle_frozen_analog_input(
                    HeaderInfo::new(self.variation(), qualifier, false, false),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group31Var7(seq) => {
                handler.handle_frozen_analog_input(
                    HeaderInfo::new(self.variation(), qualifier, false, true),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group31Var8(seq) => {
                handler.handle_frozen_analog_input(
                    HeaderInfo::new(self.variation(), qualifier, false, true),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group34Var0 => {
                false // extraction not supported
            }
            RangedVariation::Group34Var1(seq) => {
                handler.handle_analog_input_dead_band(
                    HeaderInfo::new(self.variation(), qualifier, false, false),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group34Var2(seq) => {
                handler.handle_analog_input_dead_band(
                    HeaderInfo::new(self.variation(), qualifier, false, false),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group34Var3(seq) => {
                handler.handle_analog_input_dead_band(
                    HeaderInfo::new(self.variation(), qualifier, false, false),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group40Var0 => {
                false // extraction not supported
            }
            RangedVariation::Group40Var1(seq) => {
                handler.handle_analog_output_status(
                    HeaderInfo::new(self.variation(), qualifier, false, true),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group40Var2(seq) => {
                handler.handle_analog_output_status(
                    HeaderInfo::new(self.variation(), qualifier, false, true),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group40Var3(seq) => {
                handler.handle_analog_output_status(
                    HeaderInfo::new(self.variation(), qualifier, false, true),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group40Var4(seq) => {
                handler.handle_analog_output_status(
                    HeaderInfo::new(self.variation(), qualifier, false, true),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group80Var1(_) => {
                false // internal indications
            }
            RangedVariation::Group102Var0 => {
                false // extraction not supported
            }
            RangedVariation::Group102Var1(seq) => {
                handler.handle_unsigned_integer(
                    HeaderInfo::new(self.variation(), qualifier, false, false),
                    &mut seq.iter().map(|(v,i)| (v.into(), i))
                );
                true
            }
            RangedVariation::Group110VarX(_,seq) => {
                handler.handle_octet_string(
                    HeaderInfo::new(self.variation(), qualifier, false, false),
                    &mut seq.iter()
                );
                true
            }
        }
    }
}

impl<'a, I> PrefixedVariation<'a, I> where I : Index {
    pub(crate) fn extract_measurements_to<H: ReadHandler + ?Sized>(&self, cto: Option<Time>, handler: &mut H) -> bool {
        match self {
            PrefixedVariation::Group0(attr) => {
                let info = self.get_header_info();
                crate::master::handle_attribute(info.variation, info.qualifier, attr, handler)
            }
            PrefixedVariation::Group2Var1(seq) => {
                handler.handle_binary_input(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group2Var2(seq) => {
                handler.handle_binary_input(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group2Var3(seq) => {
                handler.handle_binary_input(
                    self.get_header_info(),
                    &mut seq.iter().map( |x| (x.value.to_measurement(cto), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group4Var1(seq) => {
                handler.handle_double_bit_binary_input(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group4Var2(seq) => {
                handler.handle_double_bit_binary_input(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group4Var3(seq) => {
                handler.handle_double_bit_binary_input(
                    self.get_header_info(),
                    &mut seq.iter().map( |x| (x.value.to_measurement(cto), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group11Var1(seq) => {
                handler.handle_binary_output_status(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group11Var2(seq) => {
                handler.handle_binary_output_status(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group12Var1(_) => {
                false // command
            }
            PrefixedVariation::Group13Var1(seq) => {
                handler.handle_binary_output_command_event(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group13Var2(seq) => {
                handler.handle_binary_output_command_event(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group22Var1(seq) => {
                handler.handle_counter(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group22Var2(seq) => {
                handler.handle_counter(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group22Var5(seq) => {
                handler.handle_counter(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group22Var6(seq) => {
                handler.handle_counter(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group23Var1(seq) => {
                handler.handle_frozen_counter(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group23Var2(seq) => {
                handler.handle_frozen_counter(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group23Var5(seq) => {
                handler.handle_frozen_counter(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group23Var6(seq) => {
                handler.handle_frozen_counter(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group32Var1(seq) => {
                handler.handle_analog_input(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group32Var2(seq) => {
                handler.handle_analog_input(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group32Var3(seq) => {
                handler.handle_analog_input(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group32Var4(seq) => {
                handler.handle_analog_input(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group32Var5(seq) => {
                handler.handle_analog_input(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group32Var6(seq) => {
                handler.handle_analog_input(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group32Var7(seq) => {
                handler.handle_analog_input(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group32Var8(seq) => {
                handler.handle_analog_input(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group33Var1(seq) => {
                handler.handle_frozen_analog_input(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group33Var2(seq) => {
                handler.handle_frozen_analog_input(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group33Var3(seq) => {
                handler.handle_frozen_analog_input(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group33Var4(seq) => {
                handler.handle_frozen_analog_input(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group33Var5(seq) => {
                handler.handle_frozen_analog_input(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group33Var6(seq) => {
                handler.handle_frozen_analog_input(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group33Var7(seq) => {
                handler.handle_frozen_analog_input(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group33Var8(seq) => {
                handler.handle_frozen_analog_input(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group34Var1(_) => {
                false // dead-band
            }
            PrefixedVariation::Group34Var2(_) => {
                false // dead-band
            }
            PrefixedVariation::Group34Var3(_) => {
                false // dead-band
            }
            PrefixedVariation::Group41Var1(_) => {
                false // command
            }
            PrefixedVariation::Group41Var2(_) => {
                false // command
            }
            PrefixedVariation::Group41Var3(_) => {
                false // command
            }
            PrefixedVariation::Group41Var4(_) => {
                false // command
            }
            PrefixedVariation::Group42Var1(seq) => {
                handler.handle_analog_output_status(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group42Var2(seq) => {
                handler.handle_analog_output_status(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group42Var3(seq) => {
                handler.handle_analog_output_status(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group42Var4(seq) => {
                handler.handle_analog_output_status(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group42Var5(seq) => {
                handler.handle_analog_output_status(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group42Var6(seq) => {
                handler.handle_analog_output_status(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group42Var7(seq) => {
                handler.handle_analog_output_status(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group42Var8(seq) => {
                handler.handle_analog_output_status(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group43Var1(seq) => {
                handler.handle_analog_output_command_event(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group43Var2(seq) => {
                handler.handle_analog_output_command_event(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group43Var3(seq) => {
                handler.handle_analog_output_command_event(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group43Var4(seq) => {
                handler.handle_analog_output_command_event(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group43Var5(seq) => {
                handler.handle_analog_output_command_event(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group43Var6(seq) => {
                handler.handle_analog_output_command_event(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group43Var7(seq) => {
                handler.handle_analog_output_command_event(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group43Var8(seq) => {
                handler.handle_analog_output_command_event(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.value.into(), x.index.widen_to_u16()))
                );
                true
            }
            PrefixedVariation::Group111VarX(_, seq) => {
                handler.handle_octet_string(
                    self.get_header_info(),
                    &mut seq.iter().map(|x| (x.0, x.1.widen_to_u16()))
                );
                true
            }
        }
    }
    
    pub(crate) fn get_header_info(&self) -> HeaderInfo {
        match self {
            PrefixedVariation::Group0(attr) => HeaderInfo::new(Variation::Group0(attr.variation), I::COUNT_AND_PREFIX_QUALIFIER, false, false),
            PrefixedVariation::Group2Var1(_) => HeaderInfo::new(Variation::Group2Var1, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group2Var2(_) => HeaderInfo::new(Variation::Group2Var2, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group2Var3(_) => HeaderInfo::new(Variation::Group2Var3, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group4Var1(_) => HeaderInfo::new(Variation::Group4Var1, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group4Var2(_) => HeaderInfo::new(Variation::Group4Var2, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group4Var3(_) => HeaderInfo::new(Variation::Group4Var3, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group11Var1(_) => HeaderInfo::new(Variation::Group11Var1, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group11Var2(_) => HeaderInfo::new(Variation::Group11Var2, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group12Var1(_) => HeaderInfo::new(Variation::Group12Var1, I::COUNT_AND_PREFIX_QUALIFIER, false, false),
            PrefixedVariation::Group13Var1(_) => HeaderInfo::new(Variation::Group13Var1, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group13Var2(_) => HeaderInfo::new(Variation::Group13Var2, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group22Var1(_) => HeaderInfo::new(Variation::Group22Var1, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group22Var2(_) => HeaderInfo::new(Variation::Group22Var2, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group22Var5(_) => HeaderInfo::new(Variation::Group22Var5, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group22Var6(_) => HeaderInfo::new(Variation::Group22Var6, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group23Var1(_) => HeaderInfo::new(Variation::Group23Var1, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group23Var2(_) => HeaderInfo::new(Variation::Group23Var2, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group23Var5(_) => HeaderInfo::new(Variation::Group23Var5, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group23Var6(_) => HeaderInfo::new(Variation::Group23Var6, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group32Var1(_) => HeaderInfo::new(Variation::Group32Var1, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group32Var2(_) => HeaderInfo::new(Variation::Group32Var2, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group32Var3(_) => HeaderInfo::new(Variation::Group32Var3, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group32Var4(_) => HeaderInfo::new(Variation::Group32Var4, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group32Var5(_) => HeaderInfo::new(Variation::Group32Var5, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group32Var6(_) => HeaderInfo::new(Variation::Group32Var6, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group32Var7(_) => HeaderInfo::new(Variation::Group32Var7, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group32Var8(_) => HeaderInfo::new(Variation::Group32Var8, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group33Var1(_) => HeaderInfo::new(Variation::Group33Var1, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group33Var2(_) => HeaderInfo::new(Variation::Group33Var2, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group33Var3(_) => HeaderInfo::new(Variation::Group33Var3, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group33Var4(_) => HeaderInfo::new(Variation::Group33Var4, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group33Var5(_) => HeaderInfo::new(Variation::Group33Var5, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group33Var6(_) => HeaderInfo::new(Variation::Group33Var6, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group33Var7(_) => HeaderInfo::new(Variation::Group33Var7, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group33Var8(_) => HeaderInfo::new(Variation::Group33Var8, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group34Var1(_) => HeaderInfo::new(Variation::Group34Var1, I::COUNT_AND_PREFIX_QUALIFIER, false, false),
            PrefixedVariation::Group34Var2(_) => HeaderInfo::new(Variation::Group34Var2, I::COUNT_AND_PREFIX_QUALIFIER, false, false),
            PrefixedVariation::Group34Var3(_) => HeaderInfo::new(Variation::Group34Var3, I::COUNT_AND_PREFIX_QUALIFIER, false, false),
            PrefixedVariation::Group41Var1(_) => HeaderInfo::new(Variation::Group41Var1, I::COUNT_AND_PREFIX_QUALIFIER, false, false),
            PrefixedVariation::Group41Var2(_) => HeaderInfo::new(Variation::Group41Var2, I::COUNT_AND_PREFIX_QUALIFIER, false, false),
            PrefixedVariation::Group41Var3(_) => HeaderInfo::new(Variation::Group41Var3, I::COUNT_AND_PREFIX_QUALIFIER, false, false),
            PrefixedVariation::Group41Var4(_) => HeaderInfo::new(Variation::Group41Var4, I::COUNT_AND_PREFIX_QUALIFIER, false, false),
            PrefixedVariation::Group42Var1(_) => HeaderInfo::new(Variation::Group42Var1, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group42Var2(_) => HeaderInfo::new(Variation::Group42Var2, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group42Var3(_) => HeaderInfo::new(Variation::Group42Var3, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group42Var4(_) => HeaderInfo::new(Variation::Group42Var4, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group42Var5(_) => HeaderInfo::new(Variation::Group42Var5, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group42Var6(_) => HeaderInfo::new(Variation::Group42Var6, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group42Var7(_) => HeaderInfo::new(Variation::Group42Var7, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group42Var8(_) => HeaderInfo::new(Variation::Group42Var8, I::COUNT_AND_PREFIX_QUALIFIER, true, true),
            PrefixedVariation::Group43Var1(_) => HeaderInfo::new(Variation::Group43Var1, I::COUNT_AND_PREFIX_QUALIFIER, true, false),
            PrefixedVariation::Group43Var2(_) => HeaderInfo::new(Variation::Group43Var2, I::COUNT_AND_PREFIX_QUALIFIER, true, false),
            PrefixedVariation::Group43Var3(_) => HeaderInfo::new(Variation::Group43Var3, I::COUNT_AND_PREFIX_QUALIFIER, true, false),
            PrefixedVariation::Group43Var4(_) => HeaderInfo::new(Variation::Group43Var4, I::COUNT_AND_PREFIX_QUALIFIER, true, false),
            PrefixedVariation::Group43Var5(_) => HeaderInfo::new(Variation::Group43Var5, I::COUNT_AND_PREFIX_QUALIFIER, true, false),
            PrefixedVariation::Group43Var6(_) => HeaderInfo::new(Variation::Group43Var6, I::COUNT_AND_PREFIX_QUALIFIER, true, false),
            PrefixedVariation::Group43Var7(_) => HeaderInfo::new(Variation::Group43Var7, I::COUNT_AND_PREFIX_QUALIFIER, true, false),
            PrefixedVariation::Group43Var8(_) => HeaderInfo::new(Variation::Group43Var8, I::COUNT_AND_PREFIX_QUALIFIER, true, false),
            PrefixedVariation::Group111VarX(x, _) =>  HeaderInfo::new(Variation::Group111(*x), I::COUNT_AND_PREFIX_QUALIFIER, true, false),
        }
    }
}
