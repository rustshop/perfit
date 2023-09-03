use std::fmt;
use std::str::FromStr;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use bincode::{Decode, Encode};
use color_eyre::eyre::format_err;
use rand::RngCore as _;
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Encode, Decode, Debug, Clone, Copy)]
pub struct AccessToken([u8; 32]);

impl AccessToken {
    pub fn generate() -> Self {
        let mut bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        Self(bytes)
    }
}
impl FromStr for AccessToken {
    type Err = color_eyre::eyre::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = URL_SAFE_NO_PAD.decode(s)?;
        if bytes.len() == 32 {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&bytes[..]);
            Ok(AccessToken(arr))
        } else {
            Err(format_err!("length must be 32 bytes"))
        }
    }
}

impl Serialize for AccessToken {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let base64_string = URL_SAFE_NO_PAD.encode(self.0);
        serializer.serialize_str(&base64_string)
    }
}

impl<'de> Deserialize<'de> for AccessToken {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct AccessTokenVisitor;

        impl<'de> Visitor<'de> for AccessTokenVisitor {
            type Value = AccessToken;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a base64 URL safe encoded string")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                URL_SAFE_NO_PAD
                    .decode(value)
                    .map_err(serde::de::Error::custom)
                    .and_then(|vec| {
                        if vec.len() == 32 {
                            let mut arr = [0u8; 32];
                            arr.copy_from_slice(&vec[..]);
                            Ok(AccessToken(arr))
                        } else {
                            Err(serde::de::Error::custom("length must be 32 bytes"))
                        }
                    })
            }
        }

        deserializer.deserialize_str(AccessTokenVisitor)
    }
}
