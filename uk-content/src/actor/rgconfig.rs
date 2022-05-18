use crate::{prelude::Mergeable, util, Result, UKError};
use roead::aamp::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RagdollConfig {
    pub attack_type_impulse_data: ParameterObject,
    pub impact_impulse_info: BTreeMap<usize, ParameterList>,
}

impl TryFrom<&ParameterIO> for RagdollConfig {
    type Error = UKError;

    fn try_from(pio: &ParameterIO) -> Result<Self> {
        let root = pio.list("ConfigRoot").ok_or(UKError::MissingAampKey(
            "Ragdoll config missing config root list",
        ))?;
        Ok(Self {
            attack_type_impulse_data: root
                .object("AttackTypeImpulseData")
                .ok_or(UKError::MissingAampKey(
                    "Ragdoll config missing attack type impulse data",
                ))?
                .clone(),
            impact_impulse_info: root.lists.0.values().cloned().enumerate().collect(),
        })
    }
}

impl From<RagdollConfig> for ParameterIO {
    fn from(val: RagdollConfig) -> Self {
        Self::new().with_list(
            "ConfigRoot",
            ParameterList::new()
                .with_object("AttackTypeImpulseData", val.attack_type_impulse_data)
                .with_lists(
                    val.impact_impulse_info
                        .into_iter()
                        .map(|(i, list)| (format!("ImpactImpulseInfo{:02}", i), list)),
                ),
        )
    }
}

impl Mergeable<ParameterIO> for RagdollConfig {
    fn diff(&self, other: &Self) -> Self {
        Self {
            attack_type_impulse_data: util::diff_pobj(
                &self.attack_type_impulse_data,
                &other.attack_type_impulse_data,
            ),
            impact_impulse_info: util::simple_index_diff(
                &self.impact_impulse_info,
                &other.impact_impulse_info,
            ),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            attack_type_impulse_data: util::merge_pobj(
                &self.attack_type_impulse_data,
                &diff.attack_type_impulse_data,
            ),
            impact_impulse_info: util::simple_index_merge(
                &self.impact_impulse_info,
                &diff.impact_impulse_info,
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[test]
    fn serde() {
        let actor = crate::tests::test_base_actorpack("Enemy_Moriblin_Junior");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/RagdollConfig/Moriblin_Blue_Bomb.brgconfig")
                .unwrap(),
        )
        .unwrap();
        let rgconfig = super::RagdollConfig::try_from(&pio).unwrap();
        let data = rgconfig.clone().into_pio().to_binary();
        let pio2 = roead::aamp::ParameterIO::from_binary(&data).unwrap();
        let rgconfig2 = super::RagdollConfig::try_from(&pio2).unwrap();
        assert_eq!(rgconfig, rgconfig2);
    }

    #[test]
    fn diff() {
        let actor = crate::tests::test_base_actorpack("Enemy_Moriblin_Junior");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/RagdollConfig/Moriblin_Blue_Bomb.brgconfig")
                .unwrap(),
        )
        .unwrap();
        let rgconfig = super::RagdollConfig::try_from(&pio).unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Enemy_Moriblin_Junior");
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_file_data("Actor/RagdollConfig/Moriblin_Blue_Bomb.brgconfig")
                .unwrap(),
        )
        .unwrap();
        let rgconfig2 = super::RagdollConfig::try_from(&pio2).unwrap();
        let diff = rgconfig.diff(&rgconfig2);
        println!("{}", serde_json::to_string_pretty(&diff).unwrap());
    }

    #[test]
    fn merge() {
        let actor = crate::tests::test_base_actorpack("Enemy_Moriblin_Junior");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/RagdollConfig/Moriblin_Blue_Bomb.brgconfig")
                .unwrap(),
        )
        .unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Enemy_Moriblin_Junior");
        let rgconfig = super::RagdollConfig::try_from(&pio).unwrap();
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_file_data("Actor/RagdollConfig/Moriblin_Blue_Bomb.brgconfig")
                .unwrap(),
        )
        .unwrap();
        let rgconfig2 = super::RagdollConfig::try_from(&pio2).unwrap();
        let diff = rgconfig.diff(&rgconfig2);
        let merged = rgconfig.merge(&diff);
        assert_eq!(rgconfig2, merged);
    }
}