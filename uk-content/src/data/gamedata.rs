use crate::{prelude::*, util::DeleteMap, Result, UKError};
use roead::{
    byml::Byml,
    sarc::{Sarc, SarcWriter},
};
use serde::{Deserialize, Serialize};
use std::hint::unreachable_unchecked;

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct GameData {
    pub data_type: String,
    pub flags: DeleteMap<u32, Byml>,
}

impl TryFrom<&Byml> for GameData {
    type Error = UKError;

    fn try_from(byml: &Byml) -> Result<Self> {
        let hash = byml.as_hash()?;
        Ok(Self {
            data_type: hash
                .keys()
                .next()
                .ok_or(UKError::MissingBymlKey("bgdata file missing data type key"))?
                .into(),
            flags: hash
                .values()
                .next()
                .ok_or(UKError::MissingBymlKey("bgdata file missing data"))?
                .as_array()?
                .iter()
                .map(|item| -> Result<(u32, Byml)> {
                    Ok((
                        item.as_hash()?
                            .get("HashValue")
                            .ok_or(UKError::MissingBymlKey(
                                "bgdata file entry missing HashValue",
                            ))?
                            .as_int()? as u32,
                        item.clone(),
                    ))
                })
                .collect::<Result<_>>()?,
        })
    }
}

impl From<GameData> for Byml {
    fn from(val: GameData) -> Self {
        [(
            val.data_type.to_string(),
            val.flags.values().cloned().collect(),
        )]
        .into_iter()
        .collect()
    }
}

impl GameData {
    fn divide(self) -> Vec<GameData> {
        let total = (self.flags.len() as f32 / 4096.).ceil() as usize;
        let mut iter = self.flags.into_iter();
        let mut out = Vec::with_capacity(total);
        for _ in 0..total {
            out.push(GameData {
                data_type: self.data_type.clone(),
                flags: iter.by_ref().take(4096).collect(),
            });
        }
        out
    }
}

