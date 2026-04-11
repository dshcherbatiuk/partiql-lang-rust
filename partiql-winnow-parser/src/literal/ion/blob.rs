//! Ion blob literal parsing — `{{ base64 }}`.
//!
//! Ion blobs are double-curly-delimited, base64-encoded byte strings.
//! Whitespace is permitted inside the delimiters and stripped.
//!
//! ```text
//! blob ::= '{{' ([A-Za-z0-9+/=] | whitespace)* '}}'
//! ```
//!
//! Returns the canonical (whitespace-stripped) base64 payload as a `String`,
//! ready to be wrapped in `Lit::TypedLit(payload, Type::BlobType)` matching
//! the LALRPOP `TypedLiteral` shape so downstream consumers stay unchanged.

use winnow::prelude::*;

/// Parse an Ion blob literal, returning the whitespace-stripped base64 payload.
///
/// On entry, `input` must start with `{{`. On success, `input` advances past
/// the closing `}}` and the function returns the canonical payload string.
pub fn ion_blob_payload(input: &mut &str) -> PResult<String> {
    let bytes = input.as_bytes();
    if bytes.len() < 4 || &bytes[..2] != b"{{" {
        return Err(backtrack());
    }

    // Find the closing `}}`. The base64 alphabet [A-Za-z0-9+/=] plus
    // whitespace cannot contain `}`, so the first `}` followed by another
    // `}` must be the closer.
    let mut i = 2usize;
    let payload_end;
    loop {
        if i + 1 >= bytes.len() {
            return Err(backtrack());
        }
        if bytes[i] == b'}' && bytes[i + 1] == b'}' {
            payload_end = i;
            break;
        }
        i += 1;
    }

    let raw = &input[2..payload_end];
    let payload: String = raw
        .chars()
        .filter(|c| !c.is_ascii_whitespace())
        .collect();

    *input = &input[payload_end + 2..];
    Ok(payload)
}

#[inline]
fn backtrack() -> winnow::error::ErrMode<winnow::error::ContextError> {
    winnow::error::ErrMode::Backtrack(winnow::error::ContextError::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_blob() {
        let mut input = "{{dGVzdCBkYXRh}}rest";
        let payload = ion_blob_payload(&mut input).unwrap();
        assert_eq!(payload, "dGVzdCBkYXRh");
        assert_eq!(input, "rest");
    }

    #[test]
    fn strips_inner_whitespace() {
        let mut input = "{{  dGVzdCBkYXRh  }}";
        let payload = ion_blob_payload(&mut input).unwrap();
        assert_eq!(payload, "dGVzdCBkYXRh");
    }

    #[test]
    fn handles_empty_blob() {
        let mut input = "{{}}";
        let payload = ion_blob_payload(&mut input).unwrap();
        assert_eq!(payload, "");
    }

    #[test]
    fn handles_padding_chars() {
        let mut input = "{{aGVsbG8=}}";
        let payload = ion_blob_payload(&mut input).unwrap();
        assert_eq!(payload, "aGVsbG8=");
    }

    #[test]
    fn rejects_unterminated() {
        let mut input = "{{dGVzdA";
        assert!(ion_blob_payload(&mut input).is_err());
    }

    #[test]
    fn rejects_single_curly() {
        let mut input = "{a: 1}";
        assert!(ion_blob_payload(&mut input).is_err());
    }
}
