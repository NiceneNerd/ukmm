mod collections;

pub use collections::*;
use roead::aamp::*;
use roead::byml::Byml;
use std::collections::BTreeMap;

pub fn diff_plist<P: ParamList + From<ParameterList>>(base: &P, other: &P) -> P {
    ParameterList {
        lists: other
            .lists()
            .0
            .iter()
            .filter_map(|(k, v)| {
                if !base.lists().0.contains_key(k) {
                    Some((*k, v.clone()))
                } else if base.lists().0[k] != *v {
                    Some((*k, diff_plist(&base.lists().0[k], v)))
                } else {
                    None
                }
            })
            .collect(),
        objects: other
            .objects()
            .0
            .iter()
            .filter_map(|(k, v)| {
                if !base.objects().0.contains_key(k) {
                    Some((*k, v.clone()))
                } else if base.objects().0[k] != *v {
                    Some((*k, diff_pobj(&base.objects().0[k], v)))
                } else {
                    None
                }
            })
            .collect(),
    }
    .into()
}

pub fn diff_pobj(base: &ParameterObject, other: &ParameterObject) -> ParameterObject {
    other
        .0
        .iter()
        .filter_map(|(k, v)| {
            if !base.0.contains_key(k) || base.0[k] != *v {
                Some((*k, v.clone()))
            } else {
                None
            }
        })
        .collect()
}

pub fn merge_plist<P: ParamList + From<ParameterList>>(base: &P, diff: &P) -> P {
    ParameterList {
        objects: {
            let mut new = base.objects().clone();
            for (k, v) in &diff.objects().0 {
                if !new.0.contains_key(k) {
                    new.0.insert(*k, v.clone());
                } else {
                    new.0[k] = merge_pobj(&new.0[k], v);
                }
            }
            new
        },
        lists: {
            let mut new = base.lists().clone();
            for (k, v) in &diff.lists().0 {
                if !new.0.contains_key(k) {
                    new.0.insert(*k, v.clone());
                } else {
                    new.0[k] = merge_plist(&new.0[k], v);
                }
            }
            new
        },
    }
    .into()
}

pub fn merge_pobj(base: &ParameterObject, diff: &ParameterObject) -> ParameterObject {
    base.0
        .iter()
        .chain(diff.0.iter())
        .map(|(k, v)| (*k, v.clone()))
        .collect()
}

pub fn diff_byml_shallow(base: &Byml, other: &Byml) -> Byml {
    if let Byml::Hash(base) = &base && let &Byml::Hash(other) = &other {
        Byml::Hash(other.iter().filter_map(|(key, value)| {
            if base.get(key) != Some(value) {
                Some((key.clone(), value.clone()))
            } else {
                None
            }
        }).chain(
            base.keys().filter_map(|key| (!other.contains_key(key)).then(|| (key.clone(), Byml::Null)))
        ).collect())
    } else {
        panic!("Can only shallow diff BYML hashes")
    }
}

pub fn merge_byml_shallow(base: &Byml, diff: &Byml) -> Byml {
    match (base, diff) {
        (Byml::Hash(base), Byml::Hash(diff)) => Byml::Hash(
            base.iter()
                .chain(diff.iter())
                .map(|(k, v)| (k.to_owned(), v.clone()))
                .collect(),
        ),
        (Byml::Hash(base), Byml::Null) => Byml::Hash(base.clone()),
        _ => panic!("Can only shallow merge BYML hashes"),
    }
}

pub fn simple_index_diff<T: Clone + PartialEq>(
    base: &BTreeMap<usize, T>,
    other: &BTreeMap<usize, T>,
) -> BTreeMap<usize, T> {
    other
        .iter()
        .filter_map(|(i, other_item)| {
            (base.get(i) != Some(other_item)).then(|| (*i, other_item.clone()))
        })
        .collect()
}

pub fn simple_index_merge<T: Clone + PartialEq>(
    base: &BTreeMap<usize, T>,
    diff: &BTreeMap<usize, T>,
) -> BTreeMap<usize, T> {
    base.iter()
        .chain(diff.iter())
        .map(|(i, body)| (*i, body.clone()))
        .collect()
}

#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    Eq,
    PartialEq,
    Hash,
    Ord,
    PartialOrd,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct BymlHashValue(pub u32);

impl TryFrom<Byml> for BymlHashValue {
    type Error = crate::UKError;

    fn try_from(value: Byml) -> crate::Result<Self> {
        Ok(match value {
            Byml::UInt(v) => Self(v),
            Byml::Int(v) => Self(v as u32),
            _ => {
                return Err(crate::UKError::WrongBymlType(
                    roead::byml::BymlError::TypeError,
                ))
            }
        })
    }
}

impl TryFrom<&Byml> for BymlHashValue {
    type Error = crate::UKError;

    fn try_from(value: &Byml) -> crate::Result<Self> {
        Ok(match value {
            Byml::UInt(v) => Self(*v),
            Byml::Int(v) => Self(*v as u32),
            _ => {
                return Err(crate::UKError::WrongBymlType(
                    roead::byml::BymlError::TypeError,
                ))
            }
        })
    }
}

impl From<u32> for BymlHashValue {
    fn from(val: u32) -> Self {
        Self(val)
    }
}

impl From<i32> for BymlHashValue {
    fn from(val: i32) -> Self {
        Self(val as u32)
    }
}

impl From<usize> for BymlHashValue {
    fn from(val: usize) -> Self {
        Self(val as u32)
    }
}

impl From<BymlHashValue> for u32 {
    fn from(val: BymlHashValue) -> Self {
        val.0
    }
}

impl From<BymlHashValue> for i32 {
    fn from(val: BymlHashValue) -> Self {
        val.0 as i32
    }
}

impl From<BymlHashValue> for Byml {
    fn from(val: BymlHashValue) -> Self {
        if val.0 > 0x80000000 {
            Byml::Int(val.0 as i32)
        } else {
            Byml::UInt(val.0)
        }
    }
}

impl From<&BymlHashValue> for Byml {
    fn from(val: &BymlHashValue) -> Self {
        if val.0 > 0x80000000 {
            Byml::Int(val.0 as i32)
        } else {
            Byml::UInt(val.0)
        }
    }
}
