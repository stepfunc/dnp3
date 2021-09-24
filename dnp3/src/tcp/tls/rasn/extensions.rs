use parser::Parser;
use printer::{print_type, LinePrinter, Printable};
use std::fmt::Debug;
use types::{
    ASNError, ASNObjectIdentifier, BitString, Boolean, ExplicitTag, IA5String, Integer,
    ObjectIdentifier, OctetString,
};

#[derive(Debug)]
pub struct Extension<'a> {
    pub extn_id: ASNObjectIdentifier,
    pub critical: bool,
    pub content: Box<dyn SpecificExtension + 'a>,
}

impl<'a> Extension<'a> {
    pub fn new(
        extn_id: ASNObjectIdentifier,
        critical: bool,
        content: Box<dyn SpecificExtension + 'a>,
    ) -> Extension<'a> {
        Extension {
            extn_id,
            critical,
            content,
        }
    }

    pub fn parse(input: &'a [u8]) -> Result<Extension, ASNError> {
        Parser::parse_all(input, |parser| {
            let oid = parser.expect::<ObjectIdentifier>()?;
            let is_critical = parser.get_optional_or_default::<Boolean>(false)?;
            let raw_content = parser.expect::<OctetString>()?;

            let content: Box<dyn SpecificExtension> = match oid.values() {
                [2, 5, 29, 14] => Box::new(SubjectKeyIdentifier::parse(raw_content)?),
                [2, 5, 29, 15] => Box::new(KeyUsage::parse(raw_content)?),
                [2, 5, 29, 17] => Box::new(SubjectAlternativeName::parse(raw_content)?),
                [2, 5, 29, 19] => Box::new(BasicConstraints::parse(raw_content)?),
                [2, 5, 29, 37] => Box::new(ExtendedKeyUsage::parse(raw_content)?),
                _ => Box::new(UnknownExtension::new(raw_content)),
            };

            Ok(Extension::new(oid, is_critical, content))
        })
    }
}

impl<'a> Printable for Extension<'a> {
    fn print(&self, printer: &mut dyn LinePrinter) {
        printer.begin_line();
        printer.println_str(self.content.get_name());
        printer.begin_type();
        printer.begin_line();
        printer.println_fmt(&format_args!("extension id: {}", self.extn_id));
        printer.begin_line();
        printer.println_fmt(&format_args!("critical: {}", self.critical));
        self.content.print(printer);
        printer.end_type();
    }
}

pub trait SpecificExtension: Debug + Printable {
    fn get_name(&self) -> &'static str;
}

#[derive(Debug)]
pub struct UnknownExtension<'a> {
    pub extn_value: &'a [u8],
}

impl<'a> SpecificExtension for UnknownExtension<'a> {
    fn get_name(&self) -> &'static str {
        "Unknown extension"
    }
}

impl<'a> UnknownExtension<'a> {
    fn new(extn_value: &'a [u8]) -> UnknownExtension<'a> {
        UnknownExtension { extn_value }
    }
}

impl<'a> Printable for UnknownExtension<'a> {
    fn print(&self, printer: &mut dyn LinePrinter) {
        print_type("raw content", &self.extn_value, printer);
    }
}

#[derive(Debug)]
pub struct SubjectKeyIdentifier<'a> {
    pub key_identifier: &'a [u8],
}

impl<'a> SpecificExtension for SubjectKeyIdentifier<'a> {
    fn get_name(&self) -> &'static str {
        "Subject Key Identifier"
    }
}

impl<'a> SubjectKeyIdentifier<'a> {
    fn parse(input: &[u8]) -> Result<SubjectKeyIdentifier, ASNError> {
        let mut parser = Parser::new(input);
        let key_identifier = parser.expect::<OctetString>()?;
        Ok(SubjectKeyIdentifier { key_identifier })
    }
}

impl<'a> Printable for SubjectKeyIdentifier<'a> {
    fn print(&self, printer: &mut dyn LinePrinter) {
        print_type("key identifier", &self.key_identifier, printer);
    }
}

#[derive(Debug)]
pub struct KeyUsage {
    pub digital_signature: bool,
    pub content_commitment: bool,
    pub key_encipherment: bool,
    pub data_encipherment: bool,
    pub key_agreement: bool,
    pub key_cert_sign: bool,
    pub crl_sign: bool,
    pub encipher_only: bool,
    pub decipher_only: bool,
}

impl SpecificExtension for KeyUsage {
    fn get_name(&self) -> &'static str {
        "Key Usage"
    }
}

