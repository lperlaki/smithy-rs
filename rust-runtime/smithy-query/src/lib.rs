/*
 * Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
 * SPDX-License-Identifier: Apache-2.0.
 */

//! Abstractions for the Smithy AWS Query protocol

use smithy_types::instant::Format;
use smithy_types::{Instant, Number};
use std::borrow::Cow;
use urlencoding::encode;

pub struct QueryWriter<'a> {
    output: &'a mut String,
}

impl<'a> QueryWriter<'a> {
    pub fn new(output: &'a mut String, action: &str, version: &str) -> Self {
        output.push_str("Action=");
        output.push_str(&encode(action));
        output.push_str("\n&Version=");
        output.push_str(&encode(version));
        QueryWriter { output }
    }

    pub fn prefix(&mut self, prefix: &'a str) -> QueryValueWriter {
        QueryValueWriter::new(self.output, Cow::Borrowed(prefix))
    }

    pub fn finish(self) {
        // Calling this drops self
    }
}

pub struct QueryMapWriter<'a> {
    output: &'a mut String,
    prefix: Cow<'a, str>,
    flatten: bool,
    next_index: usize,
}

impl<'a> QueryMapWriter<'a> {
    fn new(output: &'a mut String, prefix: Cow<'a, str>, flatten: bool) -> QueryMapWriter<'a> {
        QueryMapWriter {
            prefix,
            output,
            flatten,
            next_index: 1,
        }
    }

    pub fn entry(&mut self, key: &str) -> QueryValueWriter {
        let entry = if self.flatten { "" } else { ".entry" };
        self.output.push_str(&format!(
            "\n&{}{}.{}.key={}",
            self.prefix,
            entry,
            self.next_index,
            encode(key)
        ));
        let value_name = format!("{}{}.{}.value", self.prefix, entry, self.next_index);

        self.next_index += 1;
        QueryValueWriter::new(self.output, Cow::Owned(value_name))
    }

    pub fn finish(self) {
        // Calling this drops self
    }
}

pub struct QueryListWriter<'a> {
    output: &'a mut String,
    prefix: Cow<'a, str>,
    flatten: bool,
    member_override: Option<&'a str>,
    next_index: usize,
}

impl<'a> QueryListWriter<'a> {
    fn new(
        output: &'a mut String,
        prefix: Cow<'a, str>,
        flatten: bool,
        member_override: Option<&'a str>,
    ) -> QueryListWriter<'a> {
        QueryListWriter {
            prefix,
            output,
            flatten,
            member_override,
            next_index: 1,
        }
    }

    pub fn entry(&mut self) -> QueryValueWriter {
        let value_name = if self.flatten {
            format!("{}.{}", self.prefix, self.next_index)
        } else if self.member_override.is_some() {
            format!(
                "{}.{}.{}",
                self.prefix,
                self.member_override.unwrap(),
                self.next_index
            )
        } else {
            format!("{}.member.{}", self.prefix, self.next_index)
        };

        self.next_index += 1;
        QueryValueWriter::new(self.output, Cow::Owned(value_name))
    }

    pub fn finish(self) {
        // Calling this drops self
    }
}

pub struct QueryValueWriter<'a> {
    output: &'a mut String,
    prefix: Cow<'a, str>,
}

impl<'a> QueryValueWriter<'a> {
    pub fn new(output: &'a mut String, prefix: Cow<'a, str>) -> QueryValueWriter<'a> {
        QueryValueWriter { output, prefix }
    }

    /// Starts a new prefix.
    pub fn prefix(&mut self, prefix: &'a str) -> QueryValueWriter {
        QueryValueWriter::new(
            self.output,
            Cow::Owned(format!("{}.{}", self.prefix, prefix)),
        )
    }

    /// Writes the boolean `value`.
    pub fn boolean(mut self, value: bool) {
        self.write_param_name();
        self.output.push_str(match value {
            true => "true",
            _ => "false",
        });
    }

    /// Writes a string `value`.
    pub fn string(mut self, value: &str) {
        self.write_param_name();
        self.output.push_str(&encode(value));
    }

    /// Writes a number `value`.
    pub fn number(self, value: Number) {
        match value {
            Number::PosInt(value) => {
                // itoa::Buffer is a fixed-size stack allocation, so this is cheap
                self.string(itoa::Buffer::new().format(value));
            }
            Number::NegInt(value) => {
                self.string(itoa::Buffer::new().format(value));
            }
            Number::Float(value) => {
                // If the value is NaN, Infinity, or -Infinity
                if value.is_nan() || value.is_infinite() {
                    self.string("");
                } else {
                    // ryu::Buffer is a fixed-size stack allocation, so this is cheap
                    self.string(ryu::Buffer::new().format_finite(value));
                }
            }
        }
    }

    /// Writes an Instant `value` with the given `format`.
    pub fn instant(self, instant: &Instant, format: Format) {
        self.string(&instant.fmt(format));
    }

    /// Starts a map.
    pub fn start_map(self, flat: bool) -> QueryMapWriter<'a> {
        QueryMapWriter::new(self.output, self.prefix, flat)
    }

    /// Starts a list.
    pub fn start_list(self, flat: bool, member_override: Option<&'a str>) -> QueryListWriter<'a> {
        QueryListWriter::new(self.output, self.prefix, flat, member_override)
    }

    fn write_param_name(&mut self) {
        self.output.push_str("\n&");
        self.output.push_str(&self.prefix);
        self.output.push('=');
    }
}

