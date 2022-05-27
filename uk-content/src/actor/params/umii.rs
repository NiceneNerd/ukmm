use crate::prelude::*;
use roead::aamp::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct UMii(pub ParameterIO);

impl Convertible<ParameterIO> for UMii {}

impl From<&ParameterIO> for UMii {
    fn from(pio: &ParameterIO) -> Self {
        Self(pio.clone())
    }
}

impl From<ParameterIO> for UMii {
    fn from(pio: ParameterIO) -> Self {
        Self(pio)
    }
}

impl From<UMii> for ParameterIO {
    fn from(val: UMii) -> Self {
        val.0
    }
}

impl SimpleMergeableAamp for UMii {
    fn inner(&self) -> &ParameterIO {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[test]
    fn serde() {
        let actor = crate::tests::test_base_actorpack("Npc_TripMaster_00");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/UMii/Npc_TripMaster_00.bumii")
                .unwrap(),
        )
        .unwrap();
        let umii = super::UMii::try_from(&pio).unwrap();
        let data = umii.clone().into_pio().to_binary();
        let pio2 = roead::aamp::ParameterIO::from_binary(&data).unwrap();
        let umii2 = super::UMii::try_from(&pio2).unwrap();
        assert_eq!(umii, umii2);
    }

    #[test]
    fn diff() {
        let actor = crate::tests::test_base_actorpack("Npc_TripMaster_00");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/UMii/Npc_TripMaster_00.bumii")
                .unwrap(),
        )
        .unwrap();
        let umii = super::UMii::try_from(&pio).unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Npc_TripMaster_00");
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_file_data("Actor/UMii/Npc_TripMaster_00.bumii")
                .unwrap(),
        )
        .unwrap();
        let umii2 = super::UMii::try_from(&pio2).unwrap();
        let diff = umii.diff(&umii2);
        println!("{}", serde_json::to_string_pretty(&diff).unwrap());
    }

    #[test]
    fn merge() {
        let actor = crate::tests::test_base_actorpack("Npc_TripMaster_00");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/UMii/Npc_TripMaster_00.bumii")
                .unwrap(),
        )
        .unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Npc_TripMaster_00");
        let umii = super::UMii::try_from(&pio).unwrap();
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_file_data("Actor/UMii/Npc_TripMaster_00.bumii")
                .unwrap(),
        )
        .unwrap();
        let umii2 = super::UMii::try_from(&pio2).unwrap();
        let diff = umii.diff(&umii2);
        let merged = umii.merge(&diff);
        assert_eq!(umii2, merged);
    }
}