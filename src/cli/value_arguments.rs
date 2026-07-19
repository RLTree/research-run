use clap::ValueEnum;

use crate::domain::{Assessment, Authorship, Outcome, SourceProvenance, Stance};

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(super) enum ProvenanceArg {
    Human,
    Imported,
    Ai,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(super) enum AuthorshipArg {
    Human,
    Ai,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(super) enum StanceArg {
    Supports,
    Limits,
    Contradicts,
    Context,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(super) enum OutcomeArg {
    Positive,
    Negative,
    Ambiguous,
    Inconclusive,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(super) enum ReviewArg {
    Unsupported,
    Limited,
    Supported,
    Contradicted,
}

impl From<ProvenanceArg> for SourceProvenance {
    fn from(value: ProvenanceArg) -> Self {
        match value {
            ProvenanceArg::Human => Self::Human,
            ProvenanceArg::Imported => Self::Imported,
            ProvenanceArg::Ai => Self::Ai,
        }
    }
}

impl From<AuthorshipArg> for Authorship {
    fn from(value: AuthorshipArg) -> Self {
        match value {
            AuthorshipArg::Human => Self::Human,
            AuthorshipArg::Ai => Self::Ai,
        }
    }
}

impl From<StanceArg> for Stance {
    fn from(value: StanceArg) -> Self {
        match value {
            StanceArg::Supports => Self::Supports,
            StanceArg::Limits => Self::Limits,
            StanceArg::Contradicts => Self::Contradicts,
            StanceArg::Context => Self::Context,
        }
    }
}

impl From<OutcomeArg> for Outcome {
    fn from(value: OutcomeArg) -> Self {
        match value {
            OutcomeArg::Positive => Self::Positive,
            OutcomeArg::Negative => Self::Negative,
            OutcomeArg::Ambiguous => Self::Ambiguous,
            OutcomeArg::Inconclusive => Self::Inconclusive,
        }
    }
}

impl From<ReviewArg> for Assessment {
    fn from(value: ReviewArg) -> Self {
        match value {
            ReviewArg::Unsupported => Self::Unsupported,
            ReviewArg::Limited => Self::Limited,
            ReviewArg::Supported => Self::Supported,
            ReviewArg::Contradicted => Self::Contradicted,
        }
    }
}
