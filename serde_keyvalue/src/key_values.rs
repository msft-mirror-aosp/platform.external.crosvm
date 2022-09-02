// Copyright 2022 The Chromium OS Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::borrow::Cow;
use std::fmt;
use std::fmt::Debug;
use std::fmt::Display;
use std::num::ParseIntError;

use nom::branch::alt;
use nom::bytes::complete::escaped_transform;
use nom::bytes::complete::is_not;
use nom::bytes::complete::tag;
use nom::bytes::complete::take_while;
use nom::bytes::complete::take_while1;
use nom::character::complete::alphanumeric1;
use nom::character::complete::anychar;
use nom::character::complete::char;
use nom::character::complete::none_of;
use nom::combinator::map;
use nom::combinator::map_res;
use nom::combinator::opt;
use nom::combinator::recognize;
use nom::combinator::value;
use nom::combinator::verify;
use nom::sequence::delimited;
use nom::sequence::pair;
use nom::sequence::tuple;
use nom::AsChar;
use nom::Finish;
use nom::IResult;
use num_traits::Num;
use remain::sorted;
use serde::de;
use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
#[sorted]
#[non_exhaustive]
#[allow(missing_docs)]
/// Different kinds of errors that can be returned by the parser.
pub enum ErrorKind {
    #[error("unexpected end of input")]
    Eof,
    #[error("expected a boolean")]
    ExpectedBoolean,
    #[error("expected ','")]
    ExpectedComma,
    #[error("expected '='")]
    ExpectedEqual,
    #[error("expected an identifier")]
    ExpectedIdentifier,
    #[error("expected a string")]
    ExpectedString,
    #[error("\" and ' can only be used in quoted strings")]
    InvalidCharInString,
    #[error("invalid characters for number or number does not fit into its destination type")]
    InvalidNumber,
    #[error("serde error: {0}")]
    SerdeError(String),
    #[error("remaining characters in input")]
    TrailingCharacters,
}

/// Error that may be thown while parsing a key-values string.
#[derive(Debug, Error, PartialEq)]
pub struct ParseError {
    /// Detailed error that occurred.
    pub kind: ErrorKind,
    /// Index of the error in the input string.
    pub pos: usize,
}

impl Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            ErrorKind::SerdeError(s) => write!(f, "{}", s),
            _ => write!(f, "{} at position {}", self.kind, self.pos),
        }
    }
}

impl de::Error for ParseError {
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        Self {
            kind: ErrorKind::SerdeError(msg.to_string()),
            pos: 0,
        }
    }
}

type Result<T> = std::result::Result<T, ParseError>;

