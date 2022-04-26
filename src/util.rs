pub fn mean(data: &[f64]) -> Option<f64> {
    let sum = data.iter().sum::<f64>();
    let count = data.len();

    match count {
        positive if positive > 0 => Some(sum / count as f64),
        _ => None,
    }
}

pub fn std_dev(data: &[f64]) -> Option<f64> {
    match (mean(data), data.len()) {
        (Some(data_mean), count) if count > 0 => {
            let variance = data
                .iter()
                .map(|value| {
                    let diff = data_mean - *value;

                    diff * diff
                })
                .sum::<f64>()
                / count as f64;

            Some(variance.sqrt())
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_mean() {
        assert_eq!(mean(&[10., 20., 30., 15., 22.]), Some(19.4));
        assert_eq!(mean(&[15., 7., 55., 12., 4.]), Some(18.6));
    }

    #[test]
    fn test_std_dev() {
        assert_eq!(
            std_dev(&[100., 120., 90., 102., 94.]),
            Some(10.322790320451151)
        );
        assert_eq!(std_dev(&[15., 7., 55.]), Some(20.997354330698162));
    }
}
