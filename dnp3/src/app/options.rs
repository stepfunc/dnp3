/// Enable or disable the parsing of zero-length strings, e.g., Group 110/111 Variation 0
///
/// This is disabled by default for security reasons as enabling it can allow resource
/// exhaustion attacks.
///
/// This global option is a work-around to preserve API compatability until a future 2.0 release
pub fn parse_zero_length_strings(enabled: bool) {
    super::parse::options::ParseOptions::parse_zero_length_strings(enabled)
}
