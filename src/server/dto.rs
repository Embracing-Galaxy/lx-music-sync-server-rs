use serde::{Deserialize, Deserializer, Serialize};

#[derive(Deserialize, Serialize)]
pub(super) struct Req {
    name: String,
    path: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
}

impl Req {
    pub(super) fn new(name: &str, data: Option<Vec<serde_json::Value>>) -> (String, Self) {
        let req = Self {
            name: format!("{}__{}", name, rand::random::<u8>()),
            path: vec![name.to_string()],
            data: data.map(|v| serde_json::Value::Array(v)),
        };
        (req.name.clone(), req)
    }

    pub(super) fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Resp {
    /// the response event name, the same as the corresponding request name
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
}

impl Resp {
    pub(crate) fn get_name(&self) -> &String {
        &self.name
    }

    pub(crate) fn get_data<T: serde::de::DeserializeOwned>(self) -> Option<T> {
        self.data.map(|data| T::deserialize(data).unwrap())
    }
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct EnabledFeatures {
    #[serde(default, deserialize_with = "de_false_or_struct")]
    pub(super) list: Option<ListConfig>,
    #[serde(default, deserialize_with = "de_false_or_struct")]
    pub(super) dislike: Option<ListConfig>,
}

impl EnabledFeatures {
    pub(super) const DEFAULT: EnabledFeatures = EnabledFeatures {
        list: Some(ListConfig {
            skip_snapshot: false,
        }),
        dislike: Some(ListConfig {
            skip_snapshot: false,
        }),
    };
}

#[derive(Debug, Deserialize, PartialEq)]
pub(super) struct ListConfig {
    #[serde(rename = "skipSnapshot")]
    pub(super) skip_snapshot: bool,
}

/// custom Option<T> deserialization: false -> None，object -> Some(T)
pub fn de_false_or_struct<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct FalseOrStruct<T>(std::marker::PhantomData<T>);

    impl<'de, T: Deserialize<'de>> serde::de::Visitor<'de> for FalseOrStruct<T> {
        type Value = Option<T>;

        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("false or object")
        }

        // false -> None
        fn visit_bool<E: serde::de::Error>(self, b: bool) -> Result<Self::Value, E> {
            if !b {
                Ok(None)
            } else {
                Err(E::custom("only false is allowed"))
            }
        }

        // object -> Some(T)
        fn visit_map<A: serde::de::MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
            let t = T::deserialize(serde::de::value::MapAccessDeserializer::new(map))?;
            Ok(Some(t))
        }
    }

    deserializer.deserialize_any(FalseOrStruct(std::marker::PhantomData))
}
