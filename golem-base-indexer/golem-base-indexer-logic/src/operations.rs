use crate::types::{OperationData, OperationType};

impl From<OperationData> for OperationType {
    fn from(value: OperationData) -> Self {
        match value {
            OperationData::Create(_, _) => Self::Create,
            OperationData::Update(_, _) => Self::Update,
            OperationData::Delete => Self::Delete,
            OperationData::Extend(_) => Self::Extend,
        }
    }
}
