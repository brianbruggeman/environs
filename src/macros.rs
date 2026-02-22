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
}
