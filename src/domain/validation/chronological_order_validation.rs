use itertools::Itertools;

use crate::domain::resampled_point::ResampledPointD1;

use super::ValidationError;

pub fn validate_chronological_order(
    resampled_points: &[ResampledPointD1],
) -> Result<(), ValidationError> {
    let first_out_of_order_pair = resampled_points
        .iter()
        .map(|resampled_point| resampled_point.timestamp)
        .tuple_windows()
        .find(|(previous, timestamp)| timestamp <= previous);

    if let Some((previous, timestamp)) = first_out_of_order_pair {
        return Err(ValidationError::ResampledPointsNotChronological {
            previous,
            timestamp,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::fixtures::resampled_points_at;

    const fn not_chronological(previous: i64, timestamp: i64) -> Result<(), ValidationError> {
        Err(ValidationError::ResampledPointsNotChronological {
            previous,
            timestamp,
        })
    }

    #[test]
    fn accepts_strictly_increasing_timestamps() {
        let resampled = resampled_points_at(&[1000, 2000, 3000]);

        assert_eq!(validate_chronological_order(&resampled), Ok(()));
    }

    #[test]
    fn accepts_single_resampled_point() {
        let resampled = resampled_points_at(&[1000]);

        assert_eq!(validate_chronological_order(&resampled), Ok(()));
    }

    #[test]
    fn accepts_empty_slice() {
        assert_eq!(validate_chronological_order(&[]), Ok(()));
    }

    #[test]
    fn rejects_later_point_with_earlier_timestamp() {
        let resampled = resampled_points_at(&[1000, 3000, 2000]);

        assert_eq!(
            validate_chronological_order(&resampled),
            not_chronological(3000, 2000)
        );
    }

    #[test]
    fn rejects_two_points_with_same_timestamp() {
        let resampled = resampled_points_at(&[1000, 2000, 2000]);

        assert_eq!(
            validate_chronological_order(&resampled),
            not_chronological(2000, 2000)
        );
    }

    #[test]
    fn reports_first_offending_pair_when_there_are_several() {
        let resampled = resampled_points_at(&[1000, 4000, 3000, 2000]);

        assert_eq!(
            validate_chronological_order(&resampled),
            not_chronological(4000, 3000)
        );
    }
}
