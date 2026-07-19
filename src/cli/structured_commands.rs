use crate::Result;
use crate::domain::{KnowledgeRecord, RelationshipRecord};

use super::commands::discover_current;
use super::input::read_json_input;
use super::render::print_effect;
use super::structured_arguments::StructuredCommand;

pub(super) fn add_knowledge(command: StructuredCommand) -> Result<()> {
    let StructuredCommand::Add { input } = command;
    let record: KnowledgeRecord = read_json_input(&input)?;
    let workspace = discover_current()?;
    print_effect("knowledge", &record.id, workspace.add_knowledge(&record)?)
}

pub(super) fn add_relationship(command: StructuredCommand) -> Result<()> {
    let StructuredCommand::Add { input } = command;
    let record: RelationshipRecord = read_json_input(&input)?;
    let workspace = discover_current()?;
    print_effect(
        "relationship",
        &record.id,
        workspace.add_relationship(&record)?,
    )
}
