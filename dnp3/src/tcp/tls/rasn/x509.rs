use super::parser::Parser;
use super::types::*;

#[derive(Debug)]
pub(crate) struct Constructed<'a, T> {
    pub(crate) bytes: &'a [u8],
    pub(crate) value: T,
}

impl<'a, T> Constructed<'a, T> {
    pub(crate) fn new(bytes: &'a [u8], value: T) -> Constructed<T> {
        Constructed { bytes, value }
    }
}

#[derive(Debug)]
pub(crate) struct Certificate<'a> {
    // preserve raw bytes for signature validation using Constructed<T>
    pub(crate) tbs_certificate: Constructed<'a, TBSCertificate<'a>>,
    pub(crate) signature_algorithm: AlgorithmIdentifier<'a>,
    pub(crate) signature_value: ASNBitString<'a>,
}

#[derive(Debug)]
pub(crate) struct AlgorithmIdentifier<'a> {
    pub(crate) algorithm: ASNObjectIdentifier,
    pub(crate) parameters: Option<ASNType<'a>>,
}

#[derive(Debug)]
pub(crate) enum Version {
    V1,
    V2,
    V3,
}

#[derive(Debug)]
pub(crate) struct TBSCertificate<'a> {
    pub(crate) version: Version,
    pub(crate) serial_number: ASNInteger<'a>,
    pub(crate) signature: AlgorithmIdentifier<'a>,
    pub(crate) issuer: Name<'a>,
    pub(crate) validity: Validity,
    pub(crate) subject: Name<'a>,
    pub(crate) subject_public_key_info: SubjectPublicKeyInfo<'a>,
    pub(crate) issuer_unique_id: Option<ASNBitString<'a>>,
    pub(crate) subject_unique_id: Option<ASNBitString<'a>>,
    //pub(crate) extensions: Vec<Extension<'a>>,
}

#[derive(Debug)]
pub(crate) struct Validity {
    pub(crate) not_before: UtcTime,
    pub(crate) not_after: UtcTime,
}

impl Validity {
    fn new(not_before: UtcTime, not_after: UtcTime) -> Validity {
        Validity {
            not_before,
            not_after,
        }
    }

    fn parse(input: &[u8]) -> Result<Validity, ASNError> {
        Parser::parse_all(input, |parser| {
            Ok(Validity::new(
                parser.expect::<UtcTime>()?,
                parser.expect::<UtcTime>()?,
            ))
        })
    }

    pub(crate) fn is_valid(&self, now: UtcTime) -> bool {
        now >= self.not_before && now <= self.not_after
    }
}

#[derive(Debug)]
pub(crate) struct AttributeTypeAndValue<'a> {
    pub(crate) id: ASNObjectIdentifier,
    pub(crate) value: ASNType<'a>,
}

impl<'a> AttributeTypeAndValue<'a> {
    fn new(id: ASNObjectIdentifier, value: ASNType<'a>) -> AttributeTypeAndValue<'a> {
        AttributeTypeAndValue { id, value }
    }

    fn parse(input: &'a [u8]) -> Result<AttributeTypeAndValue<'a>, ASNError> {
        Parser::parse_all(input, |parser| {
            Ok(AttributeTypeAndValue::new(
                parser.expect::<ObjectIdentifier>()?,
                parser.expect_any()?,
            ))
        })
    }
}

#[derive(Debug)]
pub(crate) struct RelativeDistinguishedName<'a> {
    values: Vec<AttributeTypeAndValue<'a>>,
}

impl<'a> RelativeDistinguishedName<'a> {
    fn new(values: Vec<AttributeTypeAndValue<'a>>) -> RelativeDistinguishedName<'a> {
        RelativeDistinguishedName { values }
    }

    fn parse(input: &'a [u8]) -> Result<RelativeDistinguishedName<'a>, ASNError> {
        let mut parser = Parser::new(input);

        // expect at least one entry!
        let mut entries: Vec<AttributeTypeAndValue> =
            vec![AttributeTypeAndValue::parse(parser.expect::<Sequence>()?)?];

        while let Some(seq) = parser.expect_or_end::<Sequence>()? {
            entries.push(AttributeTypeAndValue::parse(seq)?);
        }

        Ok(RelativeDistinguishedName::new(entries))
    }
}

#[derive(Debug)]
pub(crate) struct Name<'a> {
    pub(crate) values: Vec<RelativeDistinguishedName<'a>>,
}

impl<'a> Name<'a> {
    fn new(values: Vec<RelativeDistinguishedName<'a>>) -> Name<'a> {
        Name { values }
    }

    fn parse(input: &[u8]) -> Result<Name, ASNError> {
        let mut parser = Parser::new(input);

        let mut values: Vec<RelativeDistinguishedName> = Vec::new();

        while let Some(set) = parser.expect_or_end::<Set>()? {
            values.push(RelativeDistinguishedName::parse(set)?);
        }

        Ok(Name::new(values))
    }
}

#[derive(Debug)]
pub(crate) struct SubjectPublicKeyInfo<'a> {
    pub(crate) algorithm: AlgorithmIdentifier<'a>,
    pub(crate) subject_public_key: ASNBitString<'a>,
}

impl<'a> SubjectPublicKeyInfo<'a> {
    fn new(
        algorithm: AlgorithmIdentifier<'a>,
        subject_public_key: ASNBitString<'a>,
    ) -> SubjectPublicKeyInfo<'a> {
        SubjectPublicKeyInfo {
            algorithm,
            subject_public_key,
        }
    }

