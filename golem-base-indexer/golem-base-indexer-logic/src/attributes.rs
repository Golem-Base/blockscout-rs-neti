use crate::types::Attribute;
use arkiv_storage_tx::Attribute as ArkivAttribute;

impl<T: std::fmt::Debug> From<ArkivAttribute<T>> for Attribute<T> {
    fn from(value: ArkivAttribute<T>) -> Self {
        Self {
            key: value.key,
            value: value.value,
        }
    }
}
