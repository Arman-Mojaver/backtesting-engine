use crate::domain::operation_point::{
    LongOperationPointRepository, OperationPoint, ShortOperationPointRepository,
};
use crate::domain::resampled_point::{ResampledPointD1, ResampledPointD1Repository};

pub struct InMemoryLongRepo {
    pub data: Vec<OperationPoint>,
}

impl LongOperationPointRepository for InMemoryLongRepo {
    fn get_all(&self) -> Vec<OperationPoint> {
        self.data.clone()
    }
}

pub struct InMemoryShortRepo {
    pub data: Vec<OperationPoint>,
}

impl ShortOperationPointRepository for InMemoryShortRepo {
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
