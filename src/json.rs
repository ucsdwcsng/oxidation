use crate::SigmfError;
use serde::de::{DeserializeSeed, Error as _, MapAccess, SeqAccess, Visitor};
use serde::{Deserializer, de};
use serde_json::{Map, Number, Value};
use std::collections::HashSet;
use std::fmt;

struct UniqueValueSeed {
    pointer: String,
}

impl<'de> DeserializeSeed<'de> for UniqueValueSeed {
    type Value = Value;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(UniqueValueVisitor {
            pointer: self.pointer,
        })
    }
}

struct UniqueValueVisitor {
    pointer: String,
}

impl<'de> Visitor<'de> for UniqueValueVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("any valid JSON value")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(Value::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(Value::Number(value.into()))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(Value::Number(value.into()))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| E::custom("JSON number must be finite"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
        Ok(Value::String(value.to_owned()))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(Value::String(value))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(Value::Null)
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(Value::Null)
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        while let Some(value) = sequence.next_element_seed(UniqueValueSeed {
            pointer: format!("{}/{}", self.pointer, values.len()),
        })? {
            values.push(value);
        }
        Ok(Value::Array(values))
    }

    fn visit_map<A>(self, mut object: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut values = Map::new();
        let mut seen = HashSet::new();
        while let Some(key) = object.next_key::<String>()? {
            let escaped = key.replace('~', "~0").replace('/', "~1");
            let child_pointer = format!("{}/{}", self.pointer, escaped);
            if !seen.insert(key.clone()) {
                return Err(A::Error::custom(format!(
                    "duplicate JSON member at {}",
                    child_pointer
                )));
            }
            let value = object.next_value_seed(UniqueValueSeed {
                pointer: child_pointer,
            })?;
            values.insert(key, value);
        }
        Ok(Value::Object(values))
    }
}

pub(crate) fn parse_unique_value(bytes: &[u8]) -> Result<Value, SigmfError> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let value = UniqueValueSeed {
        pointer: String::new(),
    }
    .deserialize(&mut deserializer)
    .map_err(|error| SigmfError::InvalidJson(error.to_string()))?;
    deserializer
        .end()
        .map_err(|error| SigmfError::InvalidJson(error.to_string()))?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_member_errors_include_escaped_json_pointer() {
        let error = parse_unique_value(br#"{"global":{"core:author":"a","core:author":"b"}}"#)
            .unwrap_err()
            .to_string();
        assert!(error.contains("/global/core:author"), "{error}");

        let error = parse_unique_value(br#"{"a/b":{"~key":1,"~key":2}}"#)
            .unwrap_err()
            .to_string();
        assert!(error.contains("/a~1b/~0key"), "{error}");
    }
}
