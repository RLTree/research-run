use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use proc_macro2::{Span, TokenStream, TokenTree};
use syn::spanned::Spanned;
use syn::visit::{self, Visit};

const PRODUCTION_LINE_CAP: usize = 250;
const TEST_LINE_CAP: usize = 250;
const FUNCTION_LINE_CAP: usize = 80;

#[derive(Default)]
struct RustDeclarations {
    symbols: Vec<(String, Span)>,
    functions: Vec<(String, Span)>,
}

impl RustDeclarations {
    fn symbol(&mut self, name: impl ToString, span: Span) {
        self.symbols.push((name.to_string(), span));
    }

    fn function(&mut self, name: impl ToString, span: Span) {
        let name = name.to_string();
        self.symbol(name.clone(), span);
        self.functions.push((name, span));
    }
}

impl<'ast> Visit<'ast> for RustDeclarations {
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        self.function(&node.sig.ident, node.span());
        visit::visit_item_fn(self, node);
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        self.function(&node.sig.ident, node.span());
        visit::visit_impl_item_fn(self, node);
    }

    fn visit_trait_item_fn(&mut self, node: &'ast syn::TraitItemFn) {
        self.function(&node.sig.ident, node.span());
        visit::visit_trait_item_fn(self, node);
    }

    fn visit_item(&mut self, node: &'ast syn::Item) {
        match node {
            syn::Item::Const(item) => self.symbol(&item.ident, item.span()),
            syn::Item::Enum(item) => self.symbol(&item.ident, item.span()),
            syn::Item::Macro(item) => {
                if let Some(ident) = &item.ident {
                    self.symbol(ident, item.span());
                }
            }
            syn::Item::Mod(item) => self.symbol(&item.ident, item.span()),
            syn::Item::Static(item) => self.symbol(&item.ident, item.span()),
            syn::Item::Struct(item) => self.symbol(&item.ident, item.span()),
            syn::Item::Trait(item) => self.symbol(&item.ident, item.span()),
            syn::Item::Type(item) => self.symbol(&item.ident, item.span()),
            syn::Item::Union(item) => self.symbol(&item.ident, item.span()),
            _ => {}
        }
        visit::visit_item(self, node);
    }
}

#[test]
fn every_rust_declaration_and_span_obeys_semantic_tree_law() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut violations = Vec::new();
    for path in rust_files(repository) {
        validate_file(repository, &path, &mut violations);
    }
    assert!(violations.is_empty(), "{}", violations.join("\n"));
}

#[test]
fn parser_fixture_covers_raw_comments_aliases_unions_and_macros() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = repository.join("fixtures/semantic-tree/green/parser_edges.rs");
    let source = fs::read_to_string(&path).expect("read parser fixture");
    let syntax = syn::parse_file(&source).expect("parse parser fixture");
    let mut declarations = RustDeclarations::default();
    declarations.visit_file(&syntax);
    let names = declarations
        .symbols
        .iter()
        .map(|(name, _)| name.as_str())
        .collect::<BTreeSet<_>>();
    for expected in [
        "EvidenceCollection",
        "ObservationValue",
        "render_observation",
        "parse_observation_fixture",
    ] {
        assert!(names.contains(expected), "parser missed {expected}");
    }
    assert_eq!(declarations.functions.len(), 1);
}

fn rust_files(repository: &Path) -> Vec<PathBuf> {
    let mut pending = vec![repository.join("src"), repository.join("tests")];
    let mut files = Vec::new();
    while let Some(path) = pending.pop() {
        for entry in fs::read_dir(path).expect("read Rust source directory") {
            let entry = entry.expect("read Rust source entry");
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

fn validate_file(repository: &Path, path: &Path, violations: &mut Vec<String>) {
    let source = fs::read_to_string(path).expect("read Rust source");
    let syntax = syn::parse_file(&source)
        .unwrap_or_else(|error| panic!("{} failed to parse: {error}", path.display()));
    let relative = path.strip_prefix(repository).expect("repository source");
    validate_token_lines(relative, &source, violations);
    let mut declarations = RustDeclarations::default();
    declarations.visit_file(&syntax);
    for (name, span) in declarations.symbols {
        if name != "reviewer" && name != "Status" && is_banned(&name) {
            violations.push(format!(
                "{}:{} has banned Rust symbol {name}",
                relative.display(),
                span.start().line
            ));
        }
    }
    for (name, span) in declarations.functions {
        let lines = span.end().line.saturating_sub(span.start().line) + 1;
        if lines > FUNCTION_LINE_CAP {
            violations.push(format!(
                "{}:{} function {name} spans {lines} lines; functions are capped at {FUNCTION_LINE_CAP}",
                relative.display(), span.start().line
            ));
        }
    }
}

fn validate_token_lines(path: &Path, source: &str, violations: &mut Vec<String>) {
    let tokens = source
        .parse::<TokenStream>()
        .unwrap_or_else(|error| panic!("{} failed to tokenize: {error}", path.display()));
    let mut lines = BTreeSet::new();
    collect_token_lines(tokens, &mut lines);
    let test = path.starts_with("tests")
        || path
            .file_name()
            .is_some_and(|name| name.to_string_lossy().contains("_tests"));
    let cap = if test {
        TEST_LINE_CAP
    } else {
        PRODUCTION_LINE_CAP
    };
    if lines.len() > cap {
        violations.push(format!(
            "{} has {} noncomment token lines; file cap is {cap}",
            path.display(),
            lines.len()
        ));
    }
}

fn collect_token_lines(tokens: TokenStream, lines: &mut BTreeSet<usize>) {
    for token in tokens {
        let span = token.span();
        lines.extend(span.start().line..=span.end().line);
        if let TokenTree::Group(group) = token {
            collect_token_lines(group.stream(), lines);
        }
    }
}

fn is_banned(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    let exact = [
        "goal",
        "slice",
        "lane",
        "workstream",
        "checkpoint",
        "backlog",
        "progress",
        "todo",
        "wip",
        "production_proof",
        "claim_closure",
        "readiness_packet",
        "finalization_work",
        "helpers",
        "utils",
        "common",
        "misc",
        "shared",
        "support",
        "services",
        "lib",
        "core",
        "fit_command",
        "fit_path",
        "fit_goal",
        "fit_slice",
    ];
    exact.contains(&name.as_str())
        || name == "internal"
        || name.starts_with("internal_")
        || numbered_process_name(&name, "gate")
        || numbered_process_name(&name, "phase")
}

fn numbered_process_name(name: &str, prefix: &str) -> bool {
    name.strip_prefix(prefix)
        .is_some_and(|suffix| suffix.is_empty() || suffix.bytes().all(|byte| byte.is_ascii_digit()))
}
