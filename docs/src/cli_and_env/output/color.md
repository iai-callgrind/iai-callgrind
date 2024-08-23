# Changing the color output

The terminal output is colored per default but follows the value for the
`IAI_CALLGRIND_COLOR` environment variable. If `IAI_CALLGRIND_COLOR` is not set,
`CARGO_TERM_COLOR` is also tried. Accepted values are:

`always`, `never`, `auto` (default).

So, disabling colors can be achieved with setting `IAI_CALLGRIND_COLOR` or
`CARGO_TERM_COLOR=never`.
