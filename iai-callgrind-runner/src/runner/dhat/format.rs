use std::fmt::Write;

use colored::Colorize;

use super::LogfileSummary;

pub struct LogfileSummaryFormatter;

impl LogfileSummaryFormatter {
    pub fn format(summary: &LogfileSummary) -> String {
        let mut string = String::new();
        writeln!(
            string,
            "  {:<18}{}",
            "Command:",
            summary.command.display().to_string().blue().bold()
        )
        .unwrap();

        writeln!(string, "  {:<18}{}", "PID:", summary.pid.to_string().bold()).unwrap();

        for field in &summary.fields {
            writeln!(
                string,
                "  {:<18}{}",
                format!("{}:", field.0),
                field.1.bold()
            )
            .unwrap();
        }
        string
    }
}
