//! The JSON wsx writes with `--json` (D-08): a value tree rendered compact,
//! keys in the order they were built, one object per run.
//!
//! No crate: the output is a handful of objects of strings, integers and
//! booleans, and the binary takes no dependency for it. serde_json, a
//! dev-dependency, reads every rendering back in the tests, so the escaping
//! is checked against an independent parser.
//!
//! Byte values (paths, ref names) go through `bytes` (D-09): a JSON string
//! when the bytes are UTF-8, else `{"hex": <lowercase hex of the bytes>}`.
//! Nothing is converted lossily, so a consumer always gets the exact bytes.

#![cfg_attr(not(test), expect(dead_code, reason = "first caller in 05-02 Task 2"))]

/// A JSON value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    /// `null`.
    Null,
    /// `true` or `false`.
    Bool(bool),
    /// An integer.
    Int(i64),
    /// A string.
    Str(String),
    /// An array, in order.
    Array(Vec<Value>),
    /// An object, its keys written in this order.
    Object(Vec<(&'static str, Value)>),
}

/// `raw` as D-09 writes a byte value: the string when it is UTF-8, else an
/// object holding its lowercase hex.
pub fn bytes(raw: &[u8]) -> Value {
    match std::str::from_utf8(raw) {
        Ok(text) => Value::Str(text.to_owned()),
        Err(_) => Value::Object(vec![("hex", Value::Str(hex(raw)))]),
    }
}

/// The lowercase hex digits.
const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";

/// `raw` in lowercase hex, two digits per byte.
fn hex(raw: &[u8]) -> String {
    let mut out = String::with_capacity(raw.len() * 2);
    for byte in raw {
        out.push(char::from(HEX_DIGITS[usize::from(byte >> 4)]));
        out.push(char::from(HEX_DIGITS[usize::from(byte & 0x0f)]));
    }
    out
}

impl Value {
    /// The value as compact JSON, with no trailing newline.
    pub fn render(&self) -> String {
        let mut out = String::new();
        self.write(&mut out);
        out
    }

    /// Appends the value's JSON to `out`.
    fn write(&self, out: &mut String) {
        match self {
            Value::Null => out.push_str("null"),
            Value::Bool(true) => out.push_str("true"),
            Value::Bool(false) => out.push_str("false"),
            Value::Int(number) => out.push_str(&number.to_string()),
            Value::Str(text) => write_string(out, text),
            Value::Array(items) => {
                out.push('[');
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    item.write(out);
                }
                out.push(']');
            }
            Value::Object(members) => {
                out.push('{');
                for (index, (key, value)) in members.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    write_string(out, key);
                    out.push(':');
                    value.write(out);
                }
                out.push('}');
            }
        }
    }
}

/// The first character JSON lets a string hold raw; every one below it is
/// escaped (RFC 8259 section 7).
const FIRST_RAW: char = ' ';

