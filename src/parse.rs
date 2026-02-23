use std::path::PathBuf;

use falsehoods::Truthful;

pub trait FromEnvStr: Sized {
    type Err: std::error::Error + Send + Sync + 'static;
    fn from_env_str(value: &str) -> std::result::Result<Self, Self::Err>;
    fn type_name() -> &'static str;

    fn on_not_found(keys: &[&str]) -> crate::error::Result<Self> {
        Err(crate::error::Error::NotFound {
            keys: keys.join(", "),
            location: crate::error::Location::default(),
        })
    }
}

#[derive(Debug)]
pub struct BoolParseError {
    value: String,
}

impl std::fmt::Display for BoolParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "cannot parse '{}' as boolean", self.value)
    }
}

impl std::error::Error for BoolParseError {}

impl FromEnvStr for bool {
    type Err = BoolParseError;

    fn from_env_str(value: &str) -> std::result::Result<Self, Self::Err> {
        if value.is_true() {
            return Ok(true);
        }
        if value.is_false() {
            return Ok(false);
        }
        Err(BoolParseError { value: value.to_owned() })
    }

    fn type_name() -> &'static str {
        "bool"
    }
}

macro_rules! impl_from_env_str_numeric {
    ($($typ:ty),+) => {
        $(
            impl FromEnvStr for $typ {
                type Err = <$typ as std::str::FromStr>::Err;

                fn from_env_str(value: &str) -> std::result::Result<Self, Self::Err> {
                    value.parse()
                }

                fn type_name() -> &'static str {
                    stringify!($typ)
                }
            }
        )+
    };
}

impl_from_env_str_numeric!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64);

impl FromEnvStr for String {
    type Err = std::convert::Infallible;

    fn from_env_str(value: &str) -> std::result::Result<Self, Self::Err> {
        Ok(value.to_owned())
    }

    fn type_name() -> &'static str {
        "String"
    }
}

impl FromEnvStr for PathBuf {
    type Err = std::convert::Infallible;

    fn from_env_str(value: &str) -> std::result::Result<Self, Self::Err> {
        Ok(PathBuf::from(value))
    }

    fn type_name() -> &'static str {
        "PathBuf"
    }
}

#[derive(Debug)]
pub struct VecParseError {
    index: usize,
    source: Box<dyn std::error::Error + Send + Sync>,
}

impl std::fmt::Display for VecParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "element {}: {}", self.index, self.source)
    }
}

impl std::error::Error for VecParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&*self.source)
    }
}

impl<T: FromEnvStr> FromEnvStr for Option<T> {
    type Err = T::Err;

    fn from_env_str(value: &str) -> std::result::Result<Self, Self::Err> {
        T::from_env_str(value).map(Some)
    }

    fn type_name() -> &'static str {
        T::type_name()
    }

    fn on_not_found(_keys: &[&str]) -> crate::error::Result<Self> {
        Ok(None)
    }
}

#[cfg(feature = "chrono")]
mod chrono_impls {
    use super::FromEnvStr;

    #[derive(Debug)]
    pub struct ChronoParseError {
        value: String,
        type_name: &'static str,
    }

    impl std::fmt::Display for ChronoParseError {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(formatter, "cannot parse '{}' as {}", self.value, self.type_name)
        }
    }

    impl std::error::Error for ChronoParseError {}

    const DATETIME_FORMATS: &[&str] = &[
        "%Y-%m-%dT%H:%M:%S%.f%:z",
        "%Y-%m-%dT%H:%M:%S%:z",
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%dT%H:%M",
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%d %H:%M",
        "%Y-%m-%d",
        "%Y/%m/%d %H:%M:%S",
        "%Y/%m/%d",
        "%m/%d/%Y %H:%M:%S",
        "%m/%d/%Y",
    ];

    impl FromEnvStr for chrono::NaiveDateTime {
        type Err = ChronoParseError;

        fn from_env_str(value: &str) -> std::result::Result<Self, Self::Err> {
            let trimmed = value.trim();
            if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(trimmed) {
                return Ok(parsed.naive_utc());
            }
            for format in DATETIME_FORMATS {
                if let Ok(parsed) = chrono::NaiveDateTime::parse_from_str(trimmed, format) {
                    return Ok(parsed);
                }
            }
            Err(ChronoParseError {
                value: value.to_owned(),
                type_name: "NaiveDateTime",
            })
        }

        fn type_name() -> &'static str {
            "NaiveDateTime"
        }
    }

    impl FromEnvStr for chrono::DateTime<chrono::Utc> {
        type Err = ChronoParseError;

        fn from_env_str(value: &str) -> std::result::Result<Self, Self::Err> {
            let trimmed = value.trim();
            if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(trimmed) {
                return Ok(parsed.to_utc());
            }
            for format in DATETIME_FORMATS {
                if let Ok(parsed) = chrono::DateTime::parse_from_str(trimmed, format) {
                    return Ok(parsed.to_utc());
                }
            }
            // fall back to naive parsing and assume UTC
            let naive = chrono::NaiveDateTime::from_env_str(value)?;
            Ok(naive.and_utc())
        }

        fn type_name() -> &'static str {
            "DateTime<Utc>"
        }
    }

    impl FromEnvStr for chrono::NaiveDate {
        type Err = ChronoParseError;

        fn from_env_str(value: &str) -> std::result::Result<Self, Self::Err> {
            let trimmed = value.trim();
            let date_formats = &["%Y-%m-%d", "%Y/%m/%d", "%m/%d/%Y"];
            for format in date_formats {
                if let Ok(parsed) = chrono::NaiveDate::parse_from_str(trimmed, format) {
                    return Ok(parsed);
                }
            }
            Err(ChronoParseError {
                value: value.to_owned(),
                type_name: "NaiveDate",
            })
        }

        fn type_name() -> &'static str {
            "NaiveDate"
        }
    }

    impl FromEnvStr for chrono::NaiveTime {
        type Err = ChronoParseError;

        fn from_env_str(value: &str) -> std::result::Result<Self, Self::Err> {
            let trimmed = value.trim();
            let time_formats = &["%H:%M:%S%.f", "%H:%M:%S", "%H:%M"];
            for format in time_formats {
                if let Ok(parsed) = chrono::NaiveTime::parse_from_str(trimmed, format) {
                    return Ok(parsed);
                }
            }
            Err(ChronoParseError {
                value: value.to_owned(),
                type_name: "NaiveTime",
            })
        }

        fn type_name() -> &'static str {
            "NaiveTime"
        }
    }
}

