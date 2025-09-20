# Changing the logging output

Gungraun uses [env_logger](https://docs.rs/env_logger/latest/env_logger/) and the
default logging level `WARN`. To set the logging level to something different,
set the environment variable `GUNGRAUN_LOG` for example to
`GUNGRAUN_LOG=DEBUG`. Accepted values are:

`error`, `warn` (default), `info`, `debug`, `trace`.

The logging output is colored per default but follows the [Color
settings](./color.md).

See also the [documentation](https://docs.rs/env_logger/latest/env_logger/) of `env_logger`.
