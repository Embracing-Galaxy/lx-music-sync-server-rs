use serde::{Deserialize, Deserializer, Serialize};
use std::any::type_name;

#[derive(Deserialize)]
#[serde(untagged)]
pub(super) enum IncomingMsg {
    Req(Req),
    Resp(Resp),
}

#[derive(Deserialize, Serialize)]
pub(super) struct Req {
    pub(super) name: String,
    path: Vec<String>,
    pub(super) data: Vec<serde_json::Value>,
}

impl Req {
    pub(super) fn new(name: &str, data: Vec<serde_json::Value>) -> (String, Self) {
        let req = Self {
            name: format!("{}__{}", name, rand::random::<u8>()),
            path: vec![name.to_string()],
            data,
        };
        (req.name.clone(), req)
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub(in crate::server) struct Resp {
    /// the response event name, the same as the corresponding request name
    pub(super) name: String,
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
}

impl Resp {
    pub(in crate::server) fn gen_empty(name: String) -> Self {
        Self {
            name,
            error: None,
            data: None,
        }
    }

    pub(in crate::server) fn get_data<T: serde::de::DeserializeOwned>(self) -> Option<T> {
        if let Some(err) = self.error {
            panic!("client response with err: {err}");
        }
        self.data.map(|data| {
            T::deserialize(data).expect(&format!(
                "Resp of type {} deserialization error",
                type_name::<T>()
            ))
        })
    }
}

#[derive(Debug, Deserialize, PartialEq)]
pub(in crate::server) struct EnabledFeatures {
    #[serde(default, deserialize_with = "de_false_or_struct")]
    list: Option<ListConfig>,
    #[serde(default, deserialize_with = "de_false_or_struct")]
    dislike: Option<ListConfig>,
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
struct ListConfig {
    #[serde(rename = "skipSnapshot")]
    skip_snapshot: bool,
}

/// custom Option<T> deserialization: false -> None，object -> Some(T)
fn de_false_or_struct<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
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
