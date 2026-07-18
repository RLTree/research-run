use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::domain::{
    Assessment, Authorship, CanonicalRecord, ClaimRecord, EvidenceLink, ExperimentReceipt,
    ProjectManifest, ReviewDecision, SourceRecord, ensure_unique_ids, validate_workspace_locator,
};
use crate::{Error, Result};

const STATE_DIRECTORY: &str = ".research-run";
const MAX_RECORD_BYTES: u64 = 1_048_576;
const MAX_RECORDS_PER_KIND: usize = 10_000;
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone)]
pub struct Workspace {
    root: PathBuf,
    state: PathBuf,
}

#[derive(Debug, Serialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub counts: BTreeMap<&'static str, usize>,
}

#[derive(Debug, Serialize)]
pub struct Status {
    pub format_version: u32,
    pub project: ProjectStatus,
    pub claim_ceiling: &'static str,
    pub counts: BTreeMap<&'static str, usize>,
    pub claims: Vec<ClaimStatus>,
    pub experiments: Vec<ExperimentStatus>,
    pub unreviewed_ai_drafts: Vec<AiDraftStatus>,
    pub blockers: Vec<String>,
    pub next_actions: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ProjectStatus {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct ClaimStatus {
    pub id: String,
    pub text: String,
    pub scope: String,
    pub assessment: Assessment,
    pub evidence: Vec<EvidenceStatus>,
    pub blockers: Vec<String>,
    pub next_action: String,
}

#[derive(Debug, Serialize)]
pub struct EvidenceStatus {
    pub id: String,
    pub stance: crate::domain::Stance,
    pub authorship: Authorship,
}

#[derive(Debug, Serialize)]
pub struct ExperimentStatus {
    pub id: String,
    pub outcome: crate::domain::Outcome,
    pub observations: Vec<String>,
    pub interpretation: String,
    pub limitations: Vec<String>,
    pub next_move: String,
}

#[derive(Debug, Serialize)]
pub struct AiDraftStatus {
    pub kind: &'static str,
    pub id: String,
    pub reason: &'static str,
}

#[derive(Debug, Serialize)]
pub struct RecoveryResult {
    pub recovered: Vec<String>,
    pub discarded_identical: Vec<String>,
}

struct Snapshot {
    manifest: ProjectManifest,
    sources: Vec<SourceRecord>,
    claims: Vec<ClaimRecord>,
    evidence: Vec<EvidenceLink>,
    experiments: Vec<ExperimentReceipt>,
    reviews: Vec<ReviewDecision>,
}

impl Workspace {
    pub fn initialize(root: &Path, name: &str) -> Result<Self> {
        let root = absolute_path(root)?;
        reject_symlink_chain(&root)?;
        fs::create_dir_all(&root)
            .map_err(|error| Error::io("create project directory", &root, error))?;
        let workspace = Self {
            state: root.join(STATE_DIRECTORY),
            root,
        };
        reject_symlink_chain(&workspace.state)?;
        fs::create_dir(&workspace.state)
            .or_else(|error| {
                if error.kind() == std::io::ErrorKind::AlreadyExists {
                    Ok(())
                } else {
                    Err(error)
                }
            })
            .map_err(|error| Error::io("create state directory", &workspace.state, error))?;
        for directory in ["sources", "claims", "evidence", "experiments", "reviews"] {
            let path = workspace.state.join(directory);
            reject_symlink_chain(&path)?;
            fs::create_dir(&path)
                .or_else(|error| {
                    if error.kind() == std::io::ErrorKind::AlreadyExists {
                        Ok(())
                    } else {
                        Err(error)
                    }
                })
                .map_err(|error| Error::io("create record directory", &path, error))?;
        }
        let manifest = ProjectManifest::new(name)?;
        workspace.publish(&workspace.state.join("manifest.json"), &manifest)?;
        Ok(workspace)
    }

    pub fn discover(start: &Path) -> Result<Self> {
        let start = absolute_path(start)?;
        reject_symlink_chain(&start)?;
        for candidate in start.ancestors() {
            let state = candidate.join(STATE_DIRECTORY);
            let manifest = state.join("manifest.json");
            if manifest.exists() {
                reject_symlink_chain(&state)?;
                reject_symlink_chain(&manifest)?;
                let workspace = Self {
                    root: candidate.to_path_buf(),
                    state,
                };
                workspace.read_manifest()?;
                return Ok(workspace);
            }
        }
        Err(Error::NotFound(
            "no Research Run workspace found; run 'research-run init' first".to_owned(),
        ))
    }

    pub fn for_recovery(root: &Path) -> Result<Self> {
        let root = absolute_path(root)?;
        reject_symlink_chain(&root)?;
        let state = root.join(STATE_DIRECTORY);
        reject_symlink_chain(&state)?;
        if !state.is_dir() {
            return Err(Error::NotFound(format!(
                "no partial or complete Research Run workspace at {}",
                root.display()
            )));
        }
        Ok(Self { root, state })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn add_source(&self, record: &SourceRecord) -> Result<bool> {
        record.validate()?;
        self.publish_record("sources", record)
    }

    pub fn add_claim(&self, record: &ClaimRecord) -> Result<bool> {
        record.validate()?;
        self.publish_record("claims", record)
    }

    pub fn add_experiment(&self, record: &ExperimentReceipt) -> Result<bool> {
        record.validate()?;
        self.validate_artifact_paths(&record.artifacts)?;
        self.publish_record("experiments", record)
    }

    pub fn add_evidence(&self, record: &EvidenceLink) -> Result<bool> {
        record.validate()?;
        let snapshot = self.load_snapshot()?;
        if !snapshot
            .claims
            .iter()
            .any(|claim| claim.id == record.claim_id)
        {
            return Err(Error::invalid("claim reference", "claim does not exist"));
        }
        if let Some(source_id) = &record.source_id
            && !snapshot
                .sources
                .iter()
                .any(|source| &source.id == source_id)
        {
            return Err(Error::invalid("source reference", "source does not exist"));
        }
        if let Some(experiment_id) = &record.experiment_id
            && !snapshot
                .experiments
                .iter()
                .any(|experiment| &experiment.id == experiment_id)
        {
            return Err(Error::invalid(
                "experiment reference",
                "experiment does not exist",
            ));
        }
        if let Some(locator) = &record.artifact {
            self.validate_workspace_path(locator)?;
        }
        self.publish_record("evidence", record)
    }

    pub fn add_review(&self, record: &ReviewDecision) -> Result<bool> {
        record.validate()?;
        let snapshot = self.load_snapshot()?;
        if !snapshot
            .claims
            .iter()
            .any(|claim| claim.id == record.claim_id)
        {
            return Err(Error::invalid("claim reference", "claim does not exist"));
        }
        if snapshot
            .reviews
            .iter()
            .any(|review| review.claim_id == record.claim_id)
        {
            return Err(Error::Conflict(format!(
                "claim {} already has a v0.1 review decision",
                record.claim_id
            )));
        }
        self.publish_record("reviews", record)
    }

    pub fn validate(&self) -> ValidationResult {
        match self.load_snapshot() {
            Ok(snapshot) => {
                let counts = snapshot.counts();
                let errors = self.reference_errors(&snapshot);
                ValidationResult {
                    valid: errors.is_empty(),
                    errors,
                    counts,
                }
            }
            Err(error) => ValidationResult {
                valid: false,
                errors: vec![error.to_string()],
                counts: BTreeMap::new(),
            },
        }
    }

    pub fn status(&self) -> Result<Status> {
        let mut snapshot = self.load_snapshot()?;
        let reference_errors = self.reference_errors(&snapshot);
        if !reference_errors.is_empty() {
            return Err(Error::invalid(
                "workspace",
                "reference validation failed; run 'research-run validate'",
            ));
        }
        snapshot
            .claims
            .sort_by(|left, right| left.id.cmp(&right.id));
        snapshot
            .evidence
            .sort_by(|left, right| left.id.cmp(&right.id));
        snapshot
            .experiments
            .sort_by(|left, right| left.id.cmp(&right.id));
        let counts = snapshot.counts();
        let reviewed_claims: BTreeSet<_> = snapshot
            .reviews
            .iter()
            .map(|review| review.claim_id.as_str())
            .collect();
        let mut unreviewed_ai_drafts = Vec::new();
        for source in &snapshot.sources {
            let referenced_claims: Vec<_> = snapshot
                .evidence
                .iter()
                .filter(|link| link.source_id.as_deref() == Some(source.id.as_str()))
                .map(|link| link.claim_id.as_str())
                .collect();
            if source.provenance == crate::domain::SourceProvenance::Ai
                && (referenced_claims.is_empty()
                    || referenced_claims
                        .iter()
                        .any(|claim_id| !reviewed_claims.contains(claim_id)))
            {
                unreviewed_ai_drafts.push(AiDraftStatus {
                    kind: "source",
                    id: source.id.clone(),
                    reason: "AI-provenance source is not covered by a claim review.",
                });
            }
        }
        for claim in &snapshot.claims {
            if claim.authorship == Authorship::Ai && !reviewed_claims.contains(claim.id.as_str()) {
                unreviewed_ai_drafts.push(AiDraftStatus {
                    kind: "claim",
                    id: claim.id.clone(),
                    reason: "AI-authored claim lacks an explicit human review decision.",
                });
            }
        }
        for link in &snapshot.evidence {
            if link.authorship == Authorship::Ai
                && !reviewed_claims.contains(link.claim_id.as_str())
            {
                unreviewed_ai_drafts.push(AiDraftStatus {
                    kind: "evidence",
                    id: link.id.clone(),
                    reason: "AI-authored evidence link is not covered by a claim review.",
                });
            }
        }
        unreviewed_ai_drafts
            .sort_by(|left, right| (left.kind, &left.id).cmp(&(right.kind, &right.id)));
        let mut claims = Vec::with_capacity(snapshot.claims.len());
        for claim in snapshot.claims {
            let links: Vec<_> = snapshot
                .evidence
                .iter()
                .filter(|link| link.claim_id == claim.id)
                .collect();
            let review = snapshot
                .reviews
                .iter()
                .find(|review| review.claim_id == claim.id);
            let ai_draft = claim.authorship == Authorship::Ai
                || links.iter().any(|link| link.authorship == Authorship::Ai);
            let (assessment, blockers, next_action) = if let Some(review) = review {
                (
                    review.decision,
                    Vec::new(),
                    "Re-review when material evidence changes.".to_owned(),
                )
            } else if ai_draft {
                (
                    Assessment::Unreviewed,
                    vec![
                        "AI-authored material requires an explicit human review decision."
                            .to_owned(),
                    ],
                    format!("Add a human review for {}.", claim.id),
                )
            } else {
                let mut blockers =
                    vec!["Claim lacks an explicit human review decision.".to_owned()];
                if links.is_empty() {
                    blockers.push("Claim has no evidence links.".to_owned());
                }
                let next = if links.is_empty() {
                    format!("Add evidence for {}.", claim.id)
                } else {
                    format!("Add a human review for {}.", claim.id)
                };
                (Assessment::Unsupported, blockers, next)
            };
            claims.push(ClaimStatus {
                id: claim.id,
                text: claim.text,
                scope: claim.scope,
                assessment,
                evidence: links
                    .into_iter()
                    .map(|link| EvidenceStatus {
                        id: link.id.clone(),
                        stance: link.stance,
                        authorship: link.authorship,
                    })
                    .collect(),
                blockers,
                next_action,
            });
        }
        let experiments = snapshot
            .experiments
            .into_iter()
            .map(|experiment| ExperimentStatus {
                id: experiment.id,
                outcome: experiment.outcome,
                observations: experiment.observations,
                interpretation: experiment.interpretation,
                limitations: experiment.limitations,
                next_move: experiment.next_move,
            })
            .collect();
        let blockers = claims
            .iter()
            .flat_map(|claim| claim.blockers.iter().cloned())
            .collect();
        let next_actions = claims
            .iter()
            .map(|claim| claim.next_action.clone())
            .collect();
        Ok(Status {
            format_version: crate::domain::FORMAT_VERSION,
            project: ProjectStatus {
                id: snapshot.manifest.project_id,
                name: snapshot.manifest.name,
            },
            claim_ceiling: "Assessments describe reviewed support within this workspace; they do not establish scientific truth or real-world validity.",
            counts,
            claims,
            experiments,
            unreviewed_ai_drafts,
            blockers,
            next_actions,
        })
    }

    pub fn recover(&self) -> Result<RecoveryResult> {
        let mut result = RecoveryResult {
            recovered: Vec::new(),
            discarded_identical: Vec::new(),
        };
        self.recover_directory(&self.state, &mut result)?;
        let snapshot = self.load_snapshot_allow_pending(true)?;
        let errors = self.reference_errors(&snapshot);
        if !errors.is_empty() {
            return Err(Error::invalid(
                "existing workspace",
                "reference validation failed before recovery",
            ));
        }
        for directory in ["sources", "claims", "experiments", "evidence", "reviews"] {
            self.recover_directory(&self.state.join(directory), &mut result)?;
        }
        let final_snapshot = self.load_snapshot()?;
        let errors = self.reference_errors(&final_snapshot);
        if !errors.is_empty() {
            return Err(Error::invalid(
                "recovered workspace",
                "reference validation failed after recovery",
            ));
        }
        Ok(result)
    }

    fn recover_directory(&self, directory: &Path, result: &mut RecoveryResult) -> Result<()> {
        reject_symlink_chain(directory)?;
        let mut pending_effects = Vec::new();
        for entry in fs::read_dir(directory)
            .map_err(|error| Error::io("read recovery directory", directory, error))?
        {
            let entry =
                entry.map_err(|error| Error::io("read recovery entry", directory, error))?;
            let path = entry.path();
            let Some(target_name) = interrupted_target_name(&path) else {
                continue;
            };
            reject_symlink_chain(&path)?;
            let target = directory.join(target_name);
            let pending = read_bounded(&path)?;
            pending_effects.push((path, target, pending));
        }
        pending_effects.sort_by(|left, right| left.0.cmp(&right.0));

        let mut content_by_target: BTreeMap<PathBuf, Vec<u8>> = BTreeMap::new();
        let mut pending_review_claims = BTreeSet::new();
        for (_, target, pending) in &pending_effects {
            self.validate_pending_record(directory, target, pending)?;
            if let Some(existing) = content_by_target.get(target)
                && existing != pending
            {
                return Err(Error::AmbiguousEffect(format!(
                    "conflicting pending publications target {}; inspect them before recovery",
                    target.display()
                )));
            }
            content_by_target.insert(target.clone(), pending.clone());
            if directory.file_name() == Some(OsStr::new("reviews")) {
                let review: ReviewDecision =
                    serde_json::from_slice(pending).map_err(|_| Error::MalformedJson {
                        path: target.clone(),
                    })?;
                if !pending_review_claims.insert(review.claim_id.clone()) {
                    return Err(Error::AmbiguousEffect(format!(
                        "multiple pending review decisions target claim {}; inspect them before recovery",
                        review.claim_id
                    )));
                }
            }
        }

        for (path, target, pending) in pending_effects {
            if target.exists() {
                let current = read_bounded(&target)?;
                if current != pending {
                    return Err(Error::AmbiguousEffect(format!(
                        "recovery conflict for {}; inspect both files",
                        target.display()
                    )));
                }
                fs::remove_file(&path)
                    .map_err(|error| Error::io("remove identical pending file", &path, error))?;
                result
                    .discarded_identical
                    .push(target.display().to_string());
            } else {
                fs::hard_link(&path, &target)
                    .map_err(|error| Error::io("publish recovered record", &target, error))?;
                sync_directory(directory)?;
                fs::remove_file(&path)
                    .map_err(|error| Error::io("remove recovered pending file", &path, error))?;
                sync_directory(directory)?;
                result.recovered.push(target.display().to_string());
            }
        }
        Ok(())
    }

    fn validate_pending_record(&self, directory: &Path, target: &Path, bytes: &[u8]) -> Result<()> {
        if directory.file_name() == Some(OsStr::new(STATE_DIRECTORY)) {
            let manifest: ProjectManifest =
                serde_json::from_slice(bytes).map_err(|_| Error::MalformedJson {
                    path: target.to_path_buf(),
                })?;
            return manifest.validate();
        }
        let directory_name = directory
            .file_name()
            .and_then(OsStr::to_str)
            .unwrap_or_default();
        macro_rules! parse_record {
            ($record_type:ty) => {{
                let record: $record_type =
                    serde_json::from_slice(bytes).map_err(|_| Error::MalformedJson {
                        path: target.to_path_buf(),
                    })?;
                record.validate()?;
                if target.file_stem().and_then(OsStr::to_str) != Some(record.id()) {
                    return Err(Error::invalid(
                        "recovery target",
                        "filename does not match record id",
                    ));
                }
                record
            }};
        }
        match directory_name {
            "sources" => {
                let _: SourceRecord = parse_record!(SourceRecord);
            }
            "claims" => {
                let _: ClaimRecord = parse_record!(ClaimRecord);
            }
            "experiments" => {
                let record: ExperimentReceipt = parse_record!(ExperimentReceipt);
                self.validate_artifact_paths(&record.artifacts)?;
            }
            "evidence" => {
                let record: EvidenceLink = parse_record!(EvidenceLink);
                self.require_existing_record::<ClaimRecord>("claims", &record.claim_id)?;
                if let Some(source_id) = &record.source_id {
                    self.require_existing_record::<SourceRecord>("sources", source_id)?;
                }
                if let Some(experiment_id) = &record.experiment_id {
                    self.require_existing_record::<ExperimentReceipt>(
                        "experiments",
                        experiment_id,
                    )?;
                }
                if let Some(locator) = &record.artifact {
                    self.validate_workspace_path(locator)?;
                }
            }
            "reviews" => {
                let record: ReviewDecision = parse_record!(ReviewDecision);
                self.require_existing_record::<ClaimRecord>("claims", &record.claim_id)?;
                let reviews: Vec<ReviewDecision> = self.load_records("reviews", true)?;
                if reviews
                    .iter()
                    .any(|existing| existing.claim_id == record.claim_id)
                {
                    return Err(Error::Conflict(format!(
                        "claim {} already has a v0.1 review decision",
                        record.claim_id
                    )));
                }
            }
            _ => {
                return Err(Error::invalid(
                    "recovery directory",
                    "not a canonical record directory",
                ));
            }
        }
        Ok(())
    }

    fn require_existing_record<T>(&self, directory: &str, id: &str) -> Result<()>
    where
        T: DeserializeOwned + CanonicalRecord,
    {
        let path = self.state.join(directory).join(format!("{id}.json"));
        let record: T = read_json(&path).map_err(|_| {
            Error::invalid(
                "recovered record reference",
                format!("{directory}/{id} is missing or invalid"),
            )
        })?;
        record.validate()
    }

    fn publish_record<T: CanonicalRecord>(&self, directory: &str, record: &T) -> Result<bool> {
        let target = self
            .state
            .join(directory)
            .join(format!("{}.json", record.id()));
        self.publish(&target, record)
    }

    fn publish<T: Serialize>(&self, target: &Path, value: &T) -> Result<bool> {
        reject_symlink_chain(target)?;
        let mut content = serde_json::to_vec_pretty(value)
            .map_err(|_| Error::invalid("canonical record", "serialization failed"))?;
        content.push(b'\n');
        if content.len() as u64 > MAX_RECORD_BYTES {
            return Err(Error::Budget(format!(
                "record exceeds the {MAX_RECORD_BYTES} byte budget"
            )));
        }
        if target.exists() {
            if read_bounded(target)? == content {
                return Ok(false);
            }
            return Err(Error::Conflict(format!(
                "record identity already exists with different content: {}",
                target.display()
            )));
        }
        ensure_no_pending_effect(target)?;
        let parent = target
            .parent()
            .ok_or_else(|| Error::invalid("publication path", "target has no parent"))?;
        let file_name = target
            .file_name()
            .and_then(OsStr::to_str)
            .ok_or_else(|| Error::invalid("publication path", "target filename is not UTF-8"))?;
        let nonce = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temporary = parent.join(format!(".{file_name}.{}.{}.tmp", std::process::id(), nonce));
        let mut cleanup = PendingCleanup::new(temporary.clone());
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| Error::io("create pending record", &temporary, error))?;
        file.write_all(&content)
            .map_err(|error| Error::io("write pending record", &temporary, error))?;
        file.sync_all()
            .map_err(|error| Error::io("sync pending record", &temporary, error))?;
        drop(file);
        match fs::hard_link(&temporary, target) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                if read_bounded(target)? == content {
                    return Ok(false);
                }
                return Err(Error::Conflict(format!(
                    "record identity appeared during publication: {}",
                    target.display()
                )));
            }
            Err(error) => return Err(Error::io("publish canonical record", target, error)),
        }
        sync_directory(parent).map_err(|error| {
            cleanup.preserve();
            Error::AmbiguousEffect(format!(
                "record may be published at {}; directory sync failed: {error}",
                target.display()
            ))
        })?;
        fs::remove_file(&temporary).map_err(|error| {
            cleanup.preserve();
            Error::AmbiguousEffect(format!(
                "record was published at {} but pending file cleanup failed: {error}",
                target.display()
            ))
        })?;
        cleanup.disarm();
        sync_directory(parent)?;
        Ok(true)
    }