    fn parse(input: &[u8]) -> Result<SubjectPublicKeyInfo, ASNError> {
        Parser::parse_all(input, |parser| {
            Ok(SubjectPublicKeyInfo::new(
                AlgorithmIdentifier::parse(parser.expect::<Sequence>()?)?,
                parser.expect::<BitString>()?,
            ))
        })
    }
}

impl<'a> Certificate<'a> {
    pub(crate) fn parse(input: &[u8]) -> Result<Certificate, ASNError> {
        Parser::parse_all(input, |p1| {
            Parser::parse_all(p1.expect::<Sequence>()?, |p2| {
                Ok(Certificate::new(
                    TBSCertificate::parse(p2.expect::<Sequence>()?)?,
                    AlgorithmIdentifier::parse(p2.expect::<Sequence>()?)?,
                    p2.expect::<BitString>()?,
                ))
            })
        })
    }

    pub(crate) fn new(
        tbs_certificate: Constructed<'a, TBSCertificate<'a>>,
        signature_algorithm: AlgorithmIdentifier<'a>,
        signature_value: ASNBitString<'a>,
    ) -> Certificate<'a> {
        Certificate {
            tbs_certificate,
            signature_algorithm,
            signature_value,
        }
    }
}

impl<'a> AlgorithmIdentifier<'a> {
    fn parse(input: &[u8]) -> Result<AlgorithmIdentifier, ASNError> {
        let mut parser = Parser::new(input);

        Ok(AlgorithmIdentifier::new(
            parser.expect::<ObjectIdentifier>()?,
            parser.expect_any_or_end()?,
        ))
    }

    pub(crate) fn new(
        algorithm: ASNObjectIdentifier,
        parameters: Option<ASNType>,
    ) -> AlgorithmIdentifier {
        AlgorithmIdentifier {
            algorithm,
            parameters,
        }
    }
}

impl<'a> TBSCertificate<'a> {
    // certificate really has this many fields, don't warn on lint
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        version: Version,
        serial_number: ASNInteger<'a>,
        signature: AlgorithmIdentifier<'a>,
        issuer: Name<'a>,
        validity: Validity,
        subject: Name<'a>,
        subject_public_key_info: SubjectPublicKeyInfo<'a>,
        issuer_unique_id: Option<ASNBitString<'a>>,
        subject_unique_id: Option<ASNBitString<'a>>,
        _extensions: (),
        //extensions: Vec<Extension<'a>>,
    ) -> TBSCertificate<'a> {
        TBSCertificate {
            version,
            serial_number,
            signature,
            issuer,
            validity,
            subject,
            subject_public_key_info,
            issuer_unique_id,
            subject_unique_id,
            //extensions,
        }
    }

    fn parse(input: &[u8]) -> Result<Constructed<TBSCertificate>, ASNError> {
        fn parse_version(parser: &mut Parser) -> Result<Version, ASNError> {
            match parser.get_optional_explicit_tag_value::<Integer>(0)? {
                Some(value) => match value.as_i32() {
                    Some(0) => Ok(Version::V1),
                    Some(1) => Ok(Version::V2),
                    Some(2) => Ok(Version::V3),
                    Some(x) => Err(ASNError::BadEnumValue("version", x)),
                    None => Err(ASNError::IntegerTooLarge(value.bytes.len())),
                },
                None => Ok(Version::V1),
            }
        }

        fn parse_optional_bitstring<'a>(
            parser: &mut Parser<'a>,
            tag: u8,
        ) -> Result<Option<ASNBitString<'a>>, ASNError> {
            // TODO: check minimum version
            match parser.get_optional_explicit_tag(tag)? {
                Some(tag) => Parser::parse_all(tag.contents, |parser| {
                    Ok(Some(parser.expect::<BitString>()?))
                }),
                None => Ok(None),
            }
        }

        fn parse_extensions(parser: &mut Parser<'_>) -> Result<(), ASNError> {
            //TODO: we should probably parse the extensions to see if a critical one is unknown.

            // We simply parse the optional extension section, without parsing the
            // actual extensions.
            parser.get_optional_explicit_tag(3)?;

            Ok(())
        }

        /*fn parse_extensions<'a>(parser: &mut Parser<'a>) -> Result<Vec<Extension<'a>>, ASNError> {
            // TODO: check minimum version
            let mut extensions: Vec<Extension> = Vec::new();
            if let Some(tag) = parser.get_optional_explicit_tag(3)? {
                let mut parser = Parser::unwrap_outer_sequence(tag.contents)?;
                while let Some(seq) = parser.expect_or_end::<Sequence>()? {
                    extensions.push(Extension::parse(seq)?);
                }
            };
            Ok(extensions)
        }*/

        fn parse_tbs_cert<'a>(parser: &mut Parser<'a>) -> Result<TBSCertificate<'a>, ASNError> {
            Ok(TBSCertificate::new(
                parse_version(parser)?,
                parser.expect::<Integer>()?,
                AlgorithmIdentifier::parse(parser.expect::<Sequence>()?)?,
                Name::parse(parser.expect::<Sequence>()?)?,
                Validity::parse(parser.expect::<Sequence>()?)?,
                Name::parse(parser.expect::<Sequence>()?)?,
                SubjectPublicKeyInfo::parse(parser.expect::<Sequence>()?)?,
                parse_optional_bitstring(parser, 1)?,
                parse_optional_bitstring(parser, 2)?,
                parse_extensions(parser)?,
            ))
        }

        Ok(Constructed::new(
            input,
            Parser::parse_all(input, parse_tbs_cert)?,
        ))
    }
}
