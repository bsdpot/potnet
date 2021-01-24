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