    fn read_manifest(&self) -> Result<ProjectManifest> {
        let manifest: ProjectManifest = read_json(&self.state.join("manifest.json"))?;
        manifest.validate()?;
        Ok(manifest)
    }

    fn load_snapshot(&self) -> Result<Snapshot> {
        self.load_snapshot_allow_pending(false)
    }

    fn load_snapshot_allow_pending(&self, allow_pending: bool) -> Result<Snapshot> {
        let snapshot = Snapshot {
            manifest: self.read_manifest()?,
            sources: self.load_records("sources", allow_pending)?,
            claims: self.load_records("claims", allow_pending)?,
            evidence: self.load_records("evidence", allow_pending)?,
            experiments: self.load_records("experiments", allow_pending)?,
            reviews: self.load_records("reviews", allow_pending)?,
        };
        ensure_unique_ids(
            snapshot.sources.iter().map(|record| record.id.as_str()),
            "sources",
        )?;
        ensure_unique_ids(
            snapshot.claims.iter().map(|record| record.id.as_str()),
            "claims",
        )?;
        ensure_unique_ids(
            snapshot.evidence.iter().map(|record| record.id.as_str()),
            "evidence",
        )?;
        ensure_unique_ids(
            snapshot.experiments.iter().map(|record| record.id.as_str()),
            "experiments",
        )?;
        ensure_unique_ids(
            snapshot.reviews.iter().map(|record| record.id.as_str()),
            "reviews",
        )?;
        Ok(snapshot)
    }

