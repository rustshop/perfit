use std::fmt;

use serde::{de, Deserializer};

pub fn deserialize_opt_f64_from_empty_string<'de, D>(
    deserializer: D,
) -> Result<Option<f64>, D::Error>
where
    D: Deserializer<'de>,
{
    struct OptF64Visitor;

    impl<'de> de::Visitor<'de> for OptF64Visitor {
        type Value = Option<f64>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("null, an empty string, or a float")
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Option<f64>, D::Error>
        where
            D: Deserializer<'de>,
        {
            deserializer.deserialize_str(self)
        }

        // For directly handling nulls, leading to None
        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }
        // Handle missing value as None
        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        // Handle missing value as None
        fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(value))
        }

        // Handle empty string as None
        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            if value.is_empty() {
                Ok(None)
            } else {
                match value.parse::<f64>() {
                    Ok(v) => Ok(Some(v)),
                    Err(_) => Err(E::custom("Expected a float")),
                }
            }
        }
    }

    deserializer.deserialize_option(OptF64Visitor)
}

pub mod custom_rfc3339_option {
    use serde::Serializer;
    use time::OffsetDateTime;

    #[allow(clippy::wildcard_imports)]
    use super::*;

    pub fn serialize<S: Serializer>(
        option: &Option<OffsetDateTime>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        time::serde::rfc3339::option::serialize(option, serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<time::OffsetDateTime>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = Option<OffsetDateTime>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("null, an empty string, or a float")
            }

            fn visit_some<D>(self, deserializer: D) -> Result<Option<OffsetDateTime>, D::Error>
            where
                D: Deserializer<'de>,
            {
                deserializer.deserialize_str(self)
            }

            // For directly handling nulls, leading to None
            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(None)
            }
            // Handle missing value as None
            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(None)
            }

            // Handle empty string as None
            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if value.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(
                        OffsetDateTime::parse(
                            value,
                            &time::format_description::well_known::Rfc3339,
                        )
                        .map_err(|e| E::custom(e.to_string()))?,
                    ))
                }
            }
        }

        deserializer.deserialize_option(Visitor)
    }
}
