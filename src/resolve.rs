use std::env;

use crate::error::Error;
use crate::error::Location;
use crate::parse::FromEnvStr;

pub fn resolve<T: FromEnvStr>(keys: &[&str]) -> crate::error::Result<T> {
    for key in keys {
        if let Ok(raw) = env::var(key) {
            return T::from_env_str(&raw).map_err(|source| Error::Parse {
                key: (*key).to_owned(),
                expected: T::type_name(),
                got: raw,
                source: Box::new(source),
                location: Location::default(),
            });
        }
    }
    T::on_not_found(keys)
}

pub fn resolve_or<T: FromEnvStr>(keys: &[&str], default: T) -> crate::error::Result<T> {
    match resolve::<T>(keys) {
        Ok(val) => Ok(val),
        Err(Error::NotFound { .. }) => Ok(default),
        Err(err) => Err(err),
    }
}

pub fn resolve_or_parse<T: FromEnvStr>(keys: &[&str], default_str: &str) -> crate::error::Result<T> {
    match resolve::<T>(keys) {
        Ok(val) => Ok(val),
        Err(Error::NotFound { .. }) => T::from_env_str(default_str).map_err(|source| Error::Parse {
            key: "<default>".to_owned(),
            expected: T::type_name(),
            got: default_str.to_owned(),
            source: Box::new(source),
            location: Location::default(),
        }),
        Err(err) => Err(err),
    }
}

