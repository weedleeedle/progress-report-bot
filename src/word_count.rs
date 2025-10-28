//! This module handles parsing and describing word count, including total vs relative.

use std::str::FromStr;

/// Represents a parsed word count argument, which can either be relative or overall.
/// If a number parsed by WordCountArgument starts with '+' or '-' it is treated as relative,
/// otherwise it is treated as total
#[derive(Debug, PartialEq, Eq)]
pub enum WordCountArgument
{
    Relative(i32),
    Total(u32),
}

impl FromStr for WordCountArgument
{
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut relative = false;
        let mut remainder = s;
        if s.starts_with("+") || s.starts_with("-")
        {
            relative = true;
            remainder = &s[1..];
        }
        // We filter out any commas so that numbers formatted like 1,234 
        // Don't break the parser.
        let parsed_remainder: u32 = remainder.chars().filter(|x| *x != ',').collect::<String>().parse()?;
        Ok(match relative
        {
            true => Self::Relative(match s.chars().nth(0).unwrap() {
                '+' => parsed_remainder.try_into().unwrap(),
                '-' => TryInto::<i32>::try_into(parsed_remainder).unwrap() * -1,
                _ => unreachable!()
            }),
            false => Self::Total(parsed_remainder),
        })
    }
}

impl WordCountArgument
{
    /// Converts a relative or total word count into only a total word count.
    /// If the word count internally is a total word count, we just return the same value.
    /// Otherwise we add the relative offset to the total word count.
    ///
    /// # Arguments
    ///
    /// * `current_word_count` - The current total word count.
    ///
    /// # Examples
    ///
    /// ```
    /// # use progress_report_bot::word_count::WordCountArgument;
    /// # use progress_report_bot::word_count::TotalWordCount;
    /// let relative_word_count = WordCountArgument::Relative(100);
    /// let total_word_count = relative_word_count.convert_to_total(50);
    /// assert_eq!(total_word_count.word_count(), 150);
    ///
    /// let total_word_count = WordCountArgument::Total(100);
    /// let new_total_word_count = total_word_count.convert_to_total(50);
    /// assert_eq!(new_total_word_count.word_count(), 100);
    /// ```
    pub fn convert_to_total(&self, current_word_count: u32) -> TotalWordCount
    {
        match self
        {
            Self::Relative(x) => TotalWordCount(
                i32::max(x + (current_word_count as i32), 0).try_into().unwrap()
            ),
            Self::Total(x) => TotalWordCount(*x)
        }
    }
}

/// Represents a project's/user's total word count.
pub struct TotalWordCount(u32);

impl TotalWordCount
{
    pub fn word_count(&self) -> u32
    {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_parse_word_count_total()
    {
        let wc = WordCountArgument::from_str("1234").unwrap();
        assert_eq!(wc, WordCountArgument::Total(1234));
    }

    #[test]
    pub fn test_parse_word_count_relative()
    {
        let wc = WordCountArgument::from_str("+50").unwrap();
        assert_eq!(wc, WordCountArgument::Relative(50));
    }

    #[test]
    pub fn test_parse_word_count_relative_negative()
    {
        let wc = WordCountArgument::from_str("-1579").unwrap();
        assert_eq!(wc, WordCountArgument::Relative(-1579));
    }

    #[test]
    pub fn test_parse_word_count_total_with_commas()
    {
        let wc = WordCountArgument::from_str("123,456").unwrap();
        assert_eq!(wc, WordCountArgument::Total(123456));
    }

    #[test]
    pub fn test_parse_word_count_relative_with_commas()
    {
        let wc = WordCountArgument::from_str("+12,999").unwrap();
        assert_eq!(wc, WordCountArgument::Relative(12999));
    }

    #[test]
    pub fn test_parse_invalid_string_fails()
    {
        let wc_result = WordCountArgument::from_str("abcshdf");
        assert!(wc_result.is_err());
    }

    #[test]
    pub fn test_convert_total_to_total_replaces_total()
    {
        let wc = WordCountArgument::Total(1234);
        let total = wc.convert_to_total(100);
        assert_eq!(total.word_count(), 1234);
    }

    #[test]
    pub fn test_convert_relative_to_total_updates_total()
    {
        let wc = WordCountArgument::Relative(100);
        let total = wc.convert_to_total(250);
        assert_eq!(total.word_count(), 350);
    }

    #[test]
    pub fn test_convert_relative_subtracts_from_total()
    {
        let wc = WordCountArgument::Relative(-100);
        let total = wc.convert_to_total(150);
        assert_eq!(total.word_count(), 50);
    }

    #[test]
    pub fn test_convert_relative_minimum_is_zero()
    {
        let wc = WordCountArgument::Relative(-90000);
        let total = wc.convert_to_total(100);
        assert_eq!(total.word_count(), 0);
    }
}