/// Nom parser for valid strings.
///
/// A string can be quoted (using single or double quotes) or not. If it is not quoted, the string
/// is assumed to continue until the next ',' separating character. If it is escaped, it continues
/// until the next non-escaped quote.
///
/// The returned value is a slice into the current input if no characters to unescape were met,
/// or a fully owned string if we had to unescape some characters.
fn any_string(s: &str) -> IResult<&str, Cow<str>> {
    // Double-quoted strings may escape " and \ characters. Since escaped strings are modified,
    // we need to return an owned `String` instead of just a slice in the input string.
    let double_quoted = delimited(
        char('"'),
        alt((
            map(
                escaped_transform(
                    none_of(r#"\""#),
                    '\\',
                    alt((value("\"", char('"')), value("\\", char('\\')))),
                ),
                Cow::Owned,
            ),
            map(tag(""), Cow::Borrowed),
        )),
        char('"'),
    );

    // Single-quoted strings do not escape characters.
    let single_quoted = map(
        delimited(char('\''), alt((is_not(r#"'"#), tag(""))), char('\'')),
        Cow::Borrowed,
    );

    // Unquoted strings end with the next comma and may not contain a quote character or be empty.
    let unquoted = map(
        take_while1(|c: char| c != ',' && c != '"' && c != '\''),
        Cow::Borrowed,
    );

    alt((double_quoted, single_quoted, unquoted))(s)
}

/// Nom parser for valid positive of negative numbers.
///
/// Hexadecimal, octal, and binary values can be specified with the `0x`, `0o` and `0b` prefixes.
fn any_number<T>(s: &str) -> IResult<&str, T>
where
    T: Num<FromStrRadixErr = ParseIntError>,
{
    // Parses the number input and returns a tuple including the number itself (with its sign) and
    // its radix.
    //
    // We move this non-generic part into its own function so it doesn't get monomorphized, which
    // would increase the binary size more than needed.
    fn parse_number(s: &str) -> IResult<&str, (Cow<str>, u32)> {
        // Recognizes the sign prefix.
        let sign = char('-');

        // Recognizes the radix prefix.
        let radix = alt((
            value(16, tag("0x")),
            value(8, tag("0o")),
            value(2, tag("0b")),
        ));

        // Chain of parsers: sign (optional) and radix (optional), then sequence of alphanumerical
        // characters.
        //
        // Then we take all 3 recognized elements and turn them into the string and radix to pass to
        // `from_str_radix`.
        map(
            tuple((opt(sign), opt(radix), alphanumeric1)),
            |(sign, radix, number)| {
                // If the sign was specified, we need to build a string that contains it for
                // `from_str_radix` to parse the number accurately. Otherwise, simply borrow the
                // remainder of the input.
                let num_string = if let Some(sign) = sign {
                    Cow::Owned(sign.to_string() + number)
                } else {
                    Cow::Borrowed(number)
                };

                (num_string, radix.unwrap_or(10))
            },
        )(s)
    }

    map_res(parse_number, |(num_string, radix)| {
        T::from_str_radix(&num_string, radix)
    })(s)
}

/// Nom parser for booleans.
fn any_bool(s: &str) -> IResult<&str, bool> {
    let mut boolean = alt((value(true, tag("true")), value(false, tag("false"))));

    boolean(s)
}

/// Nom parser for identifiers. An identifier may contain any alphanumeric character, as well as
/// '_' and '-' at any place excepted the first one which cannot be '-'.
///
/// Usually identifiers are not allowed to start with a number, but we chose to allow this
/// here otherwise options like "mode=2d" won't parse if "2d" is an alias for an enum variant.
fn any_identifier(s: &str) -> IResult<&str, &str> {
    let mut ident = recognize(pair(
        verify(anychar, |&c| c.is_alphanum() || c == '_'),
        take_while(|c: char| c.is_alphanum() || c == '_' || c == '-'),
    ));

    ident(s)
}

/// Serde deserializer for key-values strings.
struct KeyValueDeserializer<'de> {
    /// Full input originally received for parsing.
    original_input: &'de str,
    /// Input currently remaining to parse.
    input: &'de str,
    /// If set, then `deserialize_identifier` will take and return its content the next time it is
    /// called instead of trying to parse an identifier from the input. This is needed to allow the
    /// name of the first field of a struct to be omitted, e.g.
    ///
    ///   --block "/path/to/disk.img,ro=true"
    ///
    /// instead of
    ///
    ///   --block "path=/path/to/disk.img,ro=true"
    next_identifier: Option<&'de str>,
    /// Whether the '=' sign has been parsed after a key. The absence of '=' is only valid for
    /// boolean fields, in which case the field's value will be `true`.
    has_equal: bool,
}

impl<'de> From<&'de str> for KeyValueDeserializer<'de> {
    fn from(input: &'de str) -> Self {
        Self {
            original_input: input,
            input,
            next_identifier: None,
            has_equal: false,
        }
    }
}

impl<'de> KeyValueDeserializer<'de> {
    /// Return an `kind` error for the current position of the input.
    fn error_here(&self, kind: ErrorKind) -> ParseError {
        ParseError {
            kind,
            pos: self.original_input.len() - self.input.len(),
        }
    }

    /// Returns the next char in the input string without consuming it, or None
    /// if we reached the end of input.
    fn peek_char(&self) -> Option<char> {
        self.input.chars().next()
    }

    /// Skip the next char in the input string.
    fn skip_char(&mut self) {
        let _ = self.next_char();
    }

    /// Returns the next char in the input string and consume it, or returns
    /// None if we reached the end of input.
    fn next_char(&mut self) -> Option<char> {
        let c = self.peek_char()?;
        self.input = &self.input[c.len_utf8()..];
        Some(c)
    }

    /// Attempts to parse an identifier, either for a key or for the value of an enum type.
    fn parse_identifier(&mut self) -> Result<&'de str> {
        let (remainder, res) = any_identifier(self.input)
            .finish()
            .map_err(|_| self.error_here(ErrorKind::ExpectedIdentifier))?;

        self.input = remainder;
        Ok(res)
    }

    /// Attempts to parse a string.
    fn parse_string(&mut self) -> Result<Cow<'de, str>> {
        let (remainder, res) =
            any_string(self.input)
                .finish()
                .map_err(|e: nom::error::Error<_>| {
                    self.input = e.input;
                    // Any error means we did not have a well-formed string.
                    self.error_here(ErrorKind::ExpectedString)
                })?;

        self.input = remainder;

        // The character following a string will be either a comma or EOS. If we have something
        // else, this means an unquoted string should probably have been quoted.
        match self.peek_char() {
            Some(',') | None => Ok(res),
            Some(_) => Err(self.error_here(ErrorKind::InvalidCharInString)),
        }
    }

    /// Attempt to parse a boolean.
    fn parse_bool(&mut self) -> Result<bool> {
        let (remainder, res) =
            any_bool(self.input)
                .finish()
                .map_err(|e: nom::error::Error<_>| {
                    self.input = e.input;
                    self.error_here(ErrorKind::ExpectedBoolean)
                })?;

        self.input = remainder;
        Ok(res)
    }

    /// Attempt to parse a positive or negative number.
    fn parse_number<T>(&mut self) -> Result<T>
    where
        T: Num<FromStrRadixErr = ParseIntError>,
    {
        let (remainder, val) = any_number(self.input)
            .finish()
            .map_err(|_| self.error_here(ErrorKind::InvalidNumber))?;

        self.input = remainder;
        Ok(val)
    }
}

impl<'de> de::MapAccess<'de> for KeyValueDeserializer<'de> {
    type Error = ParseError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: de::DeserializeSeed<'de>,
    {
        let has_next_identifier = self.next_identifier.is_some();

        if self.peek_char().is_none() {
            return Ok(None);
        }

        self.has_equal = false;

        let val = seed.deserialize(&mut *self).map(Some)?;

        // We just "deserialized" the content of `next_identifier`, so there should be no equal
        // character in the input. We can return now.
        if has_next_identifier {
            self.has_equal = true;
            return Ok(val);
        }

        match self.peek_char() {
            // We expect an equal after an identifier.
            Some('=') => {
                self.skip_char();
                self.has_equal = true;
                Ok(val)
            }
            // Ok if we are parsing a boolean where an empty value means true.
            Some(',') | None => Ok(val),
            Some(_) => Err(self.error_here(ErrorKind::ExpectedEqual)),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: de::DeserializeSeed<'de>,
    {
        let val = seed.deserialize(&mut *self)?;

        // We must have a comma or end of input after a value.
        match self.next_char() {
            Some(',') | None => Ok(val),
            Some(_) => Err(self.error_here(ErrorKind::ExpectedComma)),
        }
    }
}

struct Enum<'a, 'de: 'a>(&'a mut KeyValueDeserializer<'de>);

impl<'a, 'de> Enum<'a, 'de> {
    fn new(de: &'a mut KeyValueDeserializer<'de>) -> Self {
        Self(de)
    }
}

impl<'a, 'de> de::EnumAccess<'de> for Enum<'a, 'de> {
    type Error = ParseError;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: de::DeserializeSeed<'de>,
    {
        let val = seed.deserialize(&mut *self.0)?;
        Ok((val, self))
    }
}

impl<'a, 'de> de::VariantAccess<'de> for Enum<'a, 'de> {
    type Error = ParseError;

    fn unit_variant(self) -> Result<()> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, _seed: T) -> Result<T::Value>
    where
        T: de::DeserializeSeed<'de>,
    {
        unimplemented!()
    }

    fn tuple_variant<V>(self, _len: usize, _visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        unimplemented!()
    }

    fn struct_variant<V>(self, _fields: &'static [&'static str], _visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        unimplemented!()
    }
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut KeyValueDeserializer<'de> {
    type Error = ParseError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.peek_char() {
            Some('0'..='9') => self.deserialize_u64(visitor),
            Some('-') => self.deserialize_i64(visitor),
            Some('"') => self.deserialize_string(visitor),
            // Only possible option here is boolean flag.
            Some(',') | None => self.deserialize_bool(visitor),
            _ => {
                // We probably have an unquoted string, but possibly a boolean as well.
                match any_identifier(self.input) {
                    Ok((_, "true")) | Ok((_, "false")) => self.deserialize_bool(visitor),
                    _ => self.deserialize_str(visitor),
                }
            }
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        // It is valid to just mention a bool as a flag and not specify its value - in this case
        // the value is set as `true`.
        let val = if self.has_equal {
            self.parse_bool()?
        } else {
            true
        };
        visitor.visit_bool(val)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_i8(self.parse_number()?)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_i16(self.parse_number()?)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_i32(self.parse_number()?)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_i64(self.parse_number()?)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_u8(self.parse_number()?)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_u16(self.parse_number()?)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_u32(self.parse_number()?)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_u64(self.parse_number()?)
    }

    fn deserialize_f32<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_f64<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_char(
            self.next_char()
                .ok_or_else(|| self.error_here(ErrorKind::Eof))?,
        )
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.parse_string()? {
            Cow::Borrowed(s) => visitor.visit_borrowed_str(s),
            Cow::Owned(s) => visitor.visit_string(s),
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        // The fact that an option is specified implies that is exists, hence we always visit
        // Some() here.
        visitor.visit_some(self)
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_tuple<V>(self, _len: usize, _visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        _visitor: V,
    ) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_map(self)
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        // The name of the first field of a struct can be omitted (see documentation of
        // `next_identifier` for details).
        //
        // To detect this, peek the next identifier, and check if the character following is '='. If
        // it is not, then we may have a value in first position, unless the value is identical to
        // one of the field's name - in this case, assume this is a boolean using the flag syntax.
        self.next_identifier = match any_identifier(self.input) {
            Ok((_, s)) => match self.input.chars().nth(s.chars().count()) {
                Some('=') => None,
                _ => {
                    if fields.contains(&s) {
                        None
                    } else {
                        fields.get(0).copied()
                    }
                }
            },
            // Not an identifier, probably means this is a value for the first field then.
            Err(_) => fields.get(0).copied(),
        };
        visitor.visit_map(self)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_enum(Enum::new(self))
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        let identifier = self
            .next_identifier
            .take()
            .map_or_else(|| self.parse_identifier(), Ok)?;

        visitor.visit_borrowed_str(identifier)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }
}

