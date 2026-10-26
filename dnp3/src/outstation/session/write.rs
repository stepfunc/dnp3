// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use super::*;

impl OutstationSession {
    pub(super) async fn handle_write(
        &mut self,
        seq: Sequence,
        object_headers: HeaderCollection<'_>,
        database: &DatabaseHandle,
    ) -> ResponseHeader {
        let mut iin2 = Iin2::default();
        let mut empty = true;
        let mut selector = Selector::new(FunctionCode::Write);

        for header in object_headers.iter() {
            empty = false;
            iin2 |= match selector.select::<WriteTarget>(header) {
                Ok(header) => self.handle_single_write_header(header, database).await,
                Err(iin2) => iin2,
            };
        }

        selector.log();
        if empty {
            tracing::warn!("empty WRITE request");
        }

        ResponseHeader::solicited(seq, Iin::default() | iin2)
    }

    pub(super) async fn handle_single_write_header(
        &mut self,
        header: WriteHeader<'_>,
        db: &DatabaseHandle,
    ) -> Iin2 {
        match header {
            WriteHeader::Attr(attr) => self.handle_write_attr(attr, db).await,
            WriteHeader::Iin(bits) => self.handle_write_iin(bits),
            WriteHeader::AbsoluteTime(seq) => self.handle_write_abs_time(seq),
            WriteHeader::LastRecordedTime(seq) => self.handle_write_at_last_recorded_time(seq),
            WriteHeader::DeadBands16(items) => self.handle_write_dead_bands(items, db).await,
            WriteHeader::DeadBands32(items) => self.handle_write_dead_bands(items, db).await,
            WriteHeader::DeadBandsFloat(items) => self.handle_write_dead_bands(items, db).await,
        }
    }

    pub(super) async fn handle_write_attr(
        &mut self,
        attr: Attribute<'_>,
        db: &DatabaseHandle,
    ) -> Iin2 {
        // first validate that attribute can be written
        if let Err(err) = db.transaction(|db| db.inner.get_attr_map().can_write(attr)) {
            tracing::warn!("Unable to WRITE attribute: {err}");
            return err.into();
        }

        // next, ask the application to validate and persist the value
        if !self.application.write_device_attr(attr).get().await {
            // validation of the value failed
            return Iin2::PARAMETER_ERROR;
        }

        // this can't fail today: nothing removes an attribute or changes its type once defined
        if let Err(err) = db.transaction(|db| db.inner.get_attr_map().write(attr)) {
            tracing::error!("Unable to store the WRITE of an attribute: {err}");
            return Iin2::PARAMETER_ERROR;
        }
        Iin2::default()
    }

    pub(super) fn handle_write_iin(&mut self, bits: BitSequence) -> Iin2 {
        let mut iin2 = Iin2::default();
        for (value, index) in bits.iter() {
            if index == 7 {
                // restart IIN
                if value {
                    tracing::warn!("ignoring request to write IIN 1.7 to TRUE");
                    iin2 |= Iin2::PARAMETER_ERROR;
                } else {
                    // clear the restart bit
                    self.state.restart_iin_asserted = false;
                    self.info.clear_restart_iin();
                }
            } else {
                tracing::warn!("ignoring write of IIN index {} to value {}", index, value);
                iin2 |= Iin2::PARAMETER_ERROR;
            }
        }
        iin2
    }

    pub(super) fn handle_write_abs_time(&mut self, seq: CountSequence<Group50Var1>) -> Iin2 {
        if let Some(value) = seq.single() {
            match self.application.write_absolute_time(value.time) {
                Err(err) => err.into(),
                Ok(()) => Iin2::default(),
            }
        } else {
            tracing::warn!("request lacks a single g50v1");
            Iin2::PARAMETER_ERROR
        }
    }

