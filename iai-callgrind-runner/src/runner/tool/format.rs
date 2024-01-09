use std::fmt::Display;

use anyhow::Result;
use colored::Colorize;

use super::logfile_parser::LogfileSummary;
use crate::runner::format::format_vertical;

pub struct LogfileSummaryFormatter;

pub struct ToolSummaryFormatter;

fn print_compare<T: Display>(description: &str, old: Option<T>, new: T) {
    match old {
        None => println!("  {description:<18}{}", new.to_string().bold()),
        Some(old) => println!(
            "  {description:<18}{:>15}|{old:<15}",
            new.to_string().bold()
        ),
    }
}

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
            print_compare("PID:", summary.old_pid, summary.pid);

            if let Some(parent_pid) = summary.parent_pid {
                print_compare("Parent PID:", summary.old_parent_pid, parent_pid);
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

        if summary.cost_summary.is_none() || verbose {
            println!(
                "  {:<18}{}",
                "Logfile:",
                summary.log_path.display().to_string().blue().bold()
            );
        }

        Ok(())
    }
}