/// Appends `text` as a JSON string: quote and backslash escaped, the short
/// escapes for backspace, form feed, newline, carriage return and tab, a
/// six-character lowercase escape for every other control below 0x20, and
/// every other character as its UTF-8.
fn write_string(out: &mut String, text: &str) {
    out.push('"');
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            control if control < FIRST_RAW => {
                out.push_str(&format!("\\u{:04x}", u32::from(control)));
            }
            other => out.push(other),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU32, Ordering};

    use proptest::prelude::*;
    use proptest::test_runner::Config;

    use super::*;

    /// The cases each property runs.
    const PROPERTY_CASES: u32 = 2048;

    /// The config every `proptest!` block here takes: `PROPERTY_CASES`
    /// cases and no failure file, since the container mounts the checkout
    /// read-only. A failure prints its minimal input; it becomes a named
    /// regression test, not a persisted seed.
    fn property_config() -> Config {
        Config {
            cases: PROPERTY_CASES,
            failure_persistence: None,
            ..Config::default()
        }
    }

    /// `rendered` parsed by serde_json.
    fn parsed(rendered: &str) -> serde_json::Value {
        serde_json::from_str(rendered)
            .unwrap_or_else(|err| panic!("serde_json rejects {rendered:?}: {err}"))
    }

    /// The bytes `hex` spells, two lowercase digits per byte, or `None`
    /// when it is not that form.
    fn unhex(hex: &str) -> Option<Vec<u8>> {
        let digits = hex.as_bytes();
        if !digits.len().is_multiple_of(2)
            || !digits
                .iter()
                .all(|d| matches!(d, b'0'..=b'9' | b'a'..=b'f'))
        {
            return None;
        }
        digits
            .chunks(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok())
            .collect()
    }

    /// The seven characters with a two-character escape get it, the other
    /// controls below 0x20 a six-character lowercase escape, and DEL and a
    /// non-ASCII character are written raw.
    #[test]
    fn strings_escape_as_rfc8259() {
        let value = Value::Str("\"\\\n\t\r\u{8}\u{c}\u{1}\u{1f}\u{7f}\u{e9}".to_owned());
        let expected = [r#""\"\\\n\t\r\b\f\u0001\u001f"#, "\u{7f}\u{e9}\""].concat();
        assert_eq!(value.render(), expected);
    }

    /// Bytes that are not UTF-8 render as their hex; UTF-8 bytes, a newline
    /// and the empty value included, as a string.
    #[test]
    fn non_utf8_bytes_render_as_hex() {
        assert_eq!(bytes(b"\xff.md").render(), r#"{"hex":"ff2e6d64"}"#);
        assert_eq!(bytes(b"a\nb").render(), r#""a\nb""#);
        assert_eq!(bytes(b"").render(), r#""""#);
    }

    /// Integers render in decimal, and serde_json reads back the same i64 at
    /// both limits.
    #[test]
    fn integers_at_the_i64_limits_round_trip() {
        for number in [i64::MIN, i64::MAX, 0, -1] {
            let rendered = Value::Int(number).render();
            assert_eq!(rendered, number.to_string());
            let back: i64 = serde_json::from_str(&rendered)
                .unwrap_or_else(|err| panic!("serde_json rejects {rendered:?}: {err}"));
            assert_eq!(back, number);
        }
    }

    /// An object's keys come out in the order they were built, not sorted.
    #[test]
    fn object_keys_keep_their_order() {
        let value = Value::Object(vec![
            ("z", Value::Null),
            ("a", Value::Bool(true)),
            ("m", Value::Array(Vec::new())),
        ]);
        assert_eq!(value.render(), r#"{"z":null,"a":true,"m":[]}"#);
    }

    /// A property under `property_config()` runs at least `PROPERTY_CASES`
    /// cases, so a lowered `PROPTEST_CASES` cannot thin the properties below.
    #[test]
    fn property_runs_its_configured_cases() {
        let ran = AtomicU32::new(0);
        proptest!(property_config(), |(_byte in any::<u8>())| {
            ran.fetch_add(1, Ordering::Relaxed);
        });
        let ran = ran.load(Ordering::Relaxed);
        assert!(
            ran >= PROPERTY_CASES,
            "a property under property_config() ran {ran} cases, below {PROPERTY_CASES}; \
             PROPTEST_CASES lowers the count, and is {:?} here",
            std::env::var_os("PROPTEST_CASES")
        );
    }

    proptest! {
        #![proptest_config(property_config())]

        /// serde_json reads any string's rendering back as the same string.
        #[test]
        fn any_string_round_trips_through_serde_json(text in any::<String>()) {
            let back = parsed(&Value::Str(text.clone()).render());
            prop_assert_eq!(back, serde_json::Value::String(text));
        }

        /// Any bytes read back as the same string when UTF-8, else as an
        /// object whose only key is `hex`, spelling the same bytes.
        #[test]
        fn any_bytes_round_trip_through_serde_json(
            raw in proptest::collection::vec(any::<u8>(), 0..32)
        ) {
            let back = parsed(&bytes(&raw).render());
            match std::str::from_utf8(&raw) {
                Ok(text) => prop_assert_eq!(back, serde_json::Value::String(text.to_owned())),
                Err(_) => {
                    let object = back.as_object().expect("non-UTF-8 bytes render as an object");
                    prop_assert_eq!(object.len(), 1);
                    let hex = object
                        .get("hex")
                        .and_then(serde_json::Value::as_str)
                        .expect("a hex string");
                    prop_assert_eq!(unhex(hex), Some(raw.clone()));
                }
            }
        }
    }
}
