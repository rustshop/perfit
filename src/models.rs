use std::fmt;

use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod access_token;
pub mod ts;

macro_rules! define_uuidv4_newtype {
    ($name:ident) => {
        #[derive(Debug, Copy, Clone, PartialEq, Eq)]
        pub struct $name(Uuid);

        impl $name {
            pub fn generate() -> Self {
                Self(Uuid::new_v4())
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                use base64::engine::general_purpose::URL_SAFE_NO_PAD;
                use base64::prelude::*;
                let base64_uuid = URL_SAFE_NO_PAD.encode(&self.0.as_bytes());
                f.write_str(&base64_uuid)
            }
        }

        impl bincode::Encode for $name {
            fn encode<E: bincode::enc::Encoder>(
                &self,
                encoder: &mut E,
            ) -> std::prelude::v1::Result<(), bincode::error::EncodeError> {
                Encode::encode(&self.0.into_bytes(), encoder)
            }
        }
        impl bincode::Decode for $name {
            fn decode<D: bincode::de::Decoder>(
                decoder: &mut D,
            ) -> std::prelude::v1::Result<Self, bincode::error::DecodeError> {
                Ok(Self(Uuid::from_bytes(Decode::decode(decoder)?)))
            }
        }

        impl<'de> bincode::BorrowDecode<'de> for $name {
            fn borrow_decode<D: bincode::de::BorrowDecoder<'de>>(
                decoder: &mut D,
            ) -> std::prelude::v1::Result<Self, bincode::error::DecodeError> {
                Ok(Self(Uuid::from_bytes(Decode::decode(decoder)?)))
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                serializer.serialize_str(&self.to_string())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::de::Deserializer<'de>,
            {
                struct Visitor;

                impl<'de> serde::de::Visitor<'de> for Visitor {
                    type Value = $name;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("a base64 encoded UUID string")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<$name, E>
                    where
                        E: serde::de::Error,
                    {
                        use base64::engine::general_purpose::URL_SAFE_NO_PAD;
                        use base64::prelude::*;
                        let bytes = URL_SAFE_NO_PAD
                            .decode(value)
                            .map_err(serde::de::Error::custom)?;
                        Uuid::from_slice(&bytes)
                            .map($name)
                            .map_err(serde::de::Error::custom)
                    }
                }

                deserializer.deserialize_str(Visitor)
            }
        }
    };
}

define_uuidv4_newtype!(SeriesId);

#[derive(Debug, Encode, Decode, Default, Clone, Copy)]
pub struct SeriesInternalId(u64);

impl SeriesInternalId {
    pub fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

define_uuidv4_newtype!(AccountId);