impl KeyUsage {
    fn parse(input: &[u8]) -> Result<KeyUsage, ASNError> {
        let mut parser = Parser::new(input);
        let bit_string = parser.expect::<BitString>()?;

        let mut key_usage = KeyUsage {
            digital_signature: false,
            content_commitment: false,
            key_encipherment: false,
            data_encipherment: false,
            key_agreement: false,
            key_cert_sign: false,
            crl_sign: false,
            encipher_only: false,
            decipher_only: false,
        };
        let mut offset = 0;
        for bit in bit_string.iter() {
            match offset {
                0 => key_usage.digital_signature = bit,
                1 => key_usage.content_commitment = bit,
                2 => key_usage.key_encipherment = bit,
                3 => key_usage.data_encipherment = bit,
                4 => key_usage.key_agreement = bit,
                5 => key_usage.key_cert_sign = bit,
                6 => key_usage.crl_sign = bit,
                7 => key_usage.encipher_only = bit,
                8 => key_usage.decipher_only = bit,
                _ => {}
            }
            offset += offset;
        }
        Ok(key_usage)
    }
}

impl Printable for KeyUsage {
    fn print(&self, printer: &mut dyn LinePrinter) {
        fn print_usage(description: &str, printer: &mut dyn LinePrinter) {
            printer.begin_type();
            printer.begin_line();
            printer.println_str(description);
            printer.end_type();
        }

        printer.begin_line();
        printer.println_str("usages:");

        if self.digital_signature {
            print_usage("digital signature", printer)
        }
        if self.content_commitment {
            print_usage("content commitment", printer)
        }
        if self.key_encipherment {
            print_usage("key encipherment", printer)
        }
        if self.data_encipherment {
            print_usage("data encipherment", printer)
        }
        if self.key_agreement {
            print_usage("key agreement", printer)
        }
        if self.key_cert_sign {
            print_usage("key cert sign", printer)
        }
        if self.crl_sign {
            print_usage("crl sign", printer)
        }
        if self.encipher_only {
            print_usage("encipher only", printer)
        }
        if self.decipher_only {
            print_usage("decipher only", printer)
        }
    }
}