#[cfg(test)]
mod tests {
    use crate::QueryWriter;
    use smithy_types::instant::Format;
    use smithy_types::{Instant, Number};

    #[test]
    fn no_params() {
        let mut out = String::new();
        let writer = QueryWriter::new(&mut out, "SomeAction", "1.0");
        writer.finish();
        assert_eq!("Action=SomeAction\n&Version=1.0", out);
    }

    #[test]
    fn maps() {
        let mut out = String::new();
        let mut writer = QueryWriter::new(&mut out, "SomeAction", "1.0");

        let mut map = writer.prefix("MapArg").start_map(false);
        map.entry("bar").string("Bar");
        map.entry("foo").string("Foo");
        map.finish();

        let mut map = writer.prefix("Some.Flattened").start_map(true);
        map.entry("bar").string("Bar");
        map.entry("foo").string("Foo");
        map.finish();

        writer.finish();

        assert_eq!(
            "Action=SomeAction\
            \n&Version=1.0\
            \n&MapArg.entry.1.key=bar\
            \n&MapArg.entry.1.value=Bar\
            \n&MapArg.entry.2.key=foo\
            \n&MapArg.entry.2.value=Foo\
            \n&Some.Flattened.1.key=bar\
            \n&Some.Flattened.1.value=Bar\
            \n&Some.Flattened.2.key=foo\
            \n&Some.Flattened.2.value=Foo\
            ",
            out
        );
    }

    #[test]
    fn lists() {
        let mut out = String::new();
        let mut writer = QueryWriter::new(&mut out, "SomeAction", "1.0");

        let mut list = writer.prefix("ListArg").start_list(false, None);
        list.entry().string("foo");
        list.entry().string("bar");
        list.entry().string("baz");
        list.finish();

        let mut list = writer.prefix("FlattenedListArg").start_list(true, None);
        list.entry().string("A");
        list.entry().string("B");
        list.finish();

        let mut list = writer.prefix("ItemList").start_list(false, Some("item"));
        list.entry().string("foo");
        list.entry().string("bar");
        list.finish();

        writer.finish();

        assert_eq!(
            "Action=SomeAction\
            \n&Version=1.0\
            \n&ListArg.member.1=foo\
            \n&ListArg.member.2=bar\
            \n&ListArg.member.3=baz\
            \n&FlattenedListArg.1=A\
            \n&FlattenedListArg.2=B\
            \n&ItemList.item.1=foo\
            \n&ItemList.item.2=bar\
            ",
            out
        );
    }

    #[test]
    fn prefixes() {
        let mut out = String::new();
        let mut writer = QueryWriter::new(&mut out, "SomeAction", "1.0");

        let mut first = writer.prefix("first");
        let second = first.prefix("second");
        second.string("second_val");
        first.string("first_val");

        writer.finish();

        assert_eq!(
            "Action=SomeAction\
            \n&Version=1.0\
            \n&first.second=second_val\
            \n&first=first_val\
            ",
            out
        );
    }

    #[test]
    fn timestamps() {
        let mut out = String::new();
        let mut writer = QueryWriter::new(&mut out, "SomeAction", "1.0");

        writer
            .prefix("epoch_seconds")
            .instant(&Instant::from_f64(5.2), Format::EpochSeconds);
        writer.prefix("date_time").instant(
            &Instant::from_str("2021-05-24T15:34:50.123Z", Format::DateTime).unwrap(),
            Format::DateTime,
        );
        writer.prefix("http_date").instant(
            &Instant::from_str("Wed, 21 Oct 2015 07:28:00 GMT", Format::HttpDate).unwrap(),
            Format::HttpDate,
        );
        writer.finish();

        assert_eq!(
            "Action=SomeAction\
            \n&Version=1.0\
            \n&epoch_seconds=5.2\
            \n&date_time=2021-05-24T15%3A34%3A50.123Z\
            \n&http_date=Wed%2C%2021%20Oct%202015%2007%3A28%3A00%20GMT\
            ",
            out
        );
    }

    #[test]
    fn numbers() {
        let mut out = String::new();
        let mut writer = QueryWriter::new(&mut out, "SomeAction", "1.0");

        writer.prefix("PosInt").number(Number::PosInt(5));
        writer.prefix("NegInt").number(Number::NegInt(-5));
        writer
            .prefix("Infinity")
            .number(Number::Float(f64::INFINITY));
        writer
            .prefix("NegInfinity")
            .number(Number::Float(f64::NEG_INFINITY));
        writer.prefix("NaN").number(Number::Float(f64::NAN));
        writer.prefix("Floating").number(Number::Float(5.2));
        writer.finish();

        assert_eq!(
            "Action=SomeAction\
            \n&Version=1.0\
            \n&PosInt=5\
            \n&NegInt=-5\
            \n&Infinity=\
            \n&NegInfinity=\
            \n&NaN=\
            \n&Floating=5.2\
            ",
            out
        );
    }

    #[test]
    fn booleans() {
        let mut out = String::new();
        let mut writer = QueryWriter::new(&mut out, "SomeAction", "1.0");

        writer.prefix("IsTrue").boolean(true);
        writer.prefix("IsFalse").boolean(false);
        writer.finish();

        assert_eq!(
            "Action=SomeAction\
            \n&Version=1.0\
            \n&IsTrue=true\
            \n&IsFalse=false\
            ",
            out
        );
    }

    #[test]
    fn action_version_escaping() {
        let mut out = String::new();
        QueryWriter::new(&mut out, "Some Action", "1 2").finish();
        assert_eq!("Action=Some%20Action\n&Version=1%202", out);
    }
}