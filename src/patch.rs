use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A PATCH property: omit it, clear it with JSON null, or set a value.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Patch<T> {
    #[default]
    Unset,
    Null,
    Value(T),
}

impl<T> Patch<T> {
    #[must_use]
    pub const fn is_unset(&self) -> bool {
        matches!(self, Self::Unset)
    }
}

impl<T: Serialize> Serialize for Patch<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Unset | Self::Null => serializer.serialize_none(),
            Self::Value(value) => value.serialize(serializer),
        }
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Patch<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Option::<T>::deserialize(deserializer)?.map_or(Self::Null, Self::Value))
    }
}

#[cfg(test)]
mod tests {
    use super::Patch;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Document {
        #[serde(default, skip_serializing_if = "Patch::is_unset")]
        enabled: Patch<bool>,
        #[serde(default, skip_serializing_if = "Patch::is_unset")]
        title: Patch<String>,
    }

    #[test]
    fn omitted_null_false_and_empty_are_distinct() {
        for (payload, expected) in [
            (
                r#"{}"#,
                Document {
                    enabled: Patch::Unset,
                    title: Patch::Unset,
                },
            ),
            (
                r#"{"enabled":false,"title":null}"#,
                Document {
                    enabled: Patch::Value(false),
                    title: Patch::Null,
                },
            ),
            (
                r#"{"title":""}"#,
                Document {
                    enabled: Patch::Unset,
                    title: Patch::Value(String::new()),
                },
            ),
        ] {
            let value: Document = serde_json::from_str(payload).unwrap();
            assert_eq!(value, expected);
            assert_eq!(
                serde_json::to_value(value).unwrap(),
                serde_json::from_str::<serde_json::Value>(payload).unwrap()
            );
        }
    }
}