#[derive(Debug)]
pub enum GeneralName<'a> {
    OtherName(&'a [u8]),
    Rfc822Name(&'a str),
    DnsName(&'a str),
    X400Address,
    DirectoryName,
    EdiPartyName,
    UniformResourceIdentifier(&'a str),
    IpAddress(&'a [u8]),
    RegisteredId(ASNObjectIdentifier),
}

impl<'a> Printable for GeneralName<'a> {
    fn print(&self, printer: &mut dyn LinePrinter) {
        match self {
            GeneralName::Rfc822Name(value) => {
                printer.println_fmt(&format_args!("RFC822 Name: {}", value))
            }
            GeneralName::DnsName(value) => {
                printer.println_fmt(&format_args!("DNS Name: {}", value))
            }
            GeneralName::UniformResourceIdentifier(value) => {
                printer.println_fmt(&format_args!("Uniform Resource Identifier: {}", value))
            }
            GeneralName::IpAddress(value) => print_type("IP Address", value, printer),
            GeneralName::RegisteredId(value) => {
                printer.println_fmt(&format_args!("Registered ID: {}", value))
            }
            _ => printer.println_str("Unsupported name type"),
        }
    }
}

#[derive(Debug)]
pub struct SubjectAlternativeName<'a> {
    pub names: Vec<GeneralName<'a>>,
}

impl<'a> SpecificExtension for SubjectAlternativeName<'a> {
    fn get_name(&self) -> &'static str {
        "Subject Alternative Name"
    }
}

impl<'a> SubjectAlternativeName<'a> {
    fn parse(input: &[u8]) -> Result<SubjectAlternativeName, ASNError> {
        let mut parser = Parser::unwrap_outer_sequence(input)?;
        let mut names: Vec<GeneralName> = Vec::new();

        while let Some(tag) = parser.expect_or_end::<ExplicitTag>()? {
            let mut parser = Parser::new(tag.contents);
            match tag.value {
                // TODO: parse the other types
                1 => names.push(GeneralName::Rfc822Name(
                    parser.parse_implicit::<IA5String>()?,
                )),
                2 => names.push(GeneralName::DnsName(parser.parse_implicit::<IA5String>()?)),
                6 => names.push(GeneralName::UniformResourceIdentifier(
                    parser.parse_implicit::<IA5String>()?,
                )),
                7 => names.push(GeneralName::IpAddress(
                    parser.parse_implicit::<OctetString>()?,
                )),
                8 => names.push(GeneralName::RegisteredId(
                    parser.parse_implicit::<ObjectIdentifier>()?,
                )),

                _ => return Err(ASNError::UnexpectedTag(tag.value)),
            };
        }

        Ok(SubjectAlternativeName { names })
    }
}

impl<'a> Printable for SubjectAlternativeName<'a> {
    fn print(&self, printer: &mut dyn LinePrinter) {
        printer.begin_line();
        printer.println_str("names:");
        printer.begin_type();
        for name in &self.names {
            printer.begin_line();
            name.print(printer);
        }
        printer.end_type();
    }
}

#[derive(Debug)]
pub struct BasicConstraints {
    pub ca: bool,
    pub path_length_constraint: Option<i32>,
}

impl SpecificExtension for BasicConstraints {
    fn get_name(&self) -> &'static str {
        "Basic Constraints"
    }
}

impl BasicConstraints {
    fn parse(input: &[u8]) -> Result<BasicConstraints, ASNError> {
        let mut parser = Parser::unwrap_outer_sequence(input)?;
        let ca = parser.get_optional_or_default::<Boolean>(false)?;
        let constraint = parser.get_optional::<Integer>()?;
        let constraint = match constraint {
            Some(value) => match value.as_i32() {
                Some(value) => Ok(Some(value)),
                None => Err(ASNError::IntegerTooLarge(value.bytes.len())),
            },
            None => Ok(None),
        }?;

        Ok(BasicConstraints {
            ca,
            path_length_constraint: constraint,
        })
    }
}

impl Printable for BasicConstraints {
    fn print(&self, printer: &mut dyn LinePrinter) {
        printer.begin_line();
        printer.println_fmt(&format_args!("CA: {}", self.ca));
        if let Some(constraint) = self.path_length_constraint {
            printer.begin_line();
            printer.println_fmt(&format_args!("Path Length Contraint: {}", constraint));
        }
    }
}

#[derive(Debug)]
pub enum ExtendedKeyUsagePurpose {
    ServerAuth,
    ClientAuth,
    CodeSigning,
    EmailProtection,
    TimeStamping,
    OCSPSigning,
}

impl ExtendedKeyUsagePurpose {
    pub fn from_id(oid: &ASNObjectIdentifier) -> Option<ExtendedKeyUsagePurpose> {
        match oid.values() {
            [1, 3, 6, 1, 5, 5, 7, 3, 1] => Some(ExtendedKeyUsagePurpose::ServerAuth),
            [1, 3, 6, 1, 5, 5, 7, 3, 2] => Some(ExtendedKeyUsagePurpose::ClientAuth),
            [1, 3, 6, 1, 5, 5, 7, 3, 3] => Some(ExtendedKeyUsagePurpose::CodeSigning),
            [1, 3, 6, 1, 5, 5, 7, 3, 4] => Some(ExtendedKeyUsagePurpose::EmailProtection),
            [1, 3, 6, 1, 5, 5, 7, 3, 8] => Some(ExtendedKeyUsagePurpose::TimeStamping),
            [1, 3, 6, 1, 5, 5, 7, 3, 9] => Some(ExtendedKeyUsagePurpose::OCSPSigning),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct ExtendedKeyUsage {
    pub ext_key_usages: Vec<ExtendedKeyUsagePurpose>,
}

impl SpecificExtension for ExtendedKeyUsage {
    fn get_name(&self) -> &'static str {
        "Extended Key Usage"
    }
}

impl ExtendedKeyUsage {
    fn parse(input: &[u8]) -> Result<ExtendedKeyUsage, ASNError> {
        let mut parser = Parser::unwrap_outer_sequence(input)?;
        let mut purposes: Vec<ExtendedKeyUsagePurpose> = Vec::new();

        while let Some(oid) = parser.expect_or_end::<ObjectIdentifier>()? {
            match ExtendedKeyUsagePurpose::from_id(&oid) {
                Some(purpose) => purposes.push(purpose),
                None => return Err(ASNError::UnexpectedOid(oid)),
            }
        }

        Ok(ExtendedKeyUsage {
            ext_key_usages: purposes,
        })
    }
}

impl Printable for ExtendedKeyUsage {
    fn print(&self, printer: &mut dyn LinePrinter) {
        printer.begin_line();
        printer.println_str("usages:");
        printer.begin_type();
        for purpose in &self.ext_key_usages {
            printer.begin_line();
            printer.println_fmt(&format_args!("{:?}", purpose));
        }
        printer.end_type();
    }
}