impl Mergeable for GameData {
    fn diff(&self, other: &Self) -> Self {
        assert_eq!(
            self.data_type, other.data_type,
            "Attempted to diff different gamedata types: {} and {}",
            self.data_type, other.data_type
        );
        Self {
            data_type: self.data_type.clone(),
            flags: self.flags.diff(&other.flags),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        assert_eq!(
            self.data_type, diff.data_type,
            "Attempted to merge different gamedata types: {} and {}",
            self.data_type, diff.data_type
        );
        Self {
            data_type: self.data_type.clone(),
            flags: self.flags.merge(&diff.flags),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
pub struct GameDataPack {
    pub bool_array_data: GameData,
    pub bool_data: GameData,
    pub f32_array_data: GameData,
    pub f32_data: GameData,
    pub revival_bool_data: GameData,
    pub revival_s32_data: GameData,
    pub s32_array_data: GameData,
    pub s32_data: GameData,
    pub string32_data: GameData,
    pub string64_array_data: GameData,
    pub string64_data: GameData,
    pub string256_array_data: GameData,
    pub string256_data: GameData,
    pub vector2f_array_data: GameData,
    pub vector2f_data: GameData,
    pub vector3f_array_data: GameData,
    pub vector3f_data: GameData,
    pub vector4f_data: GameData,
}

#[inline(always)]
fn flag_alloc_count(data_type: &str) -> usize {
    match data_type {
        "bool_data" => 9564,
        "f32_data" => 46,
        "s32_data" => 2709,
        "string32_data" => 6,
        "string64_data" => 16,
        "string256_data" => 3,
        "vector2f_data" => 1,
        "vector3f_data" => 44,
        "vector4f_data" => 1,
        "bool_array_data" => 5,
        "f32_array_data" => 3,
        "s32_array_data" => 35,
        "string64_array_data" => 28,
        "string256_array_data" => 4,
        "vector2f_array_data" => 3,
        "vector3f_array_data" => 3,
        "revival_bool_data" => 32461,
        "revival_s32_data" => 83,
        _ => unsafe { unreachable_unchecked() },
    }
}

enum SarcSource<'a> {
    Reader(&'a Sarc<'a>),
    Writer(&'a SarcWriter),
}

impl SarcSource<'_> {
    fn iter(&self) -> Box<dyn Iterator<Item = (&str, &[u8])> + '_> {
        match self {
            Self::Reader(sarc) => Box::new(
                sarc.list_filenames()
                    .into_iter()
                    .map(|f| (f, sarc.get_file_data(f).unwrap())),
            ),
            Self::Writer(sarcwriter) => Box::new(
                sarcwriter
                    .files
                    .iter()
                    .map(|(f, d)| (f.as_ref(), d.as_ref())),
            ),
        }
    }
}

#[inline]
fn extract_gamedata_by_type(sarc: &SarcSource, key: &str) -> Result<GameData> {
    let data_type = if key == "string32_data" {
        "string_data"
    } else {
        key.trim_start_matches("revival_")
    };
    let mut flags = DeleteMap::with_capacity(flag_alloc_count(key));
    for (file, data) in sarc.iter() {
        if file.trim_start_matches('/').starts_with(key) {
            let mut byml = Byml::from_binary(data)?;
            let hash = byml.as_mut_hash()?;
            if let Some(Byml::Array(arr)) = hash.remove(data_type) {
                for item in arr {
                    flags.insert(
                        item.as_hash()?
                            .get("HashValue")
                            .ok_or(UKError::MissingBymlKey(
                                "bgdata file entry missing HashValue",
                            ))?
                            .as_int()? as u32,
                        item,
                    );
                }
            }
        }
    }
    Ok(GameData {
        data_type: data_type.into(),
        flags,
    })
}

macro_rules! build_gamedata_pack {
    ($data:expr, $sarc:expr, $($type:ident),*) => {
        let _endian = $sarc.endian;
        $(
            let _type = $data.$type.data_type.clone();
            let _name_type = stringify!($type);
            $sarc.add_files($data.$type.divide().into_iter().enumerate().map(|(i, data)| {
                (
                    join_str::jstr!("/{_name_type}_{&lexical::to_string(i)}.bgdata"),
                    Byml::from(data).to_binary(_endian),
                )
            }));
        )*
    };
}

impl GameDataPack {
    pub fn from_sarc_writer(sarc: &SarcWriter) -> Result<Self> {
        let source = SarcSource::Writer(sarc);
        if sarc
            .files
            .keys()
            .any(|f| f.trim_start_matches('/').starts_with("revival"))
        {
            Ok(GameDataPack {
                bool_array_data: extract_gamedata_by_type(&source, "bool_array_data")?,
                bool_data: extract_gamedata_by_type(&source, "bool_data")?,
                f32_array_data: extract_gamedata_by_type(&source, "f32_array_data")?,
                f32_data: extract_gamedata_by_type(&source, "f32_data")?,
                revival_bool_data: extract_gamedata_by_type(&source, "revival_bool_data")?,
                revival_s32_data: extract_gamedata_by_type(&source, "revival_s32_data")?,
                s32_array_data: extract_gamedata_by_type(&source, "s32_array_data")?,
                s32_data: extract_gamedata_by_type(&source, "s32_data")?,
                string32_data: extract_gamedata_by_type(&source, "string32_data")?,
                string64_array_data: extract_gamedata_by_type(&source, "string64_array_data")?,
                string64_data: extract_gamedata_by_type(&source, "string64_data")?,
                string256_array_data: extract_gamedata_by_type(&source, "string256_array_data")?,
                string256_data: extract_gamedata_by_type(&source, "string256_data")?,
                vector2f_array_data: extract_gamedata_by_type(&source, "vector2f_array_data")?,
                vector2f_data: extract_gamedata_by_type(&source, "vector2f_data")?,
                vector3f_array_data: extract_gamedata_by_type(&source, "vector3f_array_data")?,
                vector3f_data: extract_gamedata_by_type(&source, "vector3f_data")?,
                vector4f_data: extract_gamedata_by_type(&source, "vector4f_data")?,
            })
        } else {
            Self::from_bcml_sarc(&source)
        }
    }

    const STAGES: &'static [&'static str] =
        &["MainField", "AocField", "CDungeon", "MainFieldDungeon"];

    fn from_bcml_sarc(sarc: &SarcSource) -> Result<Self> {
        let mut revival_bool_data = DeleteMap::with_capacity(32461);
        let mut bool_data = DeleteMap::with_capacity(9564);
        let mut revival_s32_data = DeleteMap::with_capacity(83);
        let mut s32_data = DeleteMap::with_capacity(2709);
        let mut string32_data = DeleteMap::with_capacity(6);
        for (file, data) in sarc.iter() {
            let file_name = file.trim_start_matches('/');
            let mut byml = Byml::from_binary(data)?;
            let hash = byml.as_mut_hash()?;
            if file_name.starts_with("bool_data") {
                if let Some(Byml::Array(arr)) = hash.remove("bool_data") {
                    for item in arr {
                        let name = item
                            .as_hash()?
                            .get("DataName")
                            .ok_or(UKError::MissingBymlKey("Game data entry missing DataName"))?
                            .as_string()?;
                        let parts = name.split('_').collect::<Vec<_>>();
                        let hash_value = item
                            .as_hash()?
                            .get("HashValue")
                            .ok_or(UKError::MissingBymlKey(
                                "bgdata file entry missing HashValue",
                            ))?
                            .as_int()? as u32;
                        if Self::STAGES.contains(&parts[0]) && !name.contains("HiddenKorok") {
                            revival_bool_data.insert(hash_value, item);
                        } else {
                            bool_data.insert(hash_value, item);
                        }
                    }
                }
            } else if file_name.starts_with("s32_data") {
                if let Some(Byml::Array(arr)) = hash.remove("s32_data") {
                    for item in arr {
                        let name = item
                            .as_hash()?
                            .get("DataName")
                            .ok_or(UKError::MissingBymlKey("Game data entry missing DataName"))?
                            .as_string()?;
                        let parts = name.split('_').collect::<Vec<_>>();
                        let hash_value = item
                            .as_hash()?
                            .get("HashValue")
                            .ok_or(UKError::MissingBymlKey(
                                "bgdata file entry missing HashValue",
                            ))?
                            .as_int()? as u32;
                        if Self::STAGES.contains(&parts[0]) {
                            revival_s32_data.insert(hash_value, item);
                        } else {
                            s32_data.insert(hash_value, item);
                        }
                    }
                }
            } else if file_name.starts_with("string_data") {
                if let Some(Byml::Array(arr)) = hash.remove("string_data") {
                    for item in arr {
                        let hash_value = item
                            .as_hash()?
                            .get("HashValue")
                            .ok_or(UKError::MissingBymlKey(
                                "bgdata file entry missing HashValue",
                            ))?
                            .as_int()? as u32;
                        string32_data.insert(hash_value, item);
                    }
                }
            }
        }
        Ok(GameDataPack {
            bool_data: GameData {
                data_type: "bool_data".into(),
                flags: bool_data,
            },
            revival_bool_data: GameData {
                data_type: "bool_data".into(),
                flags: revival_bool_data,
            },
            s32_data: GameData {
                data_type: "s32_data".into(),
                flags: s32_data,
            },
            revival_s32_data: GameData {
                data_type: "s32_data".into(),
                flags: revival_s32_data,
            },
            string32_data: GameData {
                data_type: "string_data".into(),
                flags: string32_data,
            },
            bool_array_data: extract_gamedata_by_type(sarc, "bool_array_data")?,
            s32_array_data: extract_gamedata_by_type(sarc, "s32_array_data")?,
            f32_array_data: extract_gamedata_by_type(sarc, "f32_array_data")?,
            f32_data: extract_gamedata_by_type(sarc, "f32_data")?,
            string64_data: extract_gamedata_by_type(sarc, "string64_data")?,
            string64_array_data: extract_gamedata_by_type(sarc, "string64_array_data")?,
            string256_data: extract_gamedata_by_type(sarc, "string256_data")?,
            string256_array_data: extract_gamedata_by_type(sarc, "string256_array_data")?,
            vector2f_array_data: extract_gamedata_by_type(sarc, "vector2f_array_data")?,
            vector2f_data: extract_gamedata_by_type(sarc, "vector2f_data")?,
            vector3f_array_data: extract_gamedata_by_type(sarc, "vector3f_array_data")?,
            vector3f_data: extract_gamedata_by_type(sarc, "vector3f_data")?,
            vector4f_data: extract_gamedata_by_type(sarc, "vector4f_data")?,
        })
    }

    pub fn from_sarc(sarc: &Sarc<'_>) -> Result<Self> {
        let source = SarcSource::Reader(sarc);
        if sarc.files().any(|f| {
            f.name()
                .unwrap_or("")
                .trim_start_matches('/')
                .starts_with("revival")
        }) {
            Ok(GameDataPack {
                bool_array_data: extract_gamedata_by_type(&source, "bool_array_data")?,
                bool_data: extract_gamedata_by_type(&source, "bool_data")?,
                f32_array_data: extract_gamedata_by_type(&source, "f32_array_data")?,
                f32_data: extract_gamedata_by_type(&source, "f32_data")?,
                revival_bool_data: extract_gamedata_by_type(&source, "revival_bool_data")?,
                revival_s32_data: extract_gamedata_by_type(&source, "revival_s32_data")?,
                s32_array_data: extract_gamedata_by_type(&source, "s32_array_data")?,
                s32_data: extract_gamedata_by_type(&source, "s32_data")?,
                string32_data: extract_gamedata_by_type(&source, "string32_data")?,
                string64_array_data: extract_gamedata_by_type(&source, "string64_array_data")?,
                string64_data: extract_gamedata_by_type(&source, "string64_data")?,
                string256_array_data: extract_gamedata_by_type(&source, "string256_array_data")?,
                string256_data: extract_gamedata_by_type(&source, "string256_data")?,
                vector2f_array_data: extract_gamedata_by_type(&source, "vector2f_array_data")?,
                vector2f_data: extract_gamedata_by_type(&source, "vector2f_data")?,
                vector3f_array_data: extract_gamedata_by_type(&source, "vector3f_array_data")?,
                vector3f_data: extract_gamedata_by_type(&source, "vector3f_data")?,
                vector4f_data: extract_gamedata_by_type(&source, "vector4f_data")?,
            })
        } else {
            Self::from_bcml_sarc(&source)
        }
    }

    pub fn into_sarc_writer(self, endian: Endian) -> SarcWriter {
        let mut sarc = SarcWriter::new(endian.into());
        build_gamedata_pack!(
            self,
            sarc,
            bool_array_data,
            bool_data,
            f32_array_data,
            f32_data,
            revival_bool_data,
            revival_s32_data,
            s32_array_data,
            s32_data,
            string32_data,
            string64_array_data,
            string64_data,
            string256_array_data,
            string256_data,
            vector2f_array_data,
            vector2f_data,
            vector3f_array_data,
            vector3f_data,
            vector4f_data
        );
        sarc
    }
}

impl Mergeable for GameDataPack {
    fn diff(&self, other: &Self) -> Self {
        Self {
            bool_array_data: self.bool_array_data.diff(&other.bool_array_data),
            bool_data: self.bool_data.diff(&other.bool_data),
            f32_array_data: self.f32_array_data.diff(&other.f32_array_data),
            f32_data: self.f32_data.diff(&other.f32_data),
            revival_bool_data: self.revival_bool_data.diff(&other.revival_bool_data),
            revival_s32_data: self.revival_s32_data.diff(&other.revival_s32_data),
            s32_array_data: self.s32_array_data.diff(&other.s32_array_data),
            s32_data: self.s32_data.diff(&other.s32_data),
            string32_data: self.string32_data.diff(&other.string32_data),
            string64_array_data: self.string64_array_data.diff(&other.string64_array_data),
            string64_data: self.string64_data.diff(&other.string64_data),
            string256_array_data: self.string256_array_data.diff(&other.string256_array_data),
            string256_data: self.string256_data.diff(&other.string256_data),
            vector2f_array_data: self.vector2f_array_data.diff(&other.vector2f_array_data),
            vector2f_data: self.vector2f_data.diff(&other.vector2f_data),
            vector3f_array_data: self.vector3f_array_data.diff(&other.vector3f_array_data),
            vector3f_data: self.vector3f_data.diff(&other.vector3f_data),
            vector4f_data: self.vector4f_data.diff(&other.vector4f_data),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            bool_array_data: self.bool_array_data.merge(&diff.bool_array_data),
            bool_data: self.bool_data.merge(&diff.bool_data),
            f32_array_data: self.f32_array_data.merge(&diff.f32_array_data),
            f32_data: self.f32_data.merge(&diff.f32_data),
            revival_bool_data: self.revival_bool_data.merge(&diff.revival_bool_data),
            revival_s32_data: self.revival_s32_data.merge(&diff.revival_s32_data),
            s32_array_data: self.s32_array_data.merge(&diff.s32_array_data),
            s32_data: self.s32_data.merge(&diff.s32_data),
            string32_data: self.string32_data.merge(&diff.string32_data),
            string64_array_data: self.string64_array_data.merge(&diff.string64_array_data),
            string64_data: self.string64_data.merge(&diff.string64_data),
            string256_array_data: self.string256_array_data.merge(&diff.string256_array_data),
            string256_data: self.string256_data.merge(&diff.string256_data),
            vector2f_array_data: self.vector2f_array_data.merge(&diff.vector2f_array_data),
            vector2f_data: self.vector2f_data.merge(&diff.vector2f_data),
            vector3f_array_data: self.vector3f_array_data.merge(&diff.vector3f_array_data),
            vector3f_data: self.vector3f_data.merge(&diff.vector3f_data),
            vector4f_data: self.vector4f_data.merge(&diff.vector4f_data),
        }
    }
}

impl Resource for GameDataPack {
    fn from_binary(data: impl AsRef<[u8]>) -> crate::Result<Self> {
        Self::from_sarc(&Sarc::read(data.as_ref())?)
    }