pub fn resolve_or_else<T: FromEnvStr>(keys: &[&str], default_fn: impl FnOnce() -> T) -> crate::error::Result<T> {
    match resolve::<T>(keys) {
        Ok(val) => Ok(val),
        Err(Error::NotFound { .. }) => Ok(default_fn()),
        Err(err) => Err(err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_key_found() {
        temp_env::with_vars([("TEST_RESOLVE_A", Some("42"))], || {
            let result: i32 = resolve(&["TEST_RESOLVE_A"]).unwrap_or(-1);
            assert_eq!(result, 42);
        });
    }

    #[test]
    fn cascade_first_hit() {
        temp_env::with_vars([("TEST_CASCADE_A", Some("hello")), ("TEST_CASCADE_B", Some("world"))], || {
            let result: String = resolve(&["TEST_CASCADE_A", "TEST_CASCADE_B"]).unwrap_or_default();
            assert_eq!(result, "hello");
        });
    }

    #[test]
    fn cascade_falls_through() {
        temp_env::with_vars([("TEST_CASCADE_C", None::<&str>), ("TEST_CASCADE_D", Some("found"))], || {
            let result: String = resolve(&["TEST_CASCADE_C", "TEST_CASCADE_D"]).unwrap_or_default();
            assert_eq!(result, "found");
        });
    }

    #[test]
    fn all_missing_returns_not_found() {
        temp_env::with_vars([("TEST_MISSING_A", None::<&str>), ("TEST_MISSING_B", None::<&str>)], || {
            let result = resolve::<String>(&["TEST_MISSING_A", "TEST_MISSING_B"]);
            assert!(matches!(result, Err(Error::NotFound { .. })));
        });
    }

    #[test]
    fn unparseable_returns_parse_error() {
        temp_env::with_vars([("TEST_PARSE_FAIL", Some("banana"))], || {
            let result = resolve::<i32>(&["TEST_PARSE_FAIL"]);
            assert!(matches!(result, Err(Error::Parse { .. })));
        });
    }

    #[test]
    fn option_none_when_missing() {
        temp_env::with_vars([("TEST_OPT_MISS", None::<&str>)], || {
            let result = resolve::<Option<String>>(&["TEST_OPT_MISS"]);
            assert_eq!(result.ok(), Some(None));
        });
    }

    #[test]
    fn option_some_when_present() {
        temp_env::with_vars([("TEST_OPT_HIT", Some("value"))], || {
            let result = resolve::<Option<String>>(&["TEST_OPT_HIT"]);
            assert_eq!(result.ok(), Some(Some("value".to_owned())));
        });
    }

    #[test]
    fn option_propagates_parse_error() {
        temp_env::with_vars([("TEST_OPT_BAD", Some("notanumber"))], || {
            let result = resolve::<Option<u16>>(&["TEST_OPT_BAD"]);
            assert!(matches!(result, Err(Error::Parse { .. })));
        });
    }

    #[test]
    fn default_fallback_when_missing() {
        temp_env::with_vars([("TEST_DEF_MISS", None::<&str>)], || {
            let result = resolve_or::<i32>(&["TEST_DEF_MISS"], 99);
            assert_eq!(result.ok(), Some(99));
        });
    }

    #[test]
    fn default_uses_value_when_present() {
        temp_env::with_vars([("TEST_DEF_HIT", Some("7"))], || {
            let result = resolve_or::<i32>(&["TEST_DEF_HIT"], 99);
            assert_eq!(result.ok(), Some(7));
        });
    }

    #[test]
    fn default_propagates_parse_error() {
        temp_env::with_vars([("TEST_DEF_BAD", Some("xyz"))], || {
            let result = resolve_or::<i32>(&["TEST_DEF_BAD"], 99);
            assert!(matches!(result, Err(Error::Parse { .. })));
        });
    }

    #[test]
    fn bool_via_truthful() {
        temp_env::with_vars([("TEST_BOOL_YES", Some("yes"))], || {
            let result: bool = resolve(&["TEST_BOOL_YES"]).unwrap_or(false);
            assert!(result);
        });
    }

    #[test]
    fn bool_invalid_is_parse_error() {
        temp_env::with_vars([("TEST_BOOL_BAD", Some("maybe"))], || {
            let result = resolve::<bool>(&["TEST_BOOL_BAD"]);
            assert!(matches!(result, Err(Error::Parse { .. })));
        });
    }

    #[test]
    fn or_parse_fallback_when_missing() {
        temp_env::with_vars([("TEST_ORP_MISS", None::<&str>)], || {
            let result = resolve_or_parse::<u16>(&["TEST_ORP_MISS"], "8080");
            assert_eq!(result.ok(), Some(8080));
        });
    }

    #[test]
    fn or_parse_uses_env_when_present() {
        temp_env::with_vars([("TEST_ORP_HIT", Some("3000"))], || {
            let result = resolve_or_parse::<u16>(&["TEST_ORP_HIT"], "8080");
            assert_eq!(result.ok(), Some(3000));
        });
    }

    #[test]
    fn or_parse_bad_default_returns_parse_error() {
        temp_env::with_vars([("TEST_ORP_BAD_DEF", None::<&str>)], || {
            let result = resolve_or_parse::<u16>(&["TEST_ORP_BAD_DEF"], "not_a_number");
            assert!(matches!(result, Err(Error::Parse { .. })));
        });
    }

    #[test]
    fn or_parse_propagates_env_parse_error() {
        temp_env::with_vars([("TEST_ORP_ENV_BAD", Some("xyz"))], || {
            let result = resolve_or_parse::<u16>(&["TEST_ORP_ENV_BAD"], "8080");
            assert!(matches!(result, Err(Error::Parse { .. })));
        });
    }

    #[test]
    fn or_else_fallback_when_missing() {
        temp_env::with_vars([("TEST_ORE_MISS", None::<&str>)], || {
            let result = resolve_or_else::<i32>(&["TEST_ORE_MISS"], || 42);
            assert_eq!(result.ok(), Some(42));
        });
    }

    #[test]
    fn or_else_uses_env_when_present() {
        temp_env::with_vars([("TEST_ORE_HIT", Some("7"))], || {
            let result = resolve_or_else::<i32>(&["TEST_ORE_HIT"], || 42);
            assert_eq!(result.ok(), Some(7));
        });
    }

    #[test]
    fn or_else_propagates_parse_error() {
        temp_env::with_vars([("TEST_ORE_BAD", Some("xyz"))], || {
            let result = resolve_or_else::<i32>(&["TEST_ORE_BAD"], || 42);
            assert!(matches!(result, Err(Error::Parse { .. })));
        });
    }

    #[test]
    fn or_else_closure_not_called_when_present() {
        temp_env::with_vars([("TEST_ORE_LAZY", Some("10"))], || {
            let result = resolve_or_else::<i32>(&["TEST_ORE_LAZY"], || panic!("should not be called"));
            assert_eq!(result.ok(), Some(10));
        });
    }

    #[test]
    fn or_else_accepts_fn_pointer() {
        fn default_port() -> u16 {
            8080
        }
        temp_env::with_vars([("TEST_ORE_FN", None::<&str>)], || {
            let result = resolve_or_else::<u16>(&["TEST_ORE_FN"], default_port);
            assert_eq!(result.ok(), Some(8080));
        });
    }
}