impl<T: FromEnvStr> FromEnvStr for Vec<T> {
    type Err = VecParseError;

    fn from_env_str(value: &str) -> std::result::Result<Self, Self::Err> {
        if value.is_empty() {
            return Ok(Vec::new());
        }
        value
            .split(',')
            .enumerate()
            .map(|(index, element)| T::from_env_str(element.trim()).map_err(|source| VecParseError { index, source: Box::new(source) }))
            .collect()
    }

    fn type_name() -> &'static str {
        "Vec"
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case("true", true)]
    #[case("yes", true)]
    #[case("1", true)]
    #[case("on", true)]
    #[case("enabled", true)]
    #[case("e", true)]
    #[case("TRUE", true)]
    #[case("tRuE", true)]
    #[case("YES", true)]
    #[case("yEs", true)]
    #[case("false", false)]
    #[case("no", false)]
    #[case("0", false)]
    #[case("off", false)]
    #[case("disabled", false)]
    #[case("d", false)]
    #[case("FALSE", false)]
    #[case("fAlSe", false)]
    #[case("NO", false)]
    #[case("nO", false)]
    fn bool_parsing(#[case] input: &str, #[case] expected: bool) {
        let result = bool::from_env_str(input);
        assert_eq!(result.ok(), Some(expected));
    }

    #[rstest]
    #[case("banana")]
    #[case("maybe")]
    #[case("")]
    fn bool_parsing_invalid(#[case] input: &str) {
        assert!(bool::from_env_str(input).is_err());
    }

    #[rstest]
    #[case("42", 42i32)]
    #[case("-1", -1i32)]
    #[case("0", 0i32)]
    fn i32_parsing(#[case] input: &str, #[case] expected: i32) {
        assert_eq!(i32::from_env_str(input).ok(), Some(expected));
    }

    #[test]
    fn i32_overflow() {
        assert!(i32::from_env_str("99999999999999").is_err());
    }

    #[test]
    fn i32_non_numeric() {
        assert!(i32::from_env_str("abc").is_err());
    }

    #[rstest]
    #[case("8080", 8080u16)]
    #[case("0", 0u16)]
    fn u16_parsing(#[case] input: &str, #[case] expected: u16) {
        assert_eq!(u16::from_env_str(input).ok(), Some(expected));
    }

    #[test]
    fn u16_overflow() {
        assert!(u16::from_env_str("70000").is_err());
    }

    #[rstest]
    #[case("3.14", 3.14f64)]
    #[case("-0.5", -0.5f64)]
    fn f64_parsing(#[case] input: &str, #[case] expected: f64) {
        let result = f64::from_env_str(input);
        assert!((result.ok().unwrap_or(0.0) - expected).abs() < f64::EPSILON);
    }

    #[test]
    fn string_infallible() {
        assert_eq!(String::from_env_str("anything").ok(), Some("anything".to_owned()));
    }

    #[test]
    fn pathbuf_infallible() {
        assert_eq!(PathBuf::from_env_str("/tmp/foo").ok(), Some(PathBuf::from("/tmp/foo")));
    }

    #[test]
    fn vec_i32_parsing() {
        let result = Vec::<i32>::from_env_str("1,2,3");
        assert_eq!(result.ok(), Some(vec![1, 2, 3]));
    }

    #[test]
    fn vec_i32_with_whitespace() {
        let result = Vec::<i32>::from_env_str("1 , 2 , 3");
        assert_eq!(result.ok(), Some(vec![1, 2, 3]));
    }

    #[test]
    fn vec_string_parsing() {
        let result = Vec::<String>::from_env_str("a,b,c");
        assert_eq!(result.ok(), Some(vec!["a".to_owned(), "b".to_owned(), "c".to_owned()]));
    }

    #[test]
    fn vec_empty_string() {
        let result = Vec::<i32>::from_env_str("");
        assert_eq!(result.ok(), Some(vec![]));
    }

    #[test]
    fn vec_single_element() {
        let result = Vec::<i32>::from_env_str("42");
        assert_eq!(result.ok(), Some(vec![42]));
    }

    #[test]
    fn vec_parse_error_reports_index() {
        let result = Vec::<i32>::from_env_str("1,banana,3");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.index, 1);
        assert!(err.to_string().contains("element 1"));
    }

    #[test]
    fn vec_bool_via_truthful() {
        let result = Vec::<bool>::from_env_str("yes,no,true,false");
        assert_eq!(result.ok(), Some(vec![true, false, true, false]));
    }

    #[cfg(feature = "chrono")]
    mod chrono_tests {
        use rstest::rstest;

        use crate::parse::FromEnvStr;

        #[rstest]
        #[case::iso("2024-03-15T10:30:00")]
        #[case::iso_fractional("2024-03-15T10:30:00.123")]
        #[case::space_separated("2024-03-15 10:30:00")]
        #[case::space_fractional("2024-03-15 10:30:00.500")]
        #[case::minute_only("2024-03-15T10:30")]
        #[case::space_minute("2024-03-15 10:30")]
        #[case::slash_date("2024/03/15 10:30:00")]
        #[case::us_date("03/15/2024 10:30:00")]
        #[case::whitespace_padding("  2024-03-15T10:30:00  ")]
        fn naive_datetime_valid(#[case] input: &str) {
            assert!(chrono::NaiveDateTime::from_env_str(input).is_ok());
        }

        #[rstest]
        #[case::nonsense("banana")]
        #[case::empty("")]
        #[case::date_only("2024-03-15")]
        fn naive_datetime_invalid(#[case] input: &str) {
            assert!(chrono::NaiveDateTime::from_env_str(input).is_err());
        }

        #[test]
        fn naive_datetime_rfc3339_strips_tz() {
            let result = chrono::NaiveDateTime::from_env_str("2024-03-15T10:30:00+05:00");
            let parsed = result.expect("should parse rfc3339");
            assert_eq!(parsed.to_string(), "2024-03-15 05:30:00");
        }

        #[rstest]
        #[case::rfc3339("2024-03-15T10:30:00+00:00")]
        #[case::rfc3339_z("2024-03-15T10:30:00Z")]
        #[case::iso_naive_fallback("2024-03-15T10:30:00")]
        fn datetime_utc_valid(#[case] input: &str) {
            assert!(chrono::DateTime::<chrono::Utc>::from_env_str(input).is_ok());
        }

        #[rstest]
        #[case::nonsense("banana")]
        #[case::empty("")]
        fn datetime_utc_invalid(#[case] input: &str) {
            assert!(chrono::DateTime::<chrono::Utc>::from_env_str(input).is_err());
        }

        #[rstest]
        #[case::iso("2024-03-15")]
        #[case::slash("2024/03/15")]
        #[case::us("03/15/2024")]
        #[case::whitespace_padding("  2024-03-15  ")]
        fn naive_date_valid(#[case] input: &str) {
            assert!(chrono::NaiveDate::from_env_str(input).is_ok());
        }

        #[rstest]
        #[case::nonsense("banana")]
        #[case::empty("")]
        #[case::time_only("10:30:00")]
        fn naive_date_invalid(#[case] input: &str) {
            assert!(chrono::NaiveDate::from_env_str(input).is_err());
        }

        #[rstest]
        #[case::hms("10:30:00")]
        #[case::hm("10:30")]
        #[case::fractional("10:30:00.123456")]
        #[case::whitespace_padding("  10:30:00  ")]
        fn naive_time_valid(#[case] input: &str) {
            assert!(chrono::NaiveTime::from_env_str(input).is_ok());
        }

        #[rstest]
        #[case::nonsense("banana")]
        #[case::empty("")]
        #[case::date("2024-03-15")]
        fn naive_time_invalid(#[case] input: &str) {
            assert!(chrono::NaiveTime::from_env_str(input).is_err());
        }

        #[test]
        fn chrono_parse_error_message() {
            let err = chrono::NaiveDate::from_env_str("nope").unwrap_err();
            assert!(err.to_string().contains("nope"));
            assert!(err.to_string().contains("NaiveDate"));
        }
    }
}
