use std::fmt::Display;

use anyhow::Result;
use colored::Colorize;

use crate::runner::format::{format_vertical, NOT_AVAILABLE};
use crate::runner::summary::{CostsSummaryType, ToolRunSummary};
use crate::util::EitherOrBoth;

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

fn print_split<T, U>(name: U, left: T, right: T)
where
    T: Display,
    U: Display,
{
    println!("  {name:<18}{left}");
    println!("  {}|{right}", " ".repeat(33));
}

// TODO: TRY to use only VerticalFormat
impl ToolRunSummaryFormatter {
    pub fn print(
        summary: &ToolRunSummary,
        verbose: bool,
        is_multiple: bool,
        force_show_body: bool,
    ) -> Result<()> {
        if summary.costs_summary.is_none() || verbose || is_multiple {
            match &summary.info {
                EitherOrBoth::Left(new) => {
                    println!("  {:<18}{}", "Command:", new.command.blue().bold());
                }
                EitherOrBoth::Right(old) => {
                    println!(
                        "  {:<18}{}|{}",
                        "Command:",
                        " ".repeat(15),
                        old.command.blue()
                    );
                }
                EitherOrBoth::Both((new, old)) => {
                    print_split("Command:", new.command.blue().bold(), old.command.blue());
                }
            }
            // TODO: CLEANUP
            // let should_compare = summary.costs_summary.is_some();
            // print_compare("PID:", summary.old_pid, summary.pid, should_compare);
            // print_compare(
            //     "Parent PID:",
            //     summary.old_parent_pid,
            //     summary.parent_pid,
            //     should_compare,
            // );
        }

        match &summary.costs_summary {
            CostsSummaryType::None => {}
            CostsSummaryType::ErrorSummary(costs) => {
                print!("{}", format_vertical((None, None), costs.all_diffs())?);
            }
            CostsSummaryType::DhatSummary(costs) => {
                print!("{}", format_vertical((None, None), costs.all_diffs())?);
            }
        }

        if force_show_body || verbose || summary.new_has_errors() {
            match &summary.info {
                EitherOrBoth::Left(new) | EitherOrBoth::Both((new, _)) => {
                    let mut details = new.details.iter().flat_map(|x| x.lines());
                    if let Some(head_line) = details.next() {
                        println!("  {:<18}{}", "Details:", head_line);
                        for body_line in details {
                            println!("                    {body_line}");
                        }
                    }
                }
                EitherOrBoth::Right(_) => {}
            }
        }

        // TODO: CLEANUP
        // if summary.costs_summary.is_none() || verbose {
        if verbose {
            match &summary.info {
                EitherOrBoth::Left(new) => {
                    println!(
                        "  {:<18}{}",
                        "Logfile:",
                        new.path.display().to_string().blue().bold()
                    );
                }
                EitherOrBoth::Right(old) => {
                    println!(
                        "  {:<18}{}|{}",
                        "Logfile:",
                        " ".repeat(15),
                        old.path.display().to_string().blue().bold()
                    );
                }
                EitherOrBoth::Both((new, old)) => {
                    print_split(
                        "Logfile:",
                        new.path.display().to_string().blue().bold(),
                        old.path.display().to_string().blue(),
                    );
                }
            }
        }

        Ok(())
    }
}
