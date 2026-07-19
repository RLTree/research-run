use crate::Result;
use crate::domain::{KnowledgeRecord, RelationshipRecord};

use super::current_directory::discover_current;
use super::input::read_json_input;
use super::render::print_effect;
use super::structured_arguments::StructuredCommand;

pub(super) fn add_knowledge(command: StructuredCommand) -> Result<()> {
    let StructuredCommand::Add { input } = command;
    let record: KnowledgeRecord = read_json_input(&input)?;
    let workspace = discover_current()?;
    let created = workspace.add_knowledge(&record)?;
    print_effect("knowledge", &record.id, created)
}

pub(super) fn add_relationship(command: StructuredCommand) -> Result<()> {
    let StructuredCommand::Add { input } = command;
    let record: RelationshipRecord = read_json_input(&input)?;
    let workspace = discover_current()?;
    let created = workspace.add_relationship(&record)?;
    print_effect("relationship", &record.id, created)
}
