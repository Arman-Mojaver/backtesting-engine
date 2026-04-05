use crate::domain::operation_point::{OperationPoint, OperationPointRepository};
use crate::domain::resampled_point::{ResampledPointD1, ResampledPointD1Repository};

pub struct InMemoryOperationRepo {
    pub data: Vec<OperationPoint>,
}

impl OperationPointRepository for InMemoryOperationRepo {
    fn get_all(&self) -> Vec<OperationPoint> {
        self.data.clone()
    }
}

pub struct InMemoryResampledRepo {
    pub data: Vec<ResampledPointD1>,
}

impl ResampledPointD1Repository for InMemoryResampledRepo {
    fn get_all(&self) -> Vec<ResampledPointD1> {
        self.data.clone()
    }
}