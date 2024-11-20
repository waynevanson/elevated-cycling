use rangetools::{Bound, BoundedRange, LowerBound, UpperBound};
use std::{
    fmt::{Debug, Display},
    str::FromStr,
};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum BoundedRangeParseError<T>
where
    T: FromStr,
    T::Err: Display,
{
    #[error("{0}")]
    ValueError(T::Err),
    #[error("Expected start bound to be [ or (")]
    MissingStartBound,
    #[error("Expected to find one comma, but found more than one comma")]
    MoreThanOneComma,
    #[error("Expected to find one comma, but found less than one comma")]
    LessThanOneComma,
    #[error("Expected end bound to be ] or )")]
    MissingEndBound,
}

pub fn try_from_str<T>(str: &str) -> Result<BoundedRange<T>, BoundedRangeParseError<T>>
where
    T: FromStr,
    T::Err: Display,
{
    use BoundedRangeParseError::*;

    let last = str.len() - 1;
    let middle = &str[1..last];

    let index = find_index_of_only_comma(middle)?;

    let first = &str[1..index];
    let value = T::from_str(first).map_err(ValueError)?;
    let start: LowerBound<T> = match &str[0..1] {
        "[" => Bound::Included(value),
        "(" => Bound::Excluded(value),
        _ => Err(MissingStartBound)?,
    }
    .into();

    let second = &str[(index + 1)..last];
    let value = T::from_str(second).map_err(ValueError)?;
    let end: UpperBound<T> = match &str[str.len() - 1..str.len()] {
        "]" => Bound::Included(value),
        ")" => Bound::Excluded(value),
        _ => Err(MissingEndBound)?,
    }
    .into();

    let bounded_range = BoundedRange { start, end };

    Ok(bounded_range)
}

fn find_index_of_only_comma<T>(str: &str) -> Result<usize, BoundedRangeParseError<T>>
where
    T: FromStr,
    T::Err: Display,
{
    use BoundedRangeParseError::*;

    let mut comma = None;
    for (index, char) in str.chars().enumerate() {
        if char != ',' {
            continue;
        }

        if matches!(comma, Some(_)) {
            Err(MoreThanOneComma)?
        }

        comma = Some(index + 1);
    }

    let index = comma.ok_or(LessThanOneComma)?;

    Ok(index)
}

#[cfg(test)]
mod test {
    use bytesize::ByteSize;

    use super::*;

    #[test]
    fn i() {
        let str = "(1KB,10MB)";
        let expected = (1_000, 10_000_000);

        assert_eq!(
            try_from_str::<ByteSize>(str).unwrap(),
            BoundedRange::new(
                LowerBound::excluded(ByteSize::b(expected.0)),
                UpperBound::excluded(ByteSize::b(expected.1))
            )
        );
    }
}
