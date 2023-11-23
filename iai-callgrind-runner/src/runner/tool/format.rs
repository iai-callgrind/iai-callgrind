use colored::Colorize;

use super::logfile_parser::LogfileSummary;

pub struct LogfileSummaryFormatter;

impl LogfileSummaryFormatter {
    pub fn print(summary: &LogfileSummary) {
        println!(
            "  {:<18}{}",
            "Command:",
            summary.command.display().to_string().blue().bold()
        );

        println!("  {:<18}{}", "PID:", summary.pid.to_string().bold());

        for field in &summary.fields {
            println!("  {:<18}{}", format!("{}:", field.0), field.1.bold());
        }

        if !summary.body.is_empty() {
            let mut iter = summary.body.iter();
            println!("  {:<18}{}", "Summary:", iter.next().unwrap());

            for body_line in iter {
                println!("                    {body_line}");
            }
        }

        if let Some(error_summary) = summary.error_summary.as_ref() {
            println!("  {:<18}{}", "Error Summary:", error_summary.bold());
        }

        println!(
            "  {:<18}{}",
            "Logfile:",
            summary.log_path.display().to_string().blue().bold()
        );
    }
}
