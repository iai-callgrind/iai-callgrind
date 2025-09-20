# Changing the color output

The terminal output is colored per default but follows the value for the
`GUNGRAUN_COLOR` environment variable. If `GUNGRAUN_COLOR` is not set,
`CARGO_TERM_COLOR` is also tried. Accepted values are:

`always`, `never`, `auto` (default).

So, disabling colors can be achieved with setting `GUNGRAUN_COLOR` or
`CARGO_TERM_COLOR=never`.