    fn load_records<T>(&self, directory: &str, allow_pending: bool) -> Result<Vec<T>>
    where
        T: DeserializeOwned + CanonicalRecord,
    {
        let path = self.state.join(directory);
        reject_symlink_chain(&path)?;
        let mut entries = Vec::new();
        for entry in
            fs::read_dir(&path).map_err(|error| Error::io("read record directory", &path, error))?
        {
            let entry = entry.map_err(|error| Error::io("read record entry", &path, error))?;
            let record_path = entry.path();
            reject_symlink_chain(&record_path)?;
            if interrupted_target_name(&record_path).is_some() {
                if allow_pending {
                    continue;
                }
                return Err(Error::AmbiguousEffect(format!(
                    "interrupted publication found at {}; run 'research-run recover'",
                    record_path.display()
                )));
            }
            if record_path.extension() != Some(OsStr::new("json")) {
                return Err(Error::invalid(
                    "record directory",
                    format!("unexpected file {}", record_path.display()),
                ));
            }
            entries.push(record_path);
        }
        entries.sort();
        if entries.len() > MAX_RECORDS_PER_KIND {
            return Err(Error::Budget(format!(
                "{directory} exceeds the {MAX_RECORDS_PER_KIND} record budget"
            )));
        }
        let mut records = Vec::with_capacity(entries.len());
        for record_path in entries {
            let record: T = read_json(&record_path)?;
            record.validate()?;
            if record_path.file_stem().and_then(OsStr::to_str) != Some(record.id()) {
                return Err(Error::invalid(
                    "record filename",
                    format!("must match id in {}", record_path.display()),
                ));
            }
            records.push(record);
        }
        Ok(records)
    }

