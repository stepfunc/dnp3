// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use super::*;

impl MasterSession {
    /// Send a CONFIRM with this control field: solicited or unsolicited, and its sequence
    pub(super) async fn send_confirm<T: Transport>(
        &mut self,
        io: &mut PhysLayer,
        dest: FragmentAddr,
        control: ControlField,
        writer: &mut TransportWriter<T>,
    ) -> Result<(), RunError> {
        let mut cursor = self.tx_buffer.write_cursor();
        write::start_request(control, FunctionCode::Confirm, &mut cursor)
            .map_err(LinkError::from)?;
        let write = writer.write(io, self.decode_level, dest, cursor.written());
        unless_shutdown(&mut self.messages, write).await
    }

    pub(super) async fn send_request<T: Transport, U>(
        &mut self,
        io: &mut PhysLayer,
        addr: FragmentAddr,
        request: &U,
        writer: &mut TransportWriter<T>,
    ) -> Result<Sequence, TaskErrorKind>
    where
        U: RequestWriter,
    {
        // format the request
        let association = self.associations.get_mut(addr.link)?;
        let seq = association.increment_seq();
        let mut cursor = self.tx_buffer.write_cursor();
        let mut hw =
            write::start_request(ControlField::request(seq), request.function(), &mut cursor)?;
        request.write(&mut hw)?;
        let write = writer.write(io, self.decode_level, addr, cursor.written());
        unless_shutdown(&mut self.messages, write).await?;
        Ok(seq)
    }
}
