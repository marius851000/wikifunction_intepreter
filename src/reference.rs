use std::{fmt::{Debug, Display}, num::NonZeroU64};

use anyhow::{Context, bail};
use serde::{Deserialize, de::Visitor};

#[derive(Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
/// At least one of the value is Some
pub struct Reference(Option<NonZeroU64>, Option<NonZeroU64>);

impl Reference {
    pub fn from_zid(text: &str) -> anyhow::Result<Self> {
        let mut k_splitted = text.split('K');

        let before_key = k_splitted
            .next()
            .context("input text should not be empty")?;

        let z = if !before_key.is_empty() {
            let mut char_id_iter = before_key.chars();
            if char_id_iter
                .next()
                .context("text before K/end of string should not be empty")?
                != 'Z'
            {
                bail!("First character should be Z");
            }
            Some(
                u64::from_str_radix(char_id_iter.as_str(), 10)
                    .context("Can’t convert the first number part of the ZID to a u64 number")?,
            )
        } else {
            None
        };

        let k = if let Some(second_part) = k_splitted.next() {
            Some(
                u64::from_str_radix(second_part, 10)
                    .context("Could not parse post-key text as u64")?,
            )
        } else {
            None
        };

        if k_splitted.next().is_some() {
            bail!("Text contain extra characters")
        }

        Ok(Reference::from_u64s(z, k)?)
    }

    pub fn from_u64s(z: Option<u64>, k: Option<u64>) -> anyhow::Result<Self> {
        if z.is_none() && k.is_none() {
            bail!("z and k should not be both None");
        }
        Ok(Self(
            if let Some(z) = z {
                Some(NonZeroU64::try_from(z).context("z should be non-zero")?)
            } else {
                None
            },
            if let Some(k) = k {
                Some(NonZeroU64::try_from(k).context("k should be non-zero")?)
            } else {
                None
            },
        ))
    }

    pub const fn from_u64s_panic(z: Option<u64>, k: Option<u64>) -> Self {
        if z.is_none() && k.is_none() {
            panic!("z and k should not be both None");
        }
        Self(
            if let Some(z) = z {
                Some(NonZeroU64::new(z).unwrap())
            } else {
                None
            },
            if let Some(k) = k {
                Some(NonZeroU64::new(k).unwrap())
            } else {
                None
            },
        )
    }

    pub fn to_zid(&self) -> String {
        if let Some(z) = self.0 {
            if let Some(k) = self.1 {
                format!("Z{}K{}", z, k)
            } else {
                format!("Z{}", z)
            }
        } else {
            if let Some(k) = self.1 {
                format!("K{}", k)
            } else {
                unreachable!("z and k should be both null");
            }
        }
    }
}

impl Display for Reference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_zid())
    }
}

impl Debug for Reference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // That’s probably good. This reference has quite a specific and recogniseable syntax
        f.write_str(&self.to_zid())
    }
}

#[derive(Default)]
pub(crate) struct ReferenceVisitor {}

impl<'de> Visitor<'de> for ReferenceVisitor {
    type Value = Reference;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a ZID")
    }

    fn visit_borrowed_str<E>(self, t: &'de str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        match Reference::from_zid(t) {
            Ok(v) => Ok(v),

            Err(err) => Err(serde::de::Error::invalid_value(
                serde::de::Unexpected::Str(t),
                &err.to_string().as_str(),
            )),
        }
    }
}

impl<'de> Deserialize<'de> for Reference {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(ReferenceVisitor::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_zid() {
        assert_eq!(
            Reference::from_zid("Z156").unwrap(),
            Reference::from_u64s(Some(156), None).unwrap()
        );
        assert_eq!(
            Reference::from_zid("Z30K4").unwrap(),
            Reference::from_u64s(Some(30), Some(4)).unwrap()
        );
        assert_eq!(
            Reference::from_zid("K1").unwrap(),
            Reference(None, Some(NonZeroU64::new(1)).unwrap())
        );
        assert!(Reference::from_zid("T156").is_err());
        assert!(Reference::from_zid("Z").is_err());
        assert!(Reference::from_zid("Z-9").is_err());
        assert!(Reference::from_zid("Z1a").is_err());
        assert!(Reference::from_zid("Za1").is_err());
        assert!(Reference::from_zid("").is_err());
        assert!(Reference::from_zid("Z30K4Z1").is_err());
        assert!(Reference::from_zid("Z30K4K1").is_err());
    }

    #[test]
    fn test_to_zid() {
        assert_eq!(
            Reference::from_u64s(Some(156), None).unwrap().to_zid(),
            "Z156"
        );
        assert_eq!(
            Reference::from_u64s(Some(30), Some(4)).unwrap().to_zid(),
            "Z30K4"
        );
    }

    #[test]
    fn test_deserialize() {
        assert_eq!(
            serde_json::from_str::<Reference>("\"Z654\"").unwrap(),
            Reference::from_u64s(Some(654), None).unwrap()
        );
        assert_eq!(
            serde_json::from_str::<Reference>("\"Z30K5\"").unwrap(),
            Reference::from_u64s(Some(30), Some(5)).unwrap(),
        );
        assert!(serde_json::from_str::<Reference>("654").is_err());
        assert!(serde_json::from_str::<Reference>("Z1a").is_err());
    }
}
