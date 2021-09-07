use nom::{
    branch::alt,
    bytes::complete::{tag, take_while},
    character::complete::{alphanumeric1, char as char_},
    combinator::{map, value},
    error::{ErrorKind, ParseError},
    multi::separated_list0,
    number::complete::double,
    sequence::{delimited, preceded, separated_pair, terminated},
    IResult,
};
use std::collections::HashMap;

#[derive(Debug)]
enum JsonValue {
    Null,

    /// JavaScript primitive types is bool,f64,String
    Boolean(bool),
    NumberF64(f64),
    /// JSON only allow double quote String expression, JavaScript can use single quote String expression
    String(String),

    Array(Vec<JsonValue>),
    /// JavaScript Object
    Map(HashMap<String, JsonValue>),
}

impl JsonValue {
    fn from_str(s: &str) -> Result<Self, String> {
        if !s.chars().all(|c| c.is_ascii()) {
            return Err(
                "only support ASCII alphanumeric, does not support string contains Unicode"
                    .to_string(),
            );
        }
        match parse_json_str::<(&str, ErrorKind)>(s) {
            Ok(val) => Ok(val.1),
            Err(e) => Err(e.to_string()),
        }
    }
}

/// split whitespace or tab or newline
fn split<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, &'a str, E> {
    take_while(|c| " \t\r\n".contains(c))(i)
}

/// match a pair of double quote
fn parse_string<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, &'a str, E> {
    preceded(char_('\"'), terminated(alphanumeric1, char_('\"')))(i)
}

fn parse_json_map<'a, E: ParseError<&'a str>>(
    i: &'a str,
) -> IResult<&'a str, HashMap<String, JsonValue>, E> {
    preceded(
        char_('{'),
        terminated(
            map(
                separated_list0(
                    preceded(split, char_(',')),
                    separated_pair(
                        preceded(split, parse_string),
                        preceded(split, char_(':')),
                        parse_json_value,
                    ),
                ),
                |tuple_vec| {
                    tuple_vec
                        .into_iter()
                        .map(|(k, v)| (String::from(k), v))
                        .collect()
                },
            ),
            preceded(split, char_('}')),
        ),
    )(i)
}

fn parse_json_array<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, Vec<JsonValue>, E> {
    preceded(
        char_('['),
        terminated(
            separated_list0(preceded(split, char_(',')), parse_json_value),
            preceded(split, char_(']')),
        ),
    )(i)
}

/// The root node of json tree must be one of Null/Array/Map
fn parse_json_root<'a, E: ParseError<&'a str>>(
    _i: &str,
) -> impl FnMut(&'a str) -> IResult<&'a str, JsonValue, E> {
    alt((
        map(parse_json_map, JsonValue::Map),
        map(parse_json_array, JsonValue::Array),
        map(|i| value((), tag("null"))(i), |_| JsonValue::Null),
    ))
}

/// here, we apply the space parser before trying to parse a value
fn parse_json_value<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, JsonValue, E> {
    preceded(
        split,
        alt((
            map(parse_json_root(i), |val| val),
            map(
                alt((value(true, tag("true")), value(false, tag("false")))),
                JsonValue::Boolean,
            ),
            map(double, JsonValue::NumberF64),
            map(parse_string, |s| JsonValue::String(s.to_string())),
        )),
    )(i)
}

/// 因为 nom 类似 warp 函数签名一堆 impl trait 所以被迫函数式编程写法
fn parse_json_str<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, JsonValue, E> {
    delimited(split, map(parse_json_root(i), |val| val), split)(i)
}

#[test]
fn test_json_value() {
    assert!(JsonValue::from_str(r#"{"key": "value"}"#).is_ok());
    assert!(JsonValue::from_str(r#"[1, "two"]"#).is_ok());
    assert!(JsonValue::from_str("null").is_ok());
    assert!(JsonValue::from_str(r#"{"key": null}"#).is_ok());
    assert!(JsonValue::from_str("{\"key\": \"???\"}").is_err());
    assert!(JsonValue::from_str("{\"key\": \"中文\"}").is_err());
}