    fn reference_errors(&self, snapshot: &Snapshot) -> Vec<String> {
        let sources: BTreeSet<_> = snapshot
            .sources
            .iter()
            .map(|record| record.id.as_str())
            .collect();
        let claims: BTreeSet<_> = snapshot
            .claims
            .iter()
            .map(|record| record.id.as_str())
            .collect();
        let experiments: BTreeSet<_> = snapshot
            .experiments
            .iter()
            .map(|record| record.id.as_str())
            .collect();
        let mut errors = Vec::new();
        for link in &snapshot.evidence {
            if !claims.contains(link.claim_id.as_str()) {
                errors.push(format!("evidence/{}: unknown claim reference", link.id));
            }
            if let Some(source_id) = &link.source_id
                && !sources.contains(source_id.as_str())
            {
                errors.push(format!("evidence/{}: unknown source reference", link.id));
            }
            if let Some(experiment_id) = &link.experiment_id
                && !experiments.contains(experiment_id.as_str())
            {
                errors.push(format!(
                    "evidence/{}: unknown experiment reference",
                    link.id
                ));
            }
            if let Some(locator) = &link.artifact
                && let Err(error) = self.validate_workspace_path(locator)
            {
                errors.push(format!("evidence/{}: {error}", link.id));
            }
        }
        let mut reviewed_claims = BTreeSet::new();
        for review in &snapshot.reviews {
            if !claims.contains(review.claim_id.as_str()) {
                errors.push(format!("reviews/{}: unknown claim reference", review.id));
            }
            if !reviewed_claims.insert(review.claim_id.as_str()) {
                errors.push(format!(
                    "reviews/{}: multiple v0.1 reviews for one claim are ambiguous",
                    review.id
                ));
            }
        }
        for experiment in &snapshot.experiments {
            if let Err(error) = self.validate_artifact_paths(&experiment.artifacts) {
                errors.push(format!("experiments/{}: {error}", experiment.id));
            }
        }
        errors
    }

