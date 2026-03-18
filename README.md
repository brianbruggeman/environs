# environs

Typed environment variable resolution with fallback key chains and dotenv loading.

Built because every env config crate is either too magic or too verbose.

```sh
cargo add environs
```

## Quick start

```rust
use environs::env;

let port: u16 = env!("APP_PORT", "PORT", default = 8080)?;
let db: String = env!("DATABASE_URL")?;
let debug: Option<bool> = env!("DEBUG")?;
```

## Macro

The `env!` macro injects the caller's source location into errors so you know exactly where a lookup failed.

| Syntax | Behaviour |
|---|---|
| `env!("KEY")` | required; error if missing |
| `env!("KEY1", "KEY2")` | cascade — first set key wins |
| `env!(..., default = val)` | typed fallback value |
| `env!(..., default_str = "…")` | string fallback, parsed at call time |
| `env!(..., default_fn = \|\| expr)` | lazy fallback, not evaluated if key is found |
| `env!(..., resolve_with = \|raw\| …)` | custom parser; bypasses `FromEnvStr` |

## Builder API

For when you want to construct the lookup in steps:

```rust
use environs::Var;

// required
let port: u16 = Var::new(&["APP_PORT", "PORT"]).get()?;

// typed default
let port: u16 = Var::new(&["PORT"]).default(8080u16).get()?;

// string default (parsed at call time)
let port: u16 = Var::new(&["PORT"]).default_str("8080").get()?;

// lazy default
let port: u16 = Var::new(&["PORT"]).default_fn(|| compute_port()).get()?;

// custom parser
let hosts: Vec<String> = Var::new(&["HOSTS"])
    .resolve_with(|raw| Ok::<_, std::convert::Infallible>(
        raw.split(',').map(str::to_owned).collect()
    ))?;
```

## Dotenv

```rust
// convenience functions
environs::load()?;               // load .env (or $DOTENV_PATH); skip if missing; don't override
environs::load_override()?;      // same but overrides existing vars
environs::load_path(&path)?;     // explicit path; fail if missing; don't override
environs::load_override_path(&path)?; // explicit path; fail if missing; override

// builder — chain multiple files, mix optional and required
DotenvLoader::new()
    .path(".env")           // skip if missing
    .path(".env.local")     // skip if missing
    .require(".env.required") // fail if missing
    .override_existing()    // replace vars already in the environment
    .load()?;
```

## Supported types

`bool`, all numeric primitives, `String`, `PathBuf`, `Option<T>`, `Vec<T>` (comma-separated), and `chrono` date/time types (feature `chrono`).

Implement `FromEnvStr` on your own type to hook into the full resolution pipeline including cascades, defaults, and error location.

## Errors

Errors include the source file and line where `env!()` was called:

```
src/config.rs:14: PORT: expected u16, got 'banana': invalid digit found in string
```
