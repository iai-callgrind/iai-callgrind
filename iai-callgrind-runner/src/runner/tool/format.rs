use std::fmt::Display;

use anyhow::Result;
use colored::Colorize;

use crate::runner::format::{format_vertical, NOT_AVAILABLE};
use crate::runner::summary::{CostsSummaryType, ToolRunSummary};

pub struct ToolRunSummaryFormatter;

pub struct ToolSummaryFormatter;

fn print_compare<T: Display>(
    description: &str,
    old: Option<T>,
    new: Option<T>,
    should_compare: bool,
) {
    match (new, old) {
        (Some(new), _) if !should_compare => {
            println!("  {description:<18}{}", new.to_string().bold());
        }
        (None, Some(old)) => println!("  {description:<18}{:>15}|{old:<15}", NOT_AVAILABLE.bold(),),
        (Some(new), None) => println!(
            "  {description:<18}{:>15}|{NOT_AVAILABLE:<15}",
            new.to_string().bold(),
        ),
        (Some(new), Some(old)) => println!(
            "  {description:<18}{:>15}|{old:<15}",
            new.to_string().bold(),
        ),
        _ => {}
    }
}

impl ToolRunSummaryFormatter {
    pub fn print(
        summary: &ToolRunSummary,
        verbose: bool,
        is_multiple: bool,
        force_show_body: bool,
    ) -> Result<()> {
        if verbose || is_multiple {
            println!("  {:<18}{}", "Command:", summary.command.blue().bold());
            let should_compare = summary.costs_summary.is_some();
            print_compare("PID:", summary.old_pid, summary.pid, should_compare);
            print_compare(
                "Parent PID:",
                summary.old_parent_pid,
                summary.parent_pid,
                should_compare,
            );
        }

        // The callgrind summary was already printed
        match &summary.costs_summary {
            CostsSummaryType::None | CostsSummaryType::CallgrindSummary(_) => {}
            CostsSummaryType::ErrorSummary(costs) => {
                print!("{}", format_vertical((None, None), costs.all_diffs())?);
            }
            CostsSummaryType::DhatSummary(costs) => {
                print!("{}", format_vertical((None, None), costs.all_diffs())?);
            }
        }

        for field in &summary.summary {
            println!("  {:<18}{}", format!("{}:", field.0), field.1.bold());
        }

        if force_show_body || verbose || summary.has_errors() {
            let mut details = summary.details.iter().flat_map(|x| x.lines());
            if let Some(head_line) = details.next() {
                println!("  {:<18}{}", "Details:", head_line);
                for body_line in details {
                    println!("                    {body_line}");
                }
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

        if summary.costs_summary.is_none() || verbose {
            println!(
                "  {:<18}{}",
                "Logfile:",
                summary.log_path.display().to_string().blue().bold()
            );
        }

        Ok(())
    }
}
