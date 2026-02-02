//! Lua value type for parsed blueprints.

use crate::parser::LuaKey;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub enum LuaValue {
    Table(BTreeMap<LuaKey, LuaValue>),
    String(String),
    Number(f64),
    Bool(bool),
}

impl LuaValue {
    pub fn as_table(&self) -> Option<&BTreeMap<LuaKey, LuaValue>> {
        match self {
            LuaValue::Table(t) => Some(t),
            _ => None,
        }
    }

    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.as_table()?
            .get(&LuaKey::String(key.to_string()))
            .and_then(LuaValue::as_str)
    }

    pub fn get_num(&self, key: &str) -> Option<f64> {
        self.as_table()?
            .get(&LuaKey::String(key.to_string()))
            .and_then(LuaValue::as_number)
    }

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.as_table()?
            .get(&LuaKey::String(key.to_string()))
            .and_then(LuaValue::as_bool)
    }

    pub fn get_table(&self, key: &str) -> Option<&LuaValue> {
        self.as_table()?
            .get(&LuaKey::String(key.to_string()))
            .filter(|v| v.as_table().is_some())
    }

    pub fn get_by_index(&self, index: u32) -> Option<&LuaValue> {
        self.as_table()?
            .get(&LuaKey::Number(index))
            .or_else(|| self.as_table()?.get(&LuaKey::Number(1)))
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            LuaValue::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<f64> {
        match self {
            LuaValue::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            LuaValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn table_len(&self) -> Option<usize> {
        let t = self.as_table()?;
        let mut max = 0u32;
        for k in t.keys() {
            if let LuaKey::Number(n) = k {
                if *n > max {
                    max = *n;
                }
            }
        }
        Some(max as usize)
    }
}

impl Serialize for LuaValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            LuaValue::Table(t) => {
                use serde::ser::SerializeMap;
                let mut map = serializer.serialize_map(Some(t.len()))?;
                for (k, v) in t {
                    let key_str = match k {
                        LuaKey::String(s) => s.clone(),
                        LuaKey::Number(n) => n.to_string(),
                    };
                    map.serialize_entry(&key_str, v)?;
                }
                map.end()
            }
            LuaValue::String(s) => serializer.serialize_str(s),
            LuaValue::Number(n) => serializer.serialize_f64(*n),
            LuaValue::Bool(b) => serializer.serialize_bool(*b),
        }
    }
}

impl Serialize for LuaKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            LuaKey::String(s) => serializer.serialize_str(s),
            LuaKey::Number(n) => serializer.serialize_u32(*n),
        }
    }
}