    fn into_binary(self, endian: Endian) -> roead::Bytes {
        self.into_sarc_writer(endian).to_binary()
    }

    fn path_matches(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref().file_stem().and_then(|name| name.to_str()) == Some("gamedata")
    }
}

single_path!(GameDataPack, "Pack/Bootup.pack//GameData/gamedata.ssarc");

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use roead::{byml::Byml, sarc::Sarc};

    fn load_gamedata_sarc() -> Sarc<'static> {
        Sarc::read(std::fs::read("test/GameData/gamedata.ssarc").unwrap()).unwrap()
    }

    fn load_gamedata() -> Byml {
        let gs = load_gamedata_sarc();
        Byml::from_binary(gs.get_file_data("/string32_data_0.bgdata").unwrap()).unwrap()
    }

    fn load_mod_gamedata() -> Byml {
        let gs = load_gamedata_sarc();
        Byml::from_binary(gs.get_file_data("/string32_data_0.mod.bgdata").unwrap()).unwrap()
    }

    #[test]
    fn serde() {
        let byml = load_gamedata();
        let gamedata = super::GameData::try_from(&byml).unwrap();
        let data = Byml::from(gamedata.clone()).to_binary(roead::Endian::Big);
        let byml2 = Byml::from_binary(&data).unwrap();
        let gamedata2 = super::GameData::try_from(&byml2).unwrap();
        assert_eq!(gamedata, gamedata2);
    }

    #[test]
    fn diff() {
        let byml = load_gamedata();
        let gamedata = super::GameData::try_from(&byml).unwrap();
        let byml2 = load_mod_gamedata();
        let gamedata2 = super::GameData::try_from(&byml2).unwrap();
        let diff = gamedata.diff(&gamedata2);
        dbg!(diff);
    }

    #[test]
    fn merge() {
        let byml = load_gamedata();
        let gamedata = super::GameData::try_from(&byml).unwrap();
        let byml2 = load_mod_gamedata();
        let gamedata2 = super::GameData::try_from(&byml2).unwrap();
        let diff = gamedata.diff(&gamedata2);
        let merged = gamedata.merge(&diff);
        assert_eq!(merged, gamedata2);
    }

    #[test]
    fn pack() {
        let gs = load_gamedata_sarc();
        let gamedata = super::GameDataPack::from_sarc(&gs).unwrap();
        let gs2 = gamedata
            .clone()
            .into_sarc_writer(crate::prelude::Endian::Big);
        let gamedata2 = super::GameDataPack::from_sarc_writer(&gs2).unwrap();
        assert_eq!(gamedata, gamedata2);
    }

    #[test]
    fn identify() {
        let path = std::path::Path::new("content/Pack/Bootup.pack//GameData/gamedata.ssarc");
        assert!(super::GameDataPack::path_matches(path));
    }
}
