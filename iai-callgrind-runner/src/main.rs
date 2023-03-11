use env_logger::Env;

fn main() {
    // Configure the env_logger crate to respect CARGO_TERM_COLOR
    env_logger::Builder::from_env(
        Env::default()
            .default_filter_or("warn")
            .write_style("CARGO_TERM_COLOR"),
    )
    .format_timestamp(None)
    .init();

    // Configure the colored crate to respect CARGO_TERM_COLOR
    if let Some(var) = option_env!("CARGO_TERM_COLOR") {
        if var == "never" {
            colored::control::set_override(false);
        } else if var == "always" {
            colored::control::set_override(true);
        }
    }

    iai_callgrind_runner::run();
}