/// Attempts to deserialize `T` from the key-values string `input`.
pub fn from_key_values<'a, T>(input: &'a str) -> Result<T>
where
    T: Deserialize<'a>,
{
    let mut deserializer = KeyValueDeserializer::from(input);
    let t = T::deserialize(&mut deserializer)?;
    if deserializer.input.is_empty() {
        Ok(t)
    } else {
        Err(deserializer.error_here(ErrorKind::TrailingCharacters))
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[derive(Deserialize, PartialEq, Debug)]
    struct SingleStruct<T> {
        m: T,
    }

    #[test]
    fn deserialize_number() {
        let res = from_key_values::<SingleStruct<usize>>("m=54").unwrap();
        assert_eq!(res.m, 54);

        let res = from_key_values::<SingleStruct<isize>>("m=-54").unwrap();
        assert_eq!(res.m, -54);

        // Parsing a signed into an unsigned?
        let res = from_key_values::<SingleStruct<u32>>("m=-54").unwrap_err();
        assert_eq!(
            res,
            ParseError {
                kind: ErrorKind::InvalidNumber,
                pos: 2
            }
        );

        // Value too big for a signed?
        let val = i32::MAX as u32 + 1;
        let res = from_key_values::<SingleStruct<i32>>(&format!("m={}", val)).unwrap_err();
        assert_eq!(
            res,
            ParseError {
                kind: ErrorKind::InvalidNumber,
                pos: 2
            }
        );

        // Not a number.
        let res = from_key_values::<SingleStruct<usize>>("m=test").unwrap_err();
        assert_eq!(
            res,
            ParseError {
                kind: ErrorKind::InvalidNumber,
                pos: 2,
            }
        );

        // Parsing hex values
        let res: SingleStruct<usize> =
            from_key_values::<SingleStruct<usize>>("m=0x1234abcd").unwrap();
        assert_eq!(res.m, 0x1234abcd);
        let res: SingleStruct<isize> =
            from_key_values::<SingleStruct<isize>>("m=-0x1234abcd").unwrap();
        assert_eq!(res.m, -0x1234abcd);

        // Hex value outside range
        let res: ParseError = from_key_values::<SingleStruct<usize>>("m=0xg").unwrap_err();
        assert_eq!(
            res,
            ParseError {
                kind: ErrorKind::InvalidNumber,
                pos: 2,
            }
        );

        // Parsing octal values
        let res: SingleStruct<usize> = from_key_values::<SingleStruct<usize>>("m=0o755").unwrap();
        assert_eq!(res.m, 0o755);
        let res: SingleStruct<isize> = from_key_values::<SingleStruct<isize>>("m=-0o755").unwrap();
        assert_eq!(res.m, -0o755);

        // Octal value outside range
        let res: ParseError = from_key_values::<SingleStruct<usize>>("m=0o8").unwrap_err();
        assert_eq!(
            res,
            ParseError {
                kind: ErrorKind::InvalidNumber,
                pos: 2,
            }
        );

        // Parsing binary values
        let res: SingleStruct<usize> = from_key_values::<SingleStruct<usize>>("m=0b1100").unwrap();
        assert_eq!(res.m, 0b1100);
        let res: SingleStruct<isize> = from_key_values::<SingleStruct<isize>>("m=-0b1100").unwrap();
        assert_eq!(res.m, -0b1100);

        // Binary value outside range
        let res: ParseError = from_key_values::<SingleStruct<usize>>("m=0b2").unwrap_err();
        assert_eq!(
            res,
            ParseError {
                kind: ErrorKind::InvalidNumber,
                pos: 2,
            }
        );
    }

    #[test]
    fn deserialize_string() {
        let kv = "m=John";
        let res = from_key_values::<SingleStruct<String>>(kv).unwrap();
        assert_eq!(res.m, "John".to_string());

        // Spaces are valid (but not recommended) in unquoted strings.
        let kv = "m=John Doe";
        let res = from_key_values::<SingleStruct<String>>(kv).unwrap();
        assert_eq!(res.m, "John Doe".to_string());

        // Empty string is not valid if unquoted
        let kv = "m=";
        let err = from_key_values::<SingleStruct<String>>(kv).unwrap_err();
        assert_eq!(
            err,
            ParseError {
                kind: ErrorKind::ExpectedString,
                pos: 2
            }
        );

        // Quoted strings.
        let kv = r#"m="John Doe""#;
        let res = from_key_values::<SingleStruct<String>>(kv).unwrap();
        assert_eq!(res.m, "John Doe".to_string());
        let kv = r#"m='John Doe'"#;
        let res = from_key_values::<SingleStruct<String>>(kv).unwrap();
        assert_eq!(res.m, "John Doe".to_string());

        // Empty quoted strings.
        let kv = r#"m="""#;
        let res = from_key_values::<SingleStruct<String>>(kv).unwrap();
        assert_eq!(res.m, "".to_string());
        let kv = r#"m=''"#;
        let res = from_key_values::<SingleStruct<String>>(kv).unwrap();
        assert_eq!(res.m, "".to_string());

        // "=", "," and "'"" in quote.
        let kv = r#"m="val = [10, 20, 'a']""#;
        let res = from_key_values::<SingleStruct<String>>(kv).unwrap();
        assert_eq!(res.m, r#"val = [10, 20, 'a']"#.to_string());

        // Quotes in unquoted strings are forbidden.
        let kv = r#"m=val="a""#;
        let err = from_key_values::<SingleStruct<String>>(kv).unwrap_err();
        assert_eq!(
            err,
            ParseError {
                kind: ErrorKind::InvalidCharInString,
                pos: 6
            }
        );
        let kv = r#"m=val='a'"#;
        let err = from_key_values::<SingleStruct<String>>(kv).unwrap_err();
        assert_eq!(
            err,
            ParseError {
                kind: ErrorKind::InvalidCharInString,
                pos: 6
            }
        );

        // Numbers and booleans are technically valid strings.
        let kv = "m=10";
        let res = from_key_values::<SingleStruct<String>>(kv).unwrap();
        assert_eq!(res.m, "10".to_string());
        let kv = "m=false";
        let res = from_key_values::<SingleStruct<String>>(kv).unwrap();
        assert_eq!(res.m, "false".to_string());

        // Escaped quote.
        let kv = r#"m="Escaped \" quote""#;
        let res = from_key_values::<SingleStruct<String>>(kv).unwrap();
        assert_eq!(res.m, r#"Escaped " quote"#.to_string());

        // Escaped slash at end of string.
        let kv = r#"m="Escaped slash\\""#;
        let res = from_key_values::<SingleStruct<String>>(kv).unwrap();
        assert_eq!(res.m, r#"Escaped slash\"#.to_string());

        // Characters within single quotes should not be escaped.
        let kv = r#"m='Escaped \" quote'"#;
        let res = from_key_values::<SingleStruct<String>>(kv).unwrap();
        assert_eq!(res.m, r#"Escaped \" quote"#.to_string());
        let kv = r#"m='Escaped slash\\'"#;
        let res = from_key_values::<SingleStruct<String>>(kv).unwrap();
        assert_eq!(res.m, r#"Escaped slash\\"#.to_string());
    }

    #[test]
    fn deserialize_bool() {
        let res = from_key_values::<SingleStruct<bool>>("m=true").unwrap();
        assert_eq!(res.m, true);

        let res = from_key_values::<SingleStruct<bool>>("m=false").unwrap();
        assert_eq!(res.m, false);

        let res = from_key_values::<SingleStruct<bool>>("m").unwrap();
        assert_eq!(res.m, true);

        let res = from_key_values::<SingleStruct<bool>>("m=10").unwrap_err();
        assert_eq!(
            res,
            ParseError {
                kind: ErrorKind::ExpectedBoolean,
                pos: 2,
            }
        );

        let res = from_key_values::<SingleStruct<bool>>("m=").unwrap_err();
        assert_eq!(
            res,
            ParseError {
                kind: ErrorKind::ExpectedBoolean,
                pos: 2,
            }
        );
    }

    #[test]
    fn deserialize_complex_struct() {
        #[derive(Deserialize, PartialEq, Debug)]
        struct TestStruct {
            num: usize,
            path: PathBuf,
            enable: bool,
        }
        let kv = "num=54,path=/dev/foomatic,enable=false";
        let res = from_key_values::<TestStruct>(kv).unwrap();
        assert_eq!(
            res,
            TestStruct {
                num: 54,
                path: "/dev/foomatic".into(),
                enable: false,
            }
        );

        let kv = "num=0x54,path=/dev/foomatic,enable=false";
        let res = from_key_values::<TestStruct>(kv).unwrap();
        assert_eq!(
            res,
            TestStruct {
                num: 0x54,
                path: "/dev/foomatic".into(),
                enable: false,
            }
        );

        let kv = "enable,path=/usr/lib/libossom.so.1,num=12";
        let res = from_key_values::<TestStruct>(kv).unwrap();
        assert_eq!(
            res,
            TestStruct {
                num: 12,
                path: "/usr/lib/libossom.so.1".into(),
                enable: true,
            }
        );
    }

    #[test]
    fn deserialize_unknown_field() {
        #[derive(Deserialize, PartialEq, Debug)]
        #[serde(deny_unknown_fields)]
        struct TestStruct {
            num: usize,
            path: PathBuf,
            enable: bool,
        }

        let kv = "enable,path=/usr/lib/libossom.so.1,num=12,foo=bar";
        assert!(from_key_values::<TestStruct>(kv).is_err());
    }

    #[test]
    fn deserialize_option() {
        #[derive(Deserialize, PartialEq, Debug)]
        struct TestStruct {
            num: u32,
            opt: Option<u32>,
        }
        let kv = "num=16,opt=12";
        let res: TestStruct = from_key_values(kv).unwrap();
        assert_eq!(
            res,
            TestStruct {
                num: 16,
                opt: Some(12),
            }
        );

        let kv = "num=16";
        let res: TestStruct = from_key_values(kv).unwrap();
        assert_eq!(res, TestStruct { num: 16, opt: None });

        let kv = "";
        assert!(from_key_values::<TestStruct>(kv).is_err());
    }

    #[test]
    fn deserialize_enum() {
        #[derive(Deserialize, PartialEq, Debug)]
        enum TestEnum {
            #[serde(rename = "first")]
            FirstVariant,
            #[serde(rename = "second")]
            SecondVariant,
        }
        let res: TestEnum = from_key_values("first").unwrap();
        assert_eq!(res, TestEnum::FirstVariant,);

        let res: TestEnum = from_key_values("second").unwrap();
        assert_eq!(res, TestEnum::SecondVariant,);

        from_key_values::<TestEnum>("third").unwrap_err();
    }

    #[test]
    fn deserialize_embedded_enum() {
        #[derive(Deserialize, PartialEq, Debug)]
        enum TestEnum {
            #[serde(rename = "first")]
            FirstVariant,
            #[serde(rename = "second")]
            SecondVariant,
        }
        #[derive(Deserialize, PartialEq, Debug)]
        struct TestStruct {
            variant: TestEnum,
            #[serde(default)]
            active: bool,
        }
        let res: TestStruct = from_key_values("variant=first").unwrap();
        assert_eq!(
            res,
            TestStruct {
                variant: TestEnum::FirstVariant,
                active: false,
            }
        );
        let res: TestStruct = from_key_values("variant=second,active=true").unwrap();
        assert_eq!(
            res,
            TestStruct {
                variant: TestEnum::SecondVariant,
                active: true,
            }
        );
        let res: TestStruct = from_key_values("active=true,variant=second").unwrap();
        assert_eq!(
            res,
            TestStruct {
                variant: TestEnum::SecondVariant,
                active: true,
            }
        );
        let res: TestStruct = from_key_values("active,variant=second").unwrap();
        assert_eq!(
            res,
            TestStruct {
                variant: TestEnum::SecondVariant,
                active: true,
            }
        );
        let res: TestStruct = from_key_values("active=false,variant=second").unwrap();
        assert_eq!(
            res,
            TestStruct {
                variant: TestEnum::SecondVariant,
                active: false,
            }
        );
    }

    #[test]
    fn deserialize_first_arg_string() {
        #[derive(Deserialize, PartialEq, Debug)]
        struct TestStruct {
            name: String,
            num: u8,
        }
        let res: TestStruct = from_key_values("name=foo,num=12").unwrap();
        assert_eq!(
            res,
            TestStruct {
                name: "foo".into(),
                num: 12,
            }
        );

        let res: TestStruct = from_key_values("foo,num=12").unwrap();
        assert_eq!(
            res,
            TestStruct {
                name: "foo".into(),
                num: 12,
            }
        );
    }

    #[test]
    fn deserialize_first_arg_int() {
        #[derive(Deserialize, PartialEq, Debug)]
        struct TestStruct {
            num: u8,
            name: String,
        }
        let res: TestStruct = from_key_values("name=foo,num=12").unwrap();
        assert_eq!(
            res,
            TestStruct {
                num: 12,
                name: "foo".into(),
            }
        );

        let res: TestStruct = from_key_values("12,name=foo").unwrap();
        assert_eq!(
            res,
            TestStruct {
                num: 12,
                name: "foo".into(),
            }
        );
    }
}