    fn validate_artifact_paths(&self, artifacts: &[crate::domain::ArtifactPointer]) -> Result<()> {
        for artifact in artifacts {
            if artifact.locator_type == crate::domain::ArtifactLocatorType::Workspace {
                self.validate_workspace_path(&artifact.locator)?;
            }
        }
        Ok(())
    }

    fn validate_workspace_path(&self, locator: &str) -> Result<PathBuf> {
        validate_workspace_locator(locator)?;
        let candidate = self.root.join(locator);
        reject_symlink_chain(&candidate)?;
        Ok(candidate)
    }
}

impl Snapshot {
    fn counts(&self) -> BTreeMap<&'static str, usize> {
        BTreeMap::from([
            ("source", self.sources.len()),
            ("claim", self.claims.len()),
            ("evidence", self.evidence.len()),
            ("experiment", self.experiments.len()),
            ("review", self.reviews.len()),
        ])
    }
}

struct PendingCleanup {
    path: PathBuf,
    remove: bool,
}

impl PendingCleanup {
    fn new(path: PathBuf) -> Self {
        Self { path, remove: true }
    }

    fn preserve(&mut self) {
        self.remove = false;
    }

    fn disarm(&mut self) {
        self.remove = false;
    }
}

impl Drop for PendingCleanup {
    fn drop(&mut self) {
        if self.remove {
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let bytes = read_bounded(path)?;
    serde_json::from_slice(&bytes).map_err(|_| Error::MalformedJson {
        path: path.to_path_buf(),
    })
}

fn read_bounded(path: &Path) -> Result<Vec<u8>> {
    reject_symlink_chain(path)?;
    let metadata =
        fs::symlink_metadata(path).map_err(|error| Error::io("inspect record", path, error))?;
    if !metadata.is_file() {
        return Err(Error::invalid(
            "record path",
            format!("{} is not a regular file", path.display()),
        ));
    }
    if metadata.len() > MAX_RECORD_BYTES {
        return Err(Error::Budget(format!(
            "{} exceeds the {MAX_RECORD_BYTES} byte record budget",
            path.display()
        )));
    }
    let file = File::open(path).map_err(|error| Error::io("open record", path, error))?;
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(MAX_RECORD_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| Error::io("read record", path, error))?;
    if bytes.len() as u64 > MAX_RECORD_BYTES {
        return Err(Error::Budget(format!(
            "{} grew beyond the record budget while reading",
            path.display()
        )));
    }
    reject_symlink_chain(path)?;
    Ok(bytes)
}

fn absolute_path(path: &Path) -> Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        let current = std::env::current_dir()
            .map_err(|error| Error::io("read current directory", ".", error))?;
        current.join(path)
    };
    if absolute.exists() {
        let metadata = fs::symlink_metadata(&absolute)
            .map_err(|error| Error::io("inspect workspace root", &absolute, error))?;
        if metadata.file_type().is_symlink() {
            return Err(Error::invalid(
                "workspace root",
                format!("symlink is forbidden: {}", absolute.display()),
            ));
        }
        return absolute
            .canonicalize()
            .map_err(|error| Error::io("canonicalize workspace root", &absolute, error));
    }
    let existing = absolute
        .ancestors()
        .find(|ancestor| ancestor.exists())
        .ok_or_else(|| Error::invalid("workspace root", "has no existing ancestor"))?;
    let metadata = fs::symlink_metadata(existing)
        .map_err(|error| Error::io("inspect workspace ancestor", existing, error))?;
    if metadata.file_type().is_symlink() {
        return Err(Error::invalid(
            "workspace root",
            format!("symlink is forbidden: {}", existing.display()),
        ));
    }
    let canonical = existing
        .canonicalize()
        .map_err(|error| Error::io("canonicalize workspace ancestor", existing, error))?;
    let suffix = absolute
        .strip_prefix(existing)
        .map_err(|_| Error::invalid("workspace root", "cannot resolve path suffix"))?;
    Ok(canonical.join(suffix))
}

