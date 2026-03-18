#[macro_export]
macro_rules! env {
    ($($key:literal),+ , default_fn = $default:expr) => {
        $crate::resolve_or_else(&[$($key),+], $default)
            .map_err(|err| err.with_location(file!(), line!()))
    };
    ($($key:literal),+ , default_str = $default:expr) => {
        $crate::resolve_or_parse(&[$($key),+], $default)
            .map_err(|err| err.with_location(file!(), line!()))
    };
    ($($key:literal),+ , default = $default:expr) => {
        $crate::resolve_or(&[$($key),+], $default)
            .map_err(|err| err.with_location(file!(), line!()))
    };
    ($($key:literal),+ , resolve_with = $parse_fn:expr) => {
        $crate::resolve_with(&[$($key),+], $parse_fn)
            .map_err(|err| err.with_location(file!(), line!()))
    };
    ($($key:literal),+) => {
        $crate::resolve(&[$($key),+])
            .map_err(|err| err.with_location(file!(), line!()))
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn single_key() {
        temp_env::with_vars([("TEST_MACRO_SINGLE", Some("hello"))], || {
            let result: crate::Result<String> = env!("TEST_MACRO_SINGLE");
            assert_eq!(result.ok(), Some("hello".to_owned()));
        });
    }

    #[test]
    fn cascade_keys() {
        temp_env::with_vars([("TEST_MACRO_A", None::<&str>), ("TEST_MACRO_B", Some("found"))], || {
            let result: crate::Result<String> = env!("TEST_MACRO_A", "TEST_MACRO_B");
            assert_eq!(result.ok(), Some("found".to_owned()));
        });
    }

    #[test]
    fn single_with_default() {
        temp_env::with_vars([("TEST_MACRO_DEF", None::<&str>)], || {
            let result: crate::Result<i32> = env!("TEST_MACRO_DEF", default = 42);
            assert_eq!(result.ok(), Some(42));
        });
    }

    #[test]
    fn cascade_with_default() {
        temp_env::with_vars([("TEST_MACRO_DEF_A", None::<&str>), ("TEST_MACRO_DEF_B", None::<&str>)], || {
            let result: crate::Result<u16> = env!("TEST_MACRO_DEF_A", "TEST_MACRO_DEF_B", default = 8080);
            assert_eq!(result.ok(), Some(8080));
        });
    }

    #[test]
    fn single_option_none() {
        temp_env::with_vars([("TEST_MACRO_OPT", None::<&str>)], || {
            let result: crate::Result<Option<String>> = env!("TEST_MACRO_OPT");
            assert_eq!(result.ok(), Some(None));
        });
    }

    #[test]
    fn single_option_some() {
        temp_env::with_vars([("TEST_MACRO_OPT_SOME", Some("hello"))], || {
            let result: crate::Result<Option<String>> = env!("TEST_MACRO_OPT_SOME");
            assert_eq!(result.ok(), Some(Some("hello".to_owned())));
        });
    }

    #[test]
    fn cascade_option() {
        temp_env::with_vars([("TEST_MACRO_OPT_A", None::<&str>), ("TEST_MACRO_OPT_B", Some("yes"))], || {
            let result: crate::Result<Option<bool>> = env!("TEST_MACRO_OPT_A", "TEST_MACRO_OPT_B");
            assert_eq!(result.ok(), Some(Some(true)));
        });
    }

    #[test]
    fn cascade_option_all_missing() {
        temp_env::with_vars([("TEST_MACRO_OPT_C", None::<&str>), ("TEST_MACRO_OPT_D", None::<&str>)], || {
            let result: crate::Result<Option<bool>> = env!("TEST_MACRO_OPT_C", "TEST_MACRO_OPT_D");
            assert_eq!(result.ok(), Some(None));
        });
    }

    #[test]
    fn error_carries_source_location() {
        temp_env::with_vars([("TEST_MACRO_LOC", None::<&str>)], || {
            let result: crate::Result<String> = env!("TEST_MACRO_LOC");
            let err = result.unwrap_err();
            let msg = err.to_string();
            assert!(msg.contains("macros.rs:"), "error should contain source file: {msg}");
            assert!(msg.contains("TEST_MACRO_LOC"), "error should contain key name: {msg}");
        });
    }

    #[test]
    fn parse_error_carries_source_location() {
        temp_env::with_vars([("TEST_MACRO_PARSE_LOC", Some("banana"))], || {
            let result: crate::Result<i32> = env!("TEST_MACRO_PARSE_LOC");
            let err = result.unwrap_err();
            let msg = err.to_string();
            assert!(msg.contains("macros.rs:"), "error should contain source file: {msg}");
            assert!(msg.contains("TEST_MACRO_PARSE_LOC"), "error should contain key name: {msg}");
            assert!(msg.contains("i32"), "error should contain expected type: {msg}");
            assert!(msg.contains("banana"), "error should contain raw value: {msg}");
        });
    }

    #[test]
    fn default_str_fallback() {
        temp_env::with_vars([("TEST_MACRO_DSTR", None::<&str>)], || {
            let result: crate::Result<u16> = env!("TEST_MACRO_DSTR", default_str = "8080");
            assert_eq!(result.ok(), Some(8080));
        });
    }

    #[test]
    fn default_str_uses_env_when_present() {
        temp_env::with_vars([("TEST_MACRO_DSTR_HIT", Some("3000"))], || {
            let result: crate::Result<u16> = env!("TEST_MACRO_DSTR_HIT", default_str = "8080");
            assert_eq!(result.ok(), Some(3000));
        });
    }

    #[test]
    fn default_str_cascade() {
        temp_env::with_vars([("TEST_MACRO_DSTR_A", None::<&str>), ("TEST_MACRO_DSTR_B", None::<&str>)], || {
            let result: crate::Result<bool> = env!("TEST_MACRO_DSTR_A", "TEST_MACRO_DSTR_B", default_str = "yes");
            assert_eq!(result.ok(), Some(true));
        });
    }

    #[test]
    fn default_fn_fallback_closure() {
        temp_env::with_vars([("TEST_MACRO_DFN", None::<&str>)], || {
            let result: crate::Result<i32> = env!("TEST_MACRO_DFN", default_fn = || 42);
            assert_eq!(result.ok(), Some(42));
        });
    }

    #[test]
    fn default_fn_uses_env_when_present() {
        temp_env::with_vars([("TEST_MACRO_DFN_HIT", Some("7"))], || {
            let result: crate::Result<i32> = env!("TEST_MACRO_DFN_HIT", default_fn = || 42);
            assert_eq!(result.ok(), Some(7));
        });
    }

    #[test]
    fn default_fn_accepts_named_function() {
        fn fallback_port() -> u16 {
            9090
        }
        temp_env::with_vars([("TEST_MACRO_DFN_NAMED", None::<&str>)], || {
            let result: crate::Result<u16> = env!("TEST_MACRO_DFN_NAMED", default_fn = fallback_port);
            assert_eq!(result.ok(), Some(9090));
        });
    }

    #[test]
    fn default_fn_cascade() {
        temp_env::with_vars([("TEST_MACRO_DFN_A", None::<&str>), ("TEST_MACRO_DFN_B", None::<&str>)], || {
            let result: crate::Result<String> = env!("TEST_MACRO_DFN_A", "TEST_MACRO_DFN_B", default_fn = || "fallback".to_owned());
            assert_eq!(result.ok(), Some("fallback".to_owned()));
        });
    }

    #[test]
    fn default_fn_not_called_when_present() {
        temp_env::with_vars([("TEST_MACRO_DFN_LAZY", Some("10"))], || {
            let result: crate::Result<i32> = env!("TEST_MACRO_DFN_LAZY", default_fn = || panic!("should not be called"));
            assert_eq!(result.ok(), Some(10));
        });
    }

    #[test]
    fn resolve_with_custom_parser() {
        temp_env::with_vars([("TEST_MACRO_PFN", Some("a,b,c"))], || {
            let result = env!(
                "TEST_MACRO_PFN",
                resolve_with = |raw: &str| -> std::result::Result<Vec<String>, std::convert::Infallible> { Ok(raw.split(',').map(str::to_owned).collect()) }
            );
            assert_eq!(result.ok(), Some(vec!["a".to_owned(), "b".to_owned(), "c".to_owned()]));
        });
    }

    #[test]
    fn resolve_with_not_found_error() {
        temp_env::with_vars([("TEST_MACRO_PFN_MISS", None::<&str>)], || {
            let result = env!(
                "TEST_MACRO_PFN_MISS",
                resolve_with = |raw: &str| -> std::result::Result<String, std::convert::Infallible> { Ok(raw.to_owned()) }
            );
            assert!(result.is_err());
        });
    }

    #[test]
    fn resolve_with_carries_source_location() {
        temp_env::with_vars([("TEST_MACRO_PFN_LOC", None::<&str>)], || {
            let result = env!(
                "TEST_MACRO_PFN_LOC",
                resolve_with = |raw: &str| -> std::result::Result<String, std::convert::Infallible> { Ok(raw.to_owned()) }
            );
            let err = result.unwrap_err();
            let msg = err.to_string();
            assert!(msg.contains("macros.rs:"), "error should contain source file: {msg}");
        });
    }

    #[test]
    fn default_parse_error_carries_location() {
        temp_env::with_vars([("TEST_MACRO_DEF_PERR", Some("banana"))], || {
            let result: crate::Result<i32> = env!("TEST_MACRO_DEF_PERR", default = 42);
            let err = result.unwrap_err();
            let msg = err.to_string();
            assert!(msg.contains("macros.rs:"), "error should have source location: {msg}");
        });
    }

    #[test]
    fn default_str_parse_error_carries_location() {
        temp_env::with_vars([("TEST_MACRO_DSTR_PERR", Some("banana"))], || {
            let result: crate::Result<i32> = env!("TEST_MACRO_DSTR_PERR", default_str = "42");
            let err = result.unwrap_err();
            let msg = err.to_string();
            assert!(msg.contains("macros.rs:"), "error should have source location: {msg}");
        });
    }

    #[test]
    fn default_fn_parse_error_carries_location() {
        temp_env::with_vars([("TEST_MACRO_DFN_PERR", Some("banana"))], || {
            let result: crate::Result<i32> = env!("TEST_MACRO_DFN_PERR", default_fn = || 42i32);
            let err = result.unwrap_err();
            let msg = err.to_string();
            assert!(msg.contains("macros.rs:"), "error should have source location: {msg}");
        });
    }

    #[test]
    fn resolve_with_cascade() {
        temp_env::with_vars([("TEST_MACRO_PFN_CASCADE_A", None::<&str>), ("TEST_MACRO_PFN_CASCADE_B", Some("99"))], || {
            let result = env!("TEST_MACRO_PFN_CASCADE_A", "TEST_MACRO_PFN_CASCADE_B", resolve_with = |raw: &str| raw.parse::<i32>());
            assert_eq!(result.ok(), Some(99));
        });
    }
}
