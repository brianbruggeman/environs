use std::marker::PhantomData;

use crate::parse::FromEnvStr;
use crate::resolve::{resolve, resolve_or, resolve_or_else, resolve_or_parse, resolve_with};

pub struct Var<'a> {
    keys: Vec<&'a str>,
}

impl<'a> Var<'a> {
    pub fn new(keys: &[&'a str]) -> Self {
        Self { keys: keys.to_vec() }
    }

    pub fn get<T: FromEnvStr>(self) -> crate::Result<T> {
        resolve(&self.keys)
    }

    pub fn default<T: FromEnvStr>(self, val: T) -> VarOr<'a, T> {
        VarOr { keys: self.keys, default: val }
    }

    pub fn default_str(self, s: &'a str) -> VarOrStr<'a> {
        VarOrStr { keys: self.keys, default: s }
    }

    pub fn default_fn<T, F>(self, f: F) -> VarOrElse<'a, T, F>
    where
        F: FnOnce() -> T,
    {
        VarOrElse {
            keys: self.keys,
            default_fn: f,
            _marker: PhantomData,
        }
    }

    pub fn resolve_with<T, E, F>(self, parse_fn: F) -> crate::Result<T>
    where
        E: std::error::Error + Send + Sync + 'static,
        F: FnOnce(&str) -> std::result::Result<T, E>,
    {
        resolve_with(&self.keys, parse_fn)
    }
}

pub struct VarOr<'a, T> {
    keys: Vec<&'a str>,
    default: T,
}

impl<'a, T: FromEnvStr> VarOr<'a, T> {
    pub fn get(self) -> crate::Result<T> {
        resolve_or(&self.keys, self.default)
    }
}

pub struct VarOrStr<'a> {
    keys: Vec<&'a str>,
    default: &'a str,
}

impl<'a> VarOrStr<'a> {
    pub fn get<T: FromEnvStr>(self) -> crate::Result<T> {
        resolve_or_parse(&self.keys, self.default)
    }
}

pub struct VarOrElse<'a, T, F> {
    keys: Vec<&'a str>,
    default_fn: F,
    _marker: PhantomData<T>,
}

