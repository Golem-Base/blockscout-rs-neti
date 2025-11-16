use crate::{
    types::{Address, Operation, OperationData, OperationType},
    well_known,
};

impl From<OperationData> for OperationType {
    fn from(value: OperationData) -> Self {
        match value {
            OperationData::Create(_, _, _) => Self::Create,
            OperationData::Update(_, _, _) => Self::Update,
            OperationData::Delete => Self::Delete,
            OperationData::Extend(_) => Self::Extend,
            OperationData::ChangeOwner(_) => Self::ChangeOwner,
        }
    }
}

impl Operation {
    pub fn owner(&self) -> Option<Address> {
        match self.operation {
            OperationData::Delete
                if self.metadata.recipient == well_known::L1_BLOCK_CONTRACT_ADDRESS =>
            {
                None
            }
            OperationData::ChangeOwner(new_owner) => Some(new_owner),
            _ => Some(self.metadata.sender),
        }
    }
}
