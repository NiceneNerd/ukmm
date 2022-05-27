use crate::{prelude::Mergeable, util::SortedDeleteSet, Result, UKError};
use roead::{aamp::hash_name, byml::Byml};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct SaveDataHeader {
    pub is_common: bool,
    pub is_common_at_same_account: bool,
    pub is_save_secure_code: bool,
    pub file_name: String,
}

impl TryFrom<&Byml> for SaveDataHeader {
    type Error = UKError;

    fn try_from(val: &Byml) -> Result<Self> {
        let hash = val.as_hash()?;
        Ok(Self {
            is_common: hash
                .get("IsCommon")
                .ok_or(UKError::MissingBymlKey("bgsvdata header missing IsCommon"))?
                .as_bool()?,
            is_common_at_same_account: hash
                .get("IsCommonAtSameAccount")
                .ok_or(UKError::MissingBymlKey(
                    "bgsvdata header missing IsCommonAtSameAccount",
                ))?
                .as_bool()?,
            is_save_secure_code: hash
                .get("IsSaveSecureCode")
                .ok_or(UKError::MissingBymlKey(
                    "bgsvdata header missing IsSaveSecureCode",
                ))?
                .as_bool()?,
            file_name: hash
                .get("file_name")
                .ok_or(UKError::MissingBymlKey("bgsvdata header missing file_name"))?
                .as_string()?
                .to_owned(),
        })
    }
}

impl From<SaveDataHeader> for Byml {
    fn from(val: SaveDataHeader) -> Self {
        [
            ("IsCommon", Byml::Bool(val.is_common)),
            (
                "IsCommonAtSameAccount",
                Byml::Bool(val.is_common_at_same_account),
            ),
            ("IsSaveSecureCode", Byml::Bool(val.is_save_secure_code)),
            ("file_name", Byml::String(val.file_name)),
        ]
        .into_iter()
        .collect()
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Flag(String, i32);

impl From<String> for Flag {
    fn from(string: String) -> Self {
        let hash = hash_name(&string) as i32;
        Self(string, hash)
    }
}

impl From<&str> for Flag {
    fn from(string: &str) -> Self {
        let hash = hash_name(string) as i32;
        Self(string.to_owned(), hash)
    }
}

impl PartialEq for Flag {
    fn eq(&self, other: &Self) -> bool {
        self.1 == other.1
    }
}

impl Eq for Flag {}

impl std::hash::Hash for Flag {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_i32(self.1)
    }
}

impl PartialOrd for Flag {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.1.partial_cmp(&other.1)
    }
}

impl Ord for Flag {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.1.cmp(&other.1)
    }
}

impl TryFrom<&Byml> for Flag {
    type Error = UKError;

    fn try_from(byml: &Byml) -> Result<Self> {
        Ok(Self(
            byml.as_hash()?
                .get("DataName")
                .ok_or(UKError::MissingBymlKey("bgsvdata missing DataName"))?
                .as_string()?
                .to_owned(),
            byml.as_hash()?
                .get("HashValue")
                .ok_or(UKError::MissingBymlKey("bgsvdata missing HashValue"))?
                .as_int()?
                .to_owned(),
        ))
    }
}

impl From<Flag> for Byml {
    fn from(val: Flag) -> Self {
        [
            ("DataName", Byml::String(val.0)),
            ("HashValue", Byml::Int(val.1)),
        ]
        .into_iter()
        .collect()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct SaveData {
    pub header: SaveDataHeader,
    pub flags: SortedDeleteSet<Flag>,
}

impl TryFrom<&Byml> for SaveData {
    type Error = UKError;

    fn try_from(val: &Byml) -> Result<Self> {
        let array = val
            .as_hash()?
            .get("file_list")
            .ok_or(UKError::MissingBymlKey("bgsvdata missing file_list"))?
            .as_array()?;
        Ok(Self {
            header: array
                .get(0)
                .ok_or(UKError::MissingBymlKey("bgsvdata missing header"))?
                .try_into()?,
            flags: array
                .get(1)
                .ok_or(UKError::MissingBymlKey("bgsvdata missing flag array"))?
                .as_array()?
                .iter()
                .map(Flag::try_from)
                .collect::<Result<SortedDeleteSet<_>>>()?,
        })
    }
}

impl From<SaveData> for Byml {
    fn from(val: SaveData) -> Self {
        [
            (
                "file_list",
                [
                    val.header.into(),
                    val.flags.into_iter().map(Byml::from).collect::<Byml>(),
                ]
                .into_iter()
                .collect::<Byml>(),
            ),
            (
                "save_info",
                Byml::Array(vec![[
                    ("directory_num", Byml::Int(8)),
                    ("is_build_machine", Byml::Bool(true)),
                    ("revision", Byml::Int(18203)),
                ]
                .into_iter()
                .collect::<Byml>()]),
            ),
        ]
        .into_iter()
        .collect()
    }
}

impl Mergeable<Byml> for SaveData {
    fn diff(&self, other: &Self) -> Self {
        assert_eq!(
            self.header, other.header,
            "Attempted to diff incompatible savedata files: {:?} and {:?}",
            self.header, other.header
        );
        Self {
            header: self.header.clone(),
            flags: self.flags.diff(&other.flags),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        assert_eq!(
            self.header, diff.header,
            "Attempted to merge incompatible savedata files: {:?} and {:?}",
            self.header, diff.header
        );
        Self {
            header: self.header.clone(),
            flags: self.flags.merge(&diff.flags),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use roead::byml::Byml;

    fn load_savedata() -> Byml {
        Byml::from_binary(&std::fs::read("test/GameData/saveformat_0.bgsvdata").unwrap()).unwrap()
    }

    fn load_mod_savedata() -> Byml {
        Byml::from_binary(&std::fs::read("test/GameData/saveformat_0.mod.bgsvdata").unwrap())
            .unwrap()
    }

    #[test]
    fn serde() {
        let byml = load_savedata();
        let savedata = super::SaveData::try_from(&byml).unwrap();
        let data = Byml::from(savedata.clone()).to_binary(roead::Endian::Big);
        let byml2 = Byml::from_binary(&data).unwrap();
        let savedata2 = super::SaveData::try_from(&byml2).unwrap();
        assert_eq!(savedata, savedata2);
    }

    #[test]
    fn diff() {
        let byml = load_savedata();
        let savedata = super::SaveData::try_from(&byml).unwrap();
        let byml2 = load_mod_savedata();
        let savedata2 = super::SaveData::try_from(&byml2).unwrap();
        let diff = savedata.diff(&savedata2);
        dbg!(diff);
    }

    #[test]
    fn merge() {
        let byml = load_savedata();
        let savedata = super::SaveData::try_from(&byml).unwrap();
        let byml2 = load_mod_savedata();
        let savedata2 = super::SaveData::try_from(&byml2).unwrap();
        let diff = savedata.diff(&savedata2);
        let merged = savedata.merge(&diff);
        assert_eq!(merged, savedata2);
    }
}