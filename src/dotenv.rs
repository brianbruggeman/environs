use std::fs;
use std::path::Path;
use std::path::PathBuf;

use crate::error::Error;

const DOTENV_PATH_KEY: &str = "DOTENV_PATH";
const DEFAULT_DOTENV: &str = ".env";

fn resolve_dotenv_path() -> PathBuf {
    std::env::var(DOTENV_PATH_KEY)
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_DOTENV))
}

fn parse_value(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let first = trimmed.as_bytes()[0];
    if first == b'"' || first == b'\'' {
        let quote = first;
        if let Some(end) = trimmed[1..].find(quote as char) {
            return trimmed[1..1 + end].to_owned();
        }
        return trimmed[1..].to_owned();
    }

    // unquoted: strip inline comment
    match trimmed.find('#') {
        Some(pos) => trimmed[..pos].trim_end().to_owned(),
        None => trimmed.to_owned(),
    }
}

fn parse_line(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }

    let stripped = trimmed.strip_prefix("export ").unwrap_or(trimmed);

    let eq_pos = stripped.find('=')?;
    let key = stripped[..eq_pos].trim().to_owned();
    let value = parse_value(&stripped[eq_pos + 1..]);
    Some((key, value))
}

fn apply_entries(path: &Path, override_existing: bool) -> crate::error::Result<()> {
    let content = fs::read_to_string(path).map_err(|source| Error::DotenvLoad { path: path.to_path_buf(), source })?;

    for (line_num, line) in content.lines().enumerate() {
        if let Some((key, value)) = parse_line(line) {
            if key.is_empty() {
                return Err(Error::DotenvParse {
                    path: path.to_path_buf(),
                    line: line_num + 1,
                    message: "empty key".into(),
                });
            }

            if override_existing || std::env::var(&key).is_err() {
                // safety: dotenv loading is inherently global state mutation,
                // callers are expected to invoke this early before spawning threads
                unsafe { std::env::set_var(&key, &value) };
            }
        }
    }

    tracing::debug!(path = %path.display(), "loaded dotenv");
    Ok(())
}

pub fn load() -> crate::error::Result<()> {
    let path = resolve_dotenv_path();
    if !path.exists() {
        tracing::debug!(path = %path.display(), "dotenv file not found, skipping");
        return Ok(());
    }
    load_path(&path)
}

pub fn load_path(path: &Path) -> crate::error::Result<()> {
    apply_entries(path, false)
}

pub fn load_override() -> crate::error::Result<()> {
    let path = resolve_dotenv_path();
    if !path.exists() {
        tracing::debug!(path = %path.display(), "dotenv file not found, skipping");
        return Ok(());
    }
    load_override_path(&path)
}

pub fn load_override_path(path: &Path) -> crate::error::Result<()> {
    apply_entries(path, true)
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    fn write_env_file(dir: &Path, filename: &str, content: &str) -> PathBuf {
        let file_path = dir.join(filename);
        let mut file = fs::File::create(&file_path).unwrap_or_else(|err| panic!("failed to create {}: {err}", file_path.display()));
        file.write_all(content.as_bytes())
            .unwrap_or_else(|err| panic!("failed to write {}: {err}", file_path.display()));
        file_path
    }

    #[test]
    fn load_path_sets_vars() {
        let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("failed to create tempdir: {err}"));
        let env_path = write_env_file(dir.path(), ".env", "TEST_OWN_LOAD=hello\n");

        temp_env::with_vars([("TEST_OWN_LOAD", None::<&str>)], || {
            load_path(&env_path).unwrap_or_else(|err| panic!("load_path failed: {err}"));
            assert_eq!(std::env::var("TEST_OWN_LOAD").ok(), Some("hello".to_owned()));
        });
    }

    #[test]
    fn load_path_does_not_override_existing() {
        let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("failed to create tempdir: {err}"));
        let env_path = write_env_file(dir.path(), ".env", "TEST_OWN_NO_OVR=from_file\n");

        temp_env::with_vars([("TEST_OWN_NO_OVR", Some("existing"))], || {
            load_path(&env_path).unwrap_or_else(|err| panic!("load_path failed: {err}"));
            assert_eq!(std::env::var("TEST_OWN_NO_OVR").ok(), Some("existing".to_owned()));
        });
    }

    #[test]
    fn load_override_path_does_override() {
        let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("failed to create tempdir: {err}"));
        let env_path = write_env_file(dir.path(), ".env", "TEST_OWN_OVR=from_file\n");

        temp_env::with_vars([("TEST_OWN_OVR", Some("existing"))], || {
            load_override_path(&env_path).unwrap_or_else(|err| panic!("load_override_path failed: {err}"));
            assert_eq!(std::env::var("TEST_OWN_OVR").ok(), Some("from_file".to_owned()));
        });
    }

    #[test]
    fn load_path_missing_file_returns_error() {
        let result = load_path(Path::new("/tmp/nonexistent_environs_test/.env"));
        assert!(matches!(result, Err(Error::DotenvLoad { .. })));
    }

    #[test]
    fn load_uses_dotenv_path_env_var() {
        let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("failed to create tempdir: {err}"));
        let env_path = write_env_file(dir.path(), "custom.env", "TEST_OWN_CUSTOM=found\n");

        temp_env::with_vars(
            [("DOTENV_PATH", Some(env_path.to_str().unwrap_or_else(|| panic!("non-utf8 path")))), ("TEST_OWN_CUSTOM", None::<&str>)],
            || {
                load().unwrap_or_else(|err| panic!("load failed: {err}"));
                assert_eq!(std::env::var("TEST_OWN_CUSTOM").ok(), Some("found".to_owned()));
            },
        );
    }

    #[test]
    fn load_skips_when_no_file_exists() {
        temp_env::with_vars([("DOTENV_PATH", Some("/tmp/nonexistent_environs_test/nope.env"))], || {
            assert!(load().is_ok());
        });
    }

    #[test]
    fn load_override_uses_dotenv_path_env_var() {
        let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("failed to create tempdir: {err}"));
        let env_path = write_env_file(dir.path(), "custom.env", "TEST_OWN_OVR_CUSTOM=overridden\n");

        temp_env::with_vars(
            [
                ("DOTENV_PATH", Some(env_path.to_str().unwrap_or_else(|| panic!("non-utf8 path")))),
                ("TEST_OWN_OVR_CUSTOM", Some("original")),
            ],
            || {
                load_override().unwrap_or_else(|err| panic!("load_override failed: {err}"));
                assert_eq!(std::env::var("TEST_OWN_OVR_CUSTOM").ok(), Some("overridden".to_owned()));
            },
        );
    }

    #[test]
    fn load_override_skips_when_no_file_exists() {
        temp_env::with_vars([("DOTENV_PATH", Some("/tmp/nonexistent_environs_test/nope.env"))], || {
            assert!(load_override().is_ok());
        });
    }

    #[test]
    fn skips_comment_lines() {
        let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("failed to create tempdir: {err}"));
        let env_path = write_env_file(dir.path(), ".env", "# this is a comment\nTEST_OWN_COMMENT=value\n");

        temp_env::with_vars([("TEST_OWN_COMMENT", None::<&str>)], || {
            load_path(&env_path).unwrap_or_else(|err| panic!("load_path failed: {err}"));
            assert_eq!(std::env::var("TEST_OWN_COMMENT").ok(), Some("value".to_owned()));
        });
    }

    #[test]
    fn skips_empty_lines() {
        let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("failed to create tempdir: {err}"));
        let env_path = write_env_file(dir.path(), ".env", "\n\nTEST_OWN_EMPTY=value\n\n");

        temp_env::with_vars([("TEST_OWN_EMPTY", None::<&str>)], || {
            load_path(&env_path).unwrap_or_else(|err| panic!("load_path failed: {err}"));
            assert_eq!(std::env::var("TEST_OWN_EMPTY").ok(), Some("value".to_owned()));
        });
    }

    #[test]
    fn strips_inline_comment() {
        let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("failed to create tempdir: {err}"));
        let env_path = write_env_file(dir.path(), ".env", "TEST_OWN_INLINE=value # a comment\n");

        temp_env::with_vars([("TEST_OWN_INLINE", None::<&str>)], || {
            load_path(&env_path).unwrap_or_else(|err| panic!("load_path failed: {err}"));
            assert_eq!(std::env::var("TEST_OWN_INLINE").ok(), Some("value".to_owned()));
        });
    }

    #[test]
    fn double_quoted_preserves_hash_and_spaces() {
        let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("failed to create tempdir: {err}"));
        let env_path = write_env_file(dir.path(), ".env", "TEST_OWN_DQUOTE=\"value # not a comment\"\n");

        temp_env::with_vars([("TEST_OWN_DQUOTE", None::<&str>)], || {
            load_path(&env_path).unwrap_or_else(|err| panic!("load_path failed: {err}"));
            assert_eq!(std::env::var("TEST_OWN_DQUOTE").ok(), Some("value # not a comment".to_owned()));
        });
    }

    #[test]
    fn single_quoted_preserves_hash_and_spaces() {
        let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("failed to create tempdir: {err}"));
        let env_path = write_env_file(dir.path(), ".env", "TEST_OWN_SQUOTE='value # not a comment'\n");

        temp_env::with_vars([("TEST_OWN_SQUOTE", None::<&str>)], || {
            load_path(&env_path).unwrap_or_else(|err| panic!("load_path failed: {err}"));
            assert_eq!(std::env::var("TEST_OWN_SQUOTE").ok(), Some("value # not a comment".to_owned()));
        });
    }

    #[test]
    fn export_prefix_stripped() {
        let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("failed to create tempdir: {err}"));
        let env_path = write_env_file(dir.path(), ".env", "export TEST_OWN_EXPORT=exported\n");

        temp_env::with_vars([("TEST_OWN_EXPORT", None::<&str>)], || {
            load_path(&env_path).unwrap_or_else(|err| panic!("load_path failed: {err}"));
            assert_eq!(std::env::var("TEST_OWN_EXPORT").ok(), Some("exported".to_owned()));
        });
    }

    #[test]
    fn multiple_entries() {
        let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("failed to create tempdir: {err}"));
        let content = "TEST_OWN_MULTI_A=alpha\n# comment\nTEST_OWN_MULTI_B=beta\nexport TEST_OWN_MULTI_C=\"gamma\"\n";
        let env_path = write_env_file(dir.path(), ".env", content);

        temp_env::with_vars([("TEST_OWN_MULTI_A", None::<&str>), ("TEST_OWN_MULTI_B", None::<&str>), ("TEST_OWN_MULTI_C", None::<&str>)], || {
            load_path(&env_path).unwrap_or_else(|err| panic!("load_path failed: {err}"));
            assert_eq!(std::env::var("TEST_OWN_MULTI_A").ok(), Some("alpha".to_owned()));
            assert_eq!(std::env::var("TEST_OWN_MULTI_B").ok(), Some("beta".to_owned()));
            assert_eq!(std::env::var("TEST_OWN_MULTI_C").ok(), Some("gamma".to_owned()));
        });
    }

    #[test]
    fn empty_value() {
        let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("failed to create tempdir: {err}"));
        let env_path = write_env_file(dir.path(), ".env", "TEST_OWN_EMPTY_VAL=\n");

        temp_env::with_vars([("TEST_OWN_EMPTY_VAL", None::<&str>)], || {
            load_path(&env_path).unwrap_or_else(|err| panic!("load_path failed: {err}"));
            assert_eq!(std::env::var("TEST_OWN_EMPTY_VAL").ok(), Some(String::new()));
        });
    }

    #[test]
    fn line_with_no_equals_is_skipped() {
        let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("failed to create tempdir: {err}"));
        let env_path = write_env_file(dir.path(), ".env", "JUST_A_KEY\nTEST_OWN_NOEQ=present\n");

        temp_env::with_vars([("TEST_OWN_NOEQ", None::<&str>)], || {
            load_path(&env_path).unwrap_or_else(|err| panic!("load_path failed: {err}"));
            assert_eq!(std::env::var("TEST_OWN_NOEQ").ok(), Some("present".to_owned()));
            assert!(std::env::var("JUST_A_KEY").is_err());
        });
    }

    #[test]
    fn whitespace_around_equals_is_trimmed() {
        let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("failed to create tempdir: {err}"));
        let env_path = write_env_file(dir.path(), ".env", "TEST_OWN_WS_EQ = spaced_value\n");

        temp_env::with_vars([("TEST_OWN_WS_EQ", None::<&str>)], || {
            load_path(&env_path).unwrap_or_else(|err| panic!("load_path failed: {err}"));
            assert_eq!(std::env::var("TEST_OWN_WS_EQ").ok(), Some("spaced_value".to_owned()));
        });
    }

    #[test]
    fn unclosed_quote_returns_rest_of_value() {
        let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("failed to create tempdir: {err}"));
        let env_path = write_env_file(dir.path(), ".env", "TEST_OWN_UNCLOSED=\"unterminated\n");

        temp_env::with_vars([("TEST_OWN_UNCLOSED", None::<&str>)], || {
            load_path(&env_path).unwrap_or_else(|err| panic!("load_path failed: {err}"));
            assert_eq!(std::env::var("TEST_OWN_UNCLOSED").ok(), Some("unterminated".to_owned()));
        });
    }

    #[test]
    fn whitespace_only_value_produces_empty_string() {
        let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("failed to create tempdir: {err}"));
        let env_path = write_env_file(dir.path(), ".env", "TEST_OWN_WSVAL=   \n");

        temp_env::with_vars([("TEST_OWN_WSVAL", None::<&str>)], || {
            load_path(&env_path).unwrap_or_else(|err| panic!("load_path failed: {err}"));
            assert_eq!(std::env::var("TEST_OWN_WSVAL").ok(), Some(String::new()));
        });
    }

    #[test]
    fn value_with_equals_sign() {
        let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("failed to create tempdir: {err}"));
        let env_path = write_env_file(dir.path(), ".env", "TEST_OWN_EQ=postgres://user:pass@host/db?opt=val\n");

        temp_env::with_vars([("TEST_OWN_EQ", None::<&str>)], || {
            load_path(&env_path).unwrap_or_else(|err| panic!("load_path failed: {err}"));
            assert_eq!(std::env::var("TEST_OWN_EQ").ok(), Some("postgres://user:pass@host/db?opt=val".to_owned()));
        });
    }
}
