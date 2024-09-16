# Changing the logging output

Iai-Callgrind uses [env_logger](https://crates.io/crates/env_logger) and the
default logging level `WARN`. To set the logging level to something different,
set the environment variable `IAI_CALLGRIND_LOG` for example to
`IAI_CALLGRIND_LOG=DEBUG`. Accepted values are:

`error`, `warn` (default), `info`, `debug`, `trace`.

The logging output is colored per default but follows the [Color
settings](./color.md).

See also the [documentation](https://docs.rs/env_logger/latest) of `env_logger`.