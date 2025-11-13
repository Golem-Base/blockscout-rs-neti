use crate::types::{Address, Operation, OperationData, OperationType};

impl From<OperationData> for OperationType {
    fn from(value: OperationData) -> Self {
        match value {
            OperationData::Create(_, _) => Self::Create,
            OperationData::Update(_, _) => Self::Update,
            OperationData::Delete => Self::Delete,
            OperationData::Extend(_) => Self::Extend,
            OperationData::ChangeOwner(_) => Self::ChangeOwner,
        }
    }
}

impl Operation {
    pub fn owner(&self) -> Address {
        match self.operation {
            OperationData::ChangeOwner(new_owner) => new_owner,
            _ => self.metadata.sender,
        }
    }
}
