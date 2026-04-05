use crate::domain::operation_point::{LongOperationPointRepository, OperationPoint, ShortOperationPointRepository};
use crate::domain::resampled_point::{ResampledPointD1, ResampledPointD1Repository};
use crate::domain::indicator::{Indicator, IndicatorRepository};

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

#[derive(Debug, Clone)]
pub struct InMemoryIndicatorRepo {
    pub data: Vec<Indicator>,
}

impl IndicatorRepository for InMemoryIndicatorRepo {
    fn get_all(&self) -> Vec<Indicator> {
        self.data.clone()
    }
}