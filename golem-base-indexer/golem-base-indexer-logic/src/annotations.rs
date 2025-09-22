use crate::types::{NumericAnnotation, StringAnnotation};
use golem_base_sdk::{
    NumericAnnotation as GolemNumericAnnotation, StringAnnotation as GolemStringAnnotation,
};

impl From<GolemStringAnnotation> for StringAnnotation {
    fn from(value: GolemStringAnnotation) -> Self {
        Self {
            key: value.key,
            value: value.value,
        }
    }
}

impl From<GolemNumericAnnotation> for NumericAnnotation {
    fn from(value: GolemNumericAnnotation) -> Self {
        Self {
            key: value.key,
            value: value.value,
        }
    }
}
