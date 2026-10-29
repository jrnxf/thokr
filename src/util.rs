pub fn mean(data: &[f64]) -> Option<f64> {
    if data.is_empty() {
        return None;
    }
    Some(data.iter().sum::<f64>() / data.len() as f64)
}

/// Population standard deviation; None for no data.
pub fn std_dev(data: &[f64]) -> Option<f64> {
    let mean = mean(data)?;
    let variance = data
        .iter()
        .map(|value| {
            let diff = mean - value;
            diff * diff
        })
        .sum::<f64>()
        / data.len() as f64;
    Some(variance.sqrt())
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
