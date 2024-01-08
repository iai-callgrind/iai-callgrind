use crate::runner::format::format_vertical;
use anyhow::Result;
use colored::Colorize;

use super::logfile_parser::LogfileSummary;

pub struct LogfileSummaryFormatter;

pub struct ToolSummaryFormatter;

impl LogfileSummaryFormatter {
    pub fn print(
        summary: &LogfileSummary,
        verbose: bool,
        is_multiple: bool,
        force_show_body: bool,
    ) -> Result<()> {
        if verbose || is_multiple {
            println!(
                "  {:<18}{}",
                "Command:",
                summary.command.display().to_string().blue().bold()
            );
            println!("  {:<18}{}", "PID:", summary.pid.to_string().bold());

            if let Some(parent_pid) = summary.parent_pid {
                println!("  {:<18}{}", "Parent PID:", parent_pid.to_string().bold());
            }
        }

        if let Some(costs) = &summary.cost_summary {
            print!("{}", format_vertical((None, None), costs.all_diffs())?);
        }

        for field in &summary.fields {
            println!("  {:<18}{}", format!("{}:", field.0), field.1.bold());
        }

        if (force_show_body || verbose || summary.has_errors()) && !summary.details.is_empty() {
            let mut iter = summary.details.iter();
            println!("  {:<18}{}", "Details:", iter.next().unwrap());

            for body_line in iter {
                println!("                    {body_line}");
            }
        }

        if let Some(error_summary) = summary.error_summary.as_ref() {
            println!(
                "  {:<18}{}",
                "Error Summary:",
                format!(
                    "{} errors from {} contexts (suppressed: {} from {})",
                    error_summary.errors,
                    error_summary.contexts,
                    error_summary.supp_errors,
                    error_summary.supp_contexts
                )
                .bold()
            );
        }

        println!(
            "  {:<18}{}",
            "Logfile:",
            summary.log_path.display().to_string().blue().bold()
        );

        Ok(())
    }
}
