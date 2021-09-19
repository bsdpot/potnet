use std::str::FromStr;

pub(crate) fn get_value<T>(line: &str) -> Option<T>
where
    T: FromStr,
{
    match line.split('=').nth(1) {
        Some(value) => match value.split(' ').next() {
            Some(value) => value.parse().ok(),
            None => None,
        },
        None => None,
    }
}

#[cfg(test)]

mod tests {
    use super::*;

    #[test]
    fn test_get_value_int() {
        let uut: Option<u32> = get_value("key=1");
        assert_eq!(uut, Some(1u32));
        let uut: Option<u32> = get_value("key=1.");
        assert_eq!(uut, None);
        let uut: Option<u32> = get_value("key=NaN");
        assert_eq!(uut, None);
    }
    #[test]
    fn test_get_value_string() {
        let uut: Option<String> = get_value("key=1");
        assert_eq!(uut, Some("1".to_string()));
        let uut: Option<String> = get_value("key=1.");
        assert_eq!(uut, Some("1.".to_string()));
        let uut: Option<String> = get_value("key=Catched Ignore");
        assert_eq!(uut, Some("Catched".to_string()));
    }
}
