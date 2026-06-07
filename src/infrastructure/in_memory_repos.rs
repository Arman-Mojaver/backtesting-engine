use crate::domain::indicator::{Indicator, IndicatorRepository};
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

    fn get_by_mms_id_and_instrument(&self, mms_id: i32, instrument: &str) -> Vec<OperationPoint> {
        self.data
            .iter()
            .filter(|op| op.money_management_strategy_id == mms_id && op.instrument == instrument)
            .cloned()
            .collect()
    }
}

pub struct InMemoryShortRepo {
    pub data: Vec<OperationPoint>,
}

impl ShortOperationPointRepository for InMemoryShortRepo {
    fn get_all(&self) -> Vec<OperationPoint> {
        self.data.clone()
    }

    fn get_by_mms_id_and_instrument(&self, mms_id: i32, instrument: &str) -> Vec<OperationPoint> {
        self.data
            .iter()
            .filter(|op| op.money_management_strategy_id == mms_id && op.instrument == instrument)
            .cloned()
            .collect()
    }
}

pub struct InMemoryResampledRepo {
    pub data: Vec<ResampledPointD1>,
}

impl ResampledPointD1Repository for InMemoryResampledRepo {
    fn get_all(&self) -> Vec<ResampledPointD1> {
        self.data.clone()
    }

    fn get_by_instrument(&self, instrument: &str) -> Vec<ResampledPointD1> {
        self.data
            .iter()
            .filter(|rp| rp.instrument == instrument)
            .cloned()
            .collect()
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

    fn get_by_type(&self, indicator_type: &str) -> Vec<Indicator> {
        self.data
            .iter()
            .filter(|ind| ind.r#type == indicator_type)
            .cloned()
            .collect()
    }
}