fn reject_symlink_chain(path: &Path) -> Result<()> {
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(Error::invalid(
                    "workspace path",
                    format!("symlink is forbidden: {}", ancestor.display()),
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(Error::io("inspect workspace path", ancestor, error)),
        }
    }
    Ok(())
}

fn ensure_no_pending_effect(target: &Path) -> Result<()> {
    let parent = target
        .parent()
        .ok_or_else(|| Error::invalid("publication target", "missing parent"))?;
    let target_name = target
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or_else(|| Error::invalid("publication target", "filename is not UTF-8"))?;
    let prefix = format!(".{target_name}.");
    for entry in
        fs::read_dir(parent).map_err(|error| Error::io("inspect pending effects", parent, error))?
    {
        let entry = entry.map_err(|error| Error::io("inspect pending effect", parent, error))?;
        let name = entry.file_name();
        let name = name
            .to_str()
            .ok_or_else(|| Error::invalid("pending effect", "filename is not UTF-8"))?;
        if name.starts_with(&prefix) && name.ends_with(".tmp") {
            return Err(Error::AmbiguousEffect(format!(
                "interrupted publication found at {}; run 'research-run recover' before retry",
                entry.path().display()
            )));
        }
    }
    Ok(())
}

fn interrupted_target_name(path: &Path) -> Option<&str> {
    let name = path.file_name()?.to_str()?;
    let body = name.strip_prefix('.')?.strip_suffix(".tmp")?;
    let (without_sequence, sequence) = body.rsplit_once('.')?;
    if sequence.parse::<u64>().is_err() {
        return None;
    }
    let (target, process) = without_sequence.rsplit_once('.')?;
    if process.parse::<u32>().is_err() || !target.ends_with(".json") {
        return None;
    }
    Some(target)
}

fn sync_directory(path: &Path) -> Result<()> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| Error::io("sync record directory", path, error))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::{Workspace, interrupted_target_name};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temporary() -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "research-run-workspace-test-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).expect("temporary directory");
        path
    }

    #[test]
    fn pending_filename_recovers_exact_target() {
        let path = std::path::Path::new(".claim-one.json.123.4.tmp");
        assert_eq!(interrupted_target_name(path), Some("claim-one.json"));
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_state_directory_is_rejected() {
        use std::os::unix::fs::symlink;

        let root = temporary();
        let outside = temporary();
        symlink(&outside, root.join(".research-run")).expect("create symlink fixture");
        let error = Workspace::initialize(&root, "Unsafe").expect_err("must reject symlink");
        assert!(error.to_string().contains("symlink"));
        fs::remove_dir_all(root).expect("remove fixture");
        fs::remove_dir_all(outside).expect("remove fixture");
    }
}