impl<'a, T: FromEnvStr, F: FnOnce() -> T> VarOrElse<'a, T, F> {
    pub fn get(self) -> crate::Result<T> {
        resolve_or_else(&self.keys, self.default_fn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_key_get() {
        temp_env::with_vars([("BUILDER_SINGLE", Some("hello"))], || {
            let result: crate::Result<String> = Var::new(&["BUILDER_SINGLE"]).get();
            assert_eq!(result.ok(), Some("hello".to_owned()));
        });
    }

    #[test]
    fn cascade_keys_get() {
        temp_env::with_vars([("BUILDER_CASCADE_A", None::<&str>), ("BUILDER_CASCADE_B", Some("found"))], || {
            let result: crate::Result<String> = Var::new(&["BUILDER_CASCADE_A", "BUILDER_CASCADE_B"]).get();
            assert_eq!(result.ok(), Some("found".to_owned()));
        });
    }

    #[test]
    fn get_not_found_is_err() {
        temp_env::with_vars([("BUILDER_MISS", None::<&str>)], || {
            let result = Var::new(&["BUILDER_MISS"]).get::<String>();
            assert!(result.is_err());
        });
    }

    #[test]
    fn get_parse_error() {
        temp_env::with_vars([("BUILDER_GET_BAD", Some("banana"))], || {
            let result = Var::new(&["BUILDER_GET_BAD"]).get::<i32>();
            assert!(result.is_err());
        });
    }

    #[test]
    fn default_value_used_when_missing() {
        temp_env::with_vars([("BUILDER_DEF_MISS", None::<&str>)], || {
            let result = Var::new(&["BUILDER_DEF_MISS"]).default(9090u16).get();
            assert_eq!(result.ok(), Some(9090));
        });
    }

    #[test]
    fn default_value_skipped_when_present() {
        temp_env::with_vars([("BUILDER_DEF_HIT", Some("3000"))], || {
            let result = Var::new(&["BUILDER_DEF_HIT"]).default(9090u16).get();
            assert_eq!(result.ok(), Some(3000));
        });
    }

    #[test]
    fn default_value_propagates_parse_error() {
        temp_env::with_vars([("BUILDER_DEF_BAD", Some("banana"))], || {
            let result = Var::new(&["BUILDER_DEF_BAD"]).default(9090u16).get();
            assert!(result.is_err());
        });
    }

    #[test]
    fn default_str_fallback() {
        temp_env::with_vars([("BUILDER_DSTR_MISS", None::<&str>)], || {
            let result = Var::new(&["BUILDER_DSTR_MISS"]).default_str("8080").get::<u16>();
            assert_eq!(result.ok(), Some(8080));
        });
    }

    #[test]
    fn default_str_uses_env_when_present() {
        temp_env::with_vars([("BUILDER_DSTR_HIT", Some("5000"))], || {
            let result = Var::new(&["BUILDER_DSTR_HIT"]).default_str("8080").get::<u16>();
            assert_eq!(result.ok(), Some(5000));
        });
    }

    #[test]
    fn default_str_bad_default_returns_err() {
        temp_env::with_vars([("BUILDER_DSTR_BAD_DEF", None::<&str>)], || {
            let result = Var::new(&["BUILDER_DSTR_BAD_DEF"]).default_str("banana").get::<u16>();
            assert!(result.is_err());
        });
    }

    #[test]
    fn default_str_env_parse_error() {
        temp_env::with_vars([("BUILDER_DSTR_ENV_BAD", Some("banana"))], || {
            let result = Var::new(&["BUILDER_DSTR_ENV_BAD"]).default_str("8080").get::<u16>();
            assert!(result.is_err());
        });
    }

    #[test]
    fn default_fn_fallback() {
        temp_env::with_vars([("BUILDER_DFN_MISS", None::<&str>)], || {
            let result = Var::new(&["BUILDER_DFN_MISS"]).default_fn(|| 42i32).get();
            assert_eq!(result.ok(), Some(42));
        });
    }

    #[test]
    fn default_fn_not_called_when_present() {
        temp_env::with_vars([("BUILDER_DFN_HIT", Some("7"))], || {
            let result: crate::Result<i32> = Var::new(&["BUILDER_DFN_HIT"])
                .default_fn(|| panic!("should not be called"))
                .get();
            assert_eq!(result.ok(), Some(7));
        });
    }

    #[test]
    fn default_fn_propagates_parse_error() {
        temp_env::with_vars([("BUILDER_DFN_BAD", Some("banana"))], || {
            let result = Var::new(&["BUILDER_DFN_BAD"]).default_fn(|| 42i32).get();
            assert!(result.is_err());
        });
    }

    #[test]
    fn resolve_with_custom_parser() {
        temp_env::with_vars([("BUILDER_RW", Some("a:b:c"))], || {
            let result = Var::new(&["BUILDER_RW"]).resolve_with(|raw| -> std::result::Result<Vec<String>, std::convert::Infallible> { Ok(raw.split(':').map(str::to_owned).collect()) });
            assert_eq!(result.ok(), Some(vec!["a".to_owned(), "b".to_owned(), "c".to_owned()]));
        });
    }

    #[test]
    fn resolve_with_parse_error() {
        temp_env::with_vars([("BUILDER_RW_BAD", Some("notanumber"))], || {
            let result = Var::new(&["BUILDER_RW_BAD"]).resolve_with(|raw| raw.parse::<i32>());
            assert!(result.is_err());
        });
    }

    #[test]
    fn resolve_with_not_found() {
        temp_env::with_vars([("BUILDER_RW_MISS", None::<&str>)], || {
            let result = Var::new(&["BUILDER_RW_MISS"]).resolve_with(|raw| -> std::result::Result<String, std::convert::Infallible> { Ok(raw.to_owned()) });
            assert!(result.is_err());
        });
    }

    #[test]
    fn option_type_none_when_missing() {
        temp_env::with_vars([("BUILDER_OPT_MISS", None::<&str>)], || {
            let result = Var::new(&["BUILDER_OPT_MISS"]).get::<Option<String>>();
            assert_eq!(result.ok(), Some(None));
        });
    }

    #[test]
    fn option_type_some_when_present() {
        temp_env::with_vars([("BUILDER_OPT_HIT", Some("yes"))], || {
            let result = Var::new(&["BUILDER_OPT_HIT"]).get::<Option<bool>>();
            assert_eq!(result.ok(), Some(Some(true)));
        });
    }

    #[test]
    fn option_parse_error() {
        temp_env::with_vars([("BUILDER_OPT_BAD", Some("banana"))], || {
            let result = Var::new(&["BUILDER_OPT_BAD"]).get::<Option<u16>>();
            assert!(result.is_err());
        });
    }
}
