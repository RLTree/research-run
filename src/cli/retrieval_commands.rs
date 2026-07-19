use std::fmt::Write;

use crate::Result;
use crate::workspace::{ContextBundle, ProjectionItem, ProjectionResult};

use super::current_directory::discover_current;
use super::render::{print_json, print_text, terminal_text};
use super::retrieval_arguments::{
    ContextArgs, LimitArgs, ListArgs, RelatedArgs, SearchArgs, ShowArgs,
};

pub(super) fn list(args: ListArgs) -> Result<()> {
    let workspace = discover_current()?;
    let result = workspace.list(args.kind.as_deref(), args.output.limit)?;
    print_projection(&result, args.output.human)
}

pub(super) fn show(args: ShowArgs) -> Result<()> {
    let workspace = discover_current()?;
    let item = workspace.show(&args.kind, &args.id)?;
    if args.human {
        print_text(&render_items("Record", std::slice::from_ref(&item)))
    } else {
        print_json(&item)
    }
}

pub(super) fn search(args: SearchArgs) -> Result<()> {
    let workspace = discover_current()?;
    let result = workspace.search(&args.query, args.output.limit)?;
    print_projection(&result, args.output.human)
}

pub(super) fn recent(args: LimitArgs) -> Result<()> {
    let workspace = discover_current()?;
    let result = workspace.recent(args.limit)?;
    print_projection(&result, args.human)
}

pub(super) fn timeline(args: LimitArgs) -> Result<()> {
    let workspace = discover_current()?;
    let result = workspace.timeline(args.limit)?;
    print_projection(&result, args.human)
}

pub(super) fn unresolved(args: LimitArgs) -> Result<()> {
    let workspace = discover_current()?;
    let result = workspace.unresolved(args.limit)?;
    print_projection(&result, args.human)
}

pub(super) fn blockers(args: LimitArgs) -> Result<()> {
    let workspace = discover_current()?;
    let result = workspace.blocker_items(args.limit)?;
    print_projection(&result, args.human)
}

pub(super) fn next(args: LimitArgs) -> Result<()> {
    let workspace = discover_current()?;
    let result = workspace.next_action_items(args.limit)?;
    print_projection(&result, args.human)
}

pub(super) fn related(args: RelatedArgs) -> Result<()> {
    let workspace = discover_current()?;
    let items = workspace.related(&args.kind, &args.id, args.output.limit)?;
    if args.output.human {
        let mut output = String::from("Relationships\n");
        for item in items {
            writeln!(
                output,
                "- {}: {} {}/{} -> {}/{}\n  {}\n  Authority: {}",
                terminal_text(&item.id),
                terminal_text(&item.relationship),
                terminal_text(&item.from_kind),
                terminal_text(&item.from_id),
                terminal_text(&item.to_kind),
                terminal_text(&item.to_id),
                terminal_text(&item.rationale),
                terminal_text(&item.authority_path)
            )
            .expect("String rendering is infallible");
        }
        print_text(&output)
    } else {
        print_json(&items)
    }
}

pub(super) fn context(args: ContextArgs) -> Result<()> {
    let workspace = discover_current()?;
    let bundle = workspace.context(args.query.as_deref(), args.output.limit)?;
    if args.output.human {
        print_text(&render_context(&bundle))
    } else {
        print_json(&bundle)
    }
}

fn print_projection(result: &ProjectionResult, human: bool) -> Result<()> {
    if human {
        print_text(&render_items(result.kind, &result.items))
    } else {
        print_json(result)
    }
}

pub(super) fn render_context(bundle: &ContextBundle) -> String {
    let mut output = format!(
        "Research Run context: {} ({})\nScope: {}\nClaim ceiling: {}\n",
        terminal_text(&bundle.project_name),
        terminal_text(&bundle.project_id),
        terminal_text(&bundle.scope),
        terminal_text(&bundle.claim_ceiling)
    );
    output.push_str(&render_items("Matches", &bundle.matches));
    output.push_str(&render_items("Unresolved", &bundle.unresolved));
    output.push_str(&render_items("Blockers", &bundle.blockers));
    output.push_str(&render_items("Next actions", &bundle.next_actions));
    output
}

fn render_items(title: &str, items: &[ProjectionItem]) -> String {
    let mut output = format!("{}\n", terminal_text(title));
    if items.is_empty() {
        output.push_str("- None.\n");
    }
    for item in items {
        let flags = match (item.stale, item.invalidated) {
            (_, true) => " [invalidated]",
            (true, false) => " [stale]",
            _ => "",
        };
        writeln!(
            output,
            "- {}/{} ({}){}: {}\n  {}\n  Authority: {}",
            terminal_text(&item.kind),
            terminal_text(&item.id),
            terminal_text(&item.subtype),
            flags,
            terminal_text(&item.title),
            terminal_text(&item.summary),
            terminal_text(&item.authority_path)
        )
        .expect("String rendering is infallible");
    }
    output
}
