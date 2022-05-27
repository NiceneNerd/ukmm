use crate::{prelude::Mergeable, util::SortedDeleteMap, Result, UKError};
use roead::byml::Byml;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct Tips(pub SortedDeleteMap<String, Byml>);

impl TryFrom<&Byml> for Tips {
    type Error = UKError;

    fn try_from(byml: &Byml) -> Result<Self> {
        Ok(Self(
            byml.as_array()?
                .iter()
                .map(|entry| -> Result<(String, Byml)> {
                    let hash = entry.as_hash()?;
                    Ok((
                        hash.get("MessageId")
                            .ok_or(UKError::MissingBymlKey("Tips file entry missing MessageId"))?
                            .as_string()?
                            .to_owned(),
                        entry.clone(),
                    ))
                })
                .collect::<Result<_>>()?,
        ))
    }
}

impl From<Tips> for Byml {
    fn from(val: Tips) -> Self {
        val.0.into_iter().map(|(_, v)| v).collect()
    }
}

impl Mergeable<Byml> for Tips {
    fn diff(&self, other: &Self) -> Self {
        Self(self.0.diff(&other.0))
    }

    fn merge(&self, diff: &Self) -> Self {
        Self(self.0.merge(&diff.0))
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use roead::byml::Byml;

    fn load_tips() -> Byml {
        Byml::from_binary(
            &roead::yaz0::decompress(&std::fs::read("test/Tips/TipsWorld.sbyml").unwrap()).unwrap(),
        )
        .unwrap()
    }

    fn load_mod_tips() -> Byml {
        Byml::from_binary(
            &roead::yaz0::decompress(&std::fs::read("test/Tips/TipsWorld.mod.sbyml").unwrap())
                .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn serde() {
        let byml = load_tips();
        let tips = super::Tips::try_from(&byml).unwrap();
        let data = Byml::from(tips.clone()).to_binary(roead::Endian::Big);
        let byml2 = Byml::from_binary(&data).unwrap();
        let tips2 = super::Tips::try_from(&byml2).unwrap();
        assert_eq!(tips, tips2);
    }

    #[test]
    fn diff() {
        let byml = load_tips();
        let tips = super::Tips::try_from(&byml).unwrap();
        let byml2 = load_mod_tips();
        let tips2 = super::Tips::try_from(&byml2).unwrap();
        let _diff = tips.diff(&tips2);
    }

    #[test]
    fn merge() {
        let byml = load_tips();
        let tips = super::Tips::try_from(&byml).unwrap();
        let byml2 = load_mod_tips();
        let tips2 = super::Tips::try_from(&byml2).unwrap();
        let diff = tips.diff(&tips2);
        let merged = tips.merge(&diff);
        assert_eq!(merged, tips2);
    }
}