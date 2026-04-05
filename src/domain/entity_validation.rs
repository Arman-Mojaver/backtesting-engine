use crate::domain::indicator::IndicatorRepository;
use crate::domain::operation_point::{LongOperationPointRepository, ShortOperationPointRepository};
use crate::domain::resampled_point::ResampledPointD1Repository;

#[derive(Debug, Clone, PartialEq)]
pub enum ValidationError {
    LongOperationPointsEmpty,
    ShortOperationPointsEmpty,
    IndicatorsEmpty,
    ResampledPointsEmpty,
}

pub fn validate_entities<LPR, SPR, IR, RPR>(
    long_repo: &LPR,
    short_repo: &SPR,
    indicator_repo: &IR,
    resampled_repo: &RPR,
) -> Result<(), ValidationError>
where
    LPR: LongOperationPointRepository,
    SPR: ShortOperationPointRepository,
    IR: IndicatorRepository,
    RPR: ResampledPointD1Repository,
{
    if long_repo.get_all().is_empty() {
        return Err(ValidationError::LongOperationPointsEmpty);
    }

    if short_repo.get_all().is_empty() {
        return Err(ValidationError::ShortOperationPointsEmpty);
    }

    if indicator_repo.get_all().is_empty() {
        return Err(ValidationError::IndicatorsEmpty);
    }

    if resampled_repo.get_all().is_empty() {
        return Err(ValidationError::ResampledPointsEmpty);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::fixtures::{
        indicators_empty, indicators_default, op_points_empty, op_points_default,
        resampled_points_empty, resampled_points_default,
    };
    use crate::infrastructure::in_memory_repos::{
        InMemoryIndicatorRepo, InMemoryLongRepo, InMemoryResampledRepo, InMemoryShortRepo,
    };

    fn long_with_data() -> InMemoryLongRepo {
        InMemoryLongRepo {
            data: op_points_default(),
        }
    }

    fn short_with_data() -> InMemoryShortRepo {
        InMemoryShortRepo {
            data: op_points_default(),
        }
    }

    fn indicators_with_data() -> InMemoryIndicatorRepo {
        InMemoryIndicatorRepo {
            data: indicators_default(),
        }
    }

    fn resampled_with_data() -> InMemoryResampledRepo {
        InMemoryResampledRepo {
            data: resampled_points_default(),
        }
    }

    #[test]
    fn test_all_empty() {
        let long_repo = InMemoryLongRepo { data: op_points_empty() };
        let short_repo = InMemoryShortRepo { data: op_points_empty() };
        let indicator_repo = InMemoryIndicatorRepo { data: indicators_empty() };
        let resampled_repo = InMemoryResampledRepo { data: resampled_points_empty() };

        assert_eq!(
            validate_entities(&long_repo, &short_repo, &indicator_repo, &resampled_repo),
            Err(ValidationError::LongOperationPointsEmpty)
        );
    }

    #[test]
    fn test_long_empty_only() {
        let long_repo = InMemoryLongRepo { data: op_points_empty() };
        let short_repo = short_with_data();
        let indicator_repo = indicators_with_data();
        let resampled_repo = resampled_with_data();

        assert_eq!(
            validate_entities(&long_repo, &short_repo, &indicator_repo, &resampled_repo),
            Err(ValidationError::LongOperationPointsEmpty)
        );
    }

    #[test]
    fn test_short_empty_only() {
        let long_repo = long_with_data();
        let short_repo = InMemoryShortRepo { data: op_points_empty() };
        let indicator_repo = indicators_with_data();
        let resampled_repo = resampled_with_data();

        assert_eq!(
            validate_entities(&long_repo, &short_repo, &indicator_repo, &resampled_repo),
            Err(ValidationError::ShortOperationPointsEmpty)
        );
    }

    #[test]
    fn test_indicators_empty_only() {
        let long_repo = long_with_data();
        let short_repo = short_with_data();
        let indicator_repo = InMemoryIndicatorRepo { data: indicators_empty() };
        let resampled_repo = resampled_with_data();

        assert_eq!(
            validate_entities(&long_repo, &short_repo, &indicator_repo, &resampled_repo),
            Err(ValidationError::IndicatorsEmpty)
        );
    }

    #[test]
    fn test_resampled_empty_only() {
        let long_repo = long_with_data();
        let short_repo = short_with_data();
        let indicator_repo = indicators_with_data();
        let resampled_repo = InMemoryResampledRepo { data: resampled_points_empty() };

        assert_eq!(
            validate_entities(&long_repo, &short_repo, &indicator_repo, &resampled_repo),
            Err(ValidationError::ResampledPointsEmpty)
        );
    }

    #[test]
    fn test_all_have_data() {
        let long_repo = long_with_data();
        let short_repo = short_with_data();
        let indicator_repo = indicators_with_data();
        let resampled_repo = resampled_with_data();

        assert_eq!(
            validate_entities(&long_repo, &short_repo, &indicator_repo, &resampled_repo),
            Ok(())
        );
    }
}