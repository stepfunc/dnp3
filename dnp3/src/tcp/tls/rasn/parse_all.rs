use super::parser::Parser;
use super::types::{ASNError, ASNType};

pub(crate) trait ParseHandler {
    fn begin_constructed(&mut self);
    fn end_constructed(&mut self);
    fn on_type(&mut self, asn: &ASNType);
    fn on_error(&mut self, err: &ASNError);
}

pub(crate) fn parse_all(input: &[u8], handler: &mut dyn ParseHandler) -> Result<(), ASNError> {
    for result in Parser::new(input) {
        match result {
            Err(err) => {
                handler.on_error(&err);
                return Err(err);
            }
            Ok(asn) => {
                handler.on_type(&asn);
                match asn {
                    ASNType::Sequence(wrapper) => {
                        handler.begin_constructed();
                        parse_all(wrapper.value, handler)?;
                        handler.end_constructed();
                    }
                    ASNType::ExplicitTag(wrapper) => {
                        handler.begin_constructed();
                        parse_all(wrapper.value.contents, handler)?;
                        handler.end_constructed();
                    }
                    ASNType::Set(wrapper) => {
                        handler.begin_constructed();
                        parse_all(wrapper.value, handler)?;
                        handler.end_constructed();
                    }
                    _ => (),
                }
            }
        }
    }

    Ok(())
}