    pub(super) fn handle_write_at_last_recorded_time(
        &mut self,
        seq: CountSequence<Group50Var3>,
    ) -> Iin2 {
        let value = if let Some(value) = seq.single() {
            value
        } else {
            tracing::warn!("request didn't have a single g50v3");
            return Iin2::PARAMETER_ERROR;
        };

        let last_recorded_time = if let Some(last_recorded_time) = self.state.last_recorded_time {
            last_recorded_time
        } else {
            tracing::warn!("no previous RECORD_CURRENT_TIME");
            return Iin2::PARAMETER_ERROR;
        };

        let now = tokio::time::Instant::now();
        let delay = if let Some(delay) = now.checked_duration_since(last_recorded_time) {
            delay
        } else {
            tracing::warn!("clock rollback detected (Tokio error)");
            return Iin2::PARAMETER_ERROR;
        };

        let timestamp = if let Some(timestamp) = value.time.checked_add(delay) {
            timestamp
        } else {
            tracing::warn!("calculating current time overflown");
            return Iin2::PARAMETER_ERROR;
        };

        self.state.last_recorded_time = None;
        match self.application.write_absolute_time(timestamp) {
            Err(err) => err.into(),
            Ok(()) => Iin2::default(),
        }
    }

    pub(super) async fn handle_write_analog_deadbands(
        &mut self,
        mut items: impl Iterator<Item = (u16, f64)>,
        db: &DatabaseHandle,
    ) -> Iin2 {
        if !self.application.support_write_analog_dead_bands() {
            return Iin2::OBJECT_UNKNOWN;
        }

        self.application.begin_write_analog_dead_bands();

        let iin = db.transaction(|db| {
            let mut iin2 = Iin2::default();
            for (index, deadband) in items.by_ref() {
                match db.inner.set_analog_deadband(index, deadband) {
                    Some(deadband) => self.application.write_analog_dead_band(index, deadband),
                    None => iin2 |= Iin2::PARAMETER_ERROR,
                }
            }
            iin2
        });

        self.application.end_write_analog_dead_bands().get().await;

        iin
    }

    pub(super) async fn handle_write_ranged_analog_deadbands<V>(
        &mut self,
        items: RangedSequence<'_, V>,
        db: &DatabaseHandle,
    ) -> Iin2
    where
        V: FixedSizeVariation + Into<f64>,
    {
        self.handle_write_analog_deadbands(
            items.iter().map(|(value, index)| (index, value.into())),
            db,
        )
        .await
    }

    pub(super) async fn handle_write_prefixed_analog_deadbands<I, V>(
        &mut self,
        items: CountSequence<'_, Prefix<I, V>>,
        db: &DatabaseHandle,
    ) -> Iin2
    where
        I: Index,
        V: FixedSizeVariation + Into<f64>,
    {
        self.handle_write_analog_deadbands(
            items
                .iter()
                .map(|x| (x.index.widen_to_u16(), x.value.into())),
            db,
        )
        .await
    }

    pub(super) async fn handle_write_dead_bands<V>(
        &mut self,
        items: DeadBands<'_, V>,
        db: &DatabaseHandle,
    ) -> Iin2
    where
        V: FixedSizeVariation + Into<f64>,
    {
        match items {
            DeadBands::Range(seq) => self.handle_write_ranged_analog_deadbands(seq, db).await,
            DeadBands::OneBytePrefix(seq) => {
                self.handle_write_prefixed_analog_deadbands(seq, db).await
            }
            DeadBands::TwoBytePrefix(seq) => {
                self.handle_write_prefixed_analog_deadbands(seq, db).await
            }
        }
    }
}

impl From<Group34Var1> for f64 {
    fn from(x: Group34Var1) -> Self {
        x.value.into()
    }
}

impl From<Group34Var2> for f64 {
    fn from(x: Group34Var2) -> Self {
        x.value.into()
    }
}

impl From<Group34Var3> for f64 {
    fn from(x: Group34Var3) -> Self {
        x.value.into()
    }
}
