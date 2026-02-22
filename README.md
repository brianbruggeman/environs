# environs

Typed environment variable resolution with fallback key chains and dotenv loading.

Built because every env config crate is either too magic or too verbose.

```rust
use environs::env;

let port: u16 = env!("APP_PORT", "PORT", default = 8080)?;
let db: String = env!("DATABASE_URL")?;
let debug: Option<bool> = env!("DEBUG")?;
```

Dotenv loading checks `DOTENV_PATH` or falls back to `.env`:

```rust
environs::load()?;
```

Errors include the source file and line where `env!()` was called.

```sh
cargo add environs
```
