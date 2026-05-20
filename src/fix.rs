//! Fix strategy types for the Konveyor migration pipeline.
//!
//! These types define the JSON schema for `fix-strategies.json`, which is
//! written by the semver-analyzer and read by the frontend-analyzer-provider's
//! fix engine. All types derive both `Serialize` and `Deserialize` to ensure
//! round-trip compatibility.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use anyhow::{Context, Result};

// ── Fix guidance types ──────────────────────────────────────────────────

/// Top-level fix guidance document written to `fix-guidance.yaml`.
///
/// Contains only summary statistics derived from fix strategies.
/// Per-rule fix details are in `fix-strategies.json` (key-based, correct).
#[derive(Debug, Serialize, Deserialize)]
pub struct FixGuidanceDoc {
    /// Version range this guidance applies to.
    pub migration: MigrationInfo,
    /// Summary statistics.
    pub summary: FixSummary,
}

/// How confident a fix is.
///
/// Used by the fix-engine's `PlannedFix` to classify applied fixes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FixConfidence {
    /// Mechanical rename or direct replacement -- safe to auto-apply.
    Exact,
    /// Pattern-based fix -- likely correct but may need review.
    High,
    /// Inferred fix -- needs human verification.
    Medium,
    /// Best-effort suggestion -- may not be applicable.
    Low,
}

/// Where a fix originates.
///
/// Used by the fix-engine's `PlannedFix` to classify fix provenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FixSource {
    /// Deterministic -- derived from structural analysis.
    Pattern,
    /// AI-generated -- from LLM behavioral analysis.
    Llm,
    /// Flagged for manual intervention.
    Manual,
}

/// Migration metadata.
#[derive(Debug, Serialize, Deserialize)]
pub struct MigrationInfo {
    pub from_ref: String,
    pub to_ref: String,
    pub generated_by: String,
}

/// Summary of fix guidance.
#[derive(Debug, Serialize, Deserialize)]
pub struct FixSummary {
    pub total_fixes: usize,
    pub auto_fixable: usize,
    pub needs_review: usize,
    pub manual_only: usize,
}

// ── Machine-readable fix strategy types (fix-strategies.json) ───────────

/// A single from/to mapping within a consolidated fix strategy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappingEntry {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub component: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub prop: Option<String>,
}

/// A member-level mapping entry for structural migration strategies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberMappingEntry {
    pub old_name: String,
    pub new_name: String,
}

/// A single prop's migration mapping between a deprecated and v6 component.
///
/// Extends `MemberMappingEntry` with type information from the SD pipeline's
/// `old_component_prop_types` and `new_component_prop_types`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PropMigrationEntry {
    /// Prop name on the deprecated component.
    pub old_name: String,
    /// Prop name on the v6 replacement component.
    pub new_name: String,
    /// Full TypeScript type on the deprecated component (if available).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub old_type: Option<String>,
    /// Full TypeScript type on the v6 replacement component (if available).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub new_type: Option<String>,
    /// Whether the type changed between deprecated and v6.
    #[serde(default)]
    pub type_changed: bool,
}

/// Migration context for a deprecated component → v6 replacement.
///
/// Stored on family-level `FixStrategyEntry` entries to provide the LLM with
/// the complete old→new prop mapping, including type signatures for matching
/// props that changed type, new-only props with their types, and removed-only
/// props with no v6 equivalent.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeprecatedMigrationContext {
    /// Package the deprecated component was imported from.
    pub old_package: String,
    /// Package the v6 replacement is imported from.
    pub new_package: String,
    /// Props that exist on both old and new, with name mappings and types.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub matching_props: Vec<PropMigrationEntry>,
    /// Props that exist ONLY on the v6 component (not on deprecated).
    /// Map of prop_name → TypeScript type string.
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub new_props: BTreeMap<String, String>,
    /// Props that exist ONLY on the deprecated component (no v6 equivalent).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub removed_props: Vec<String>,
    /// Appearance/variant notes inferred from CSS modifier comparison.
    ///
    /// When the replacement component has a variant-like prop (e.g., `variant`)
    /// with union string values, and the deprecated component's CSS modifiers
    /// suggest a specific default appearance, this field provides guidance.
    ///
    /// Example: "Chip has no pf-m-outline modifier (outline is its default
    /// appearance). Add variant='outline' to Label to preserve visual parity."
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub appearance_notes: Vec<String>,
}

/// A machine-readable fix strategy entry.
///
/// For non-consolidated rules, `from`/`to` hold the single mapping.
/// For consolidated rules, `mappings` holds all individual mappings from the
/// merged rules, allowing the fix engine to apply all renames/removals.
/// For structural migration rules, `member_mappings` and `removed_members`
/// describe the member-level overlap between removed and replacement interfaces.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FixStrategyEntry {
    pub strategy: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub component: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub prop: Option<String>,
    /// All individual mappings when this strategy was merged from multiple rules.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub mappings: Vec<MappingEntry>,
    /// Structural migration: matching member mappings between removed and replacement.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub member_mappings: Vec<MemberMappingEntry>,
    /// Structural migration: member names only in the removed interface (no match).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub removed_members: Vec<String>,
    /// Structural migration: the replacement symbol name.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub replacement: Option<String>,
    /// Structural migration: overlap ratio between removed and replacement.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub overlap_ratio: Option<f64>,
    /// Dependency update: npm package name (e.g., "@patternfly/react-core").
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub package: Option<String>,
    /// Dependency update: new version range (e.g., "^6.1.0").
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub new_version: Option<String>,
    /// Dependency update: old package coordinate for proactive matching.
    /// When set, the fix engine can search manifest files for this coordinate
    /// even without kantra incidents (bypassing the kantra dependency dispatch).
    /// For namespace migrations: the old namespace (e.g., "javax.persistence").
    /// For coordinate renames: the old group:artifact (e.g., "org.hibernate:hibernate-core").
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub old_package: Option<String>,

    // ── Family migration fields ────────────────────────────────────────
    // Used by `FamilyMigration` strategy entries (keyed `family:<Name>`)
    // to describe the complete target component structure for a family.
    /// Target JSX structure template showing correct composition.
    /// Example: `<Modal ...>\n  <ModalHeader .../>\n  <ModalBody>...</ModalBody>\n</Modal>`
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub target_structure: Option<String>,
    /// Props that remain on the root component in the new version.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub retained_props: Vec<String>,
    /// Map of removed prop name → child component that now owns it (as a named prop).
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub prop_to_child: BTreeMap<String, String>,
    /// Props removed from the root that don't have an exact prop-name match
    /// on any child component. Maps prop name → description of where/how to
    /// migrate (e.g., "ModalFooter (as children)"). Unlike `prop_to_child`,
    /// these props typically become *children* of the target component or
    /// are removed entirely.
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub unmapped_removed_props: BTreeMap<String, String>,
    /// Child component names removed from the family (flattened into parent).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub removed_children: Vec<String>,
    /// Map of "Child.prop" → "Parent.prop" for child-to-prop migrations.
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub child_props_to_parent: BTreeMap<String, String>,
    /// Prop value changes: prop_name → list of old→new value mappings.
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub prop_value_changes: BTreeMap<String, Vec<MappingEntry>>,
    /// Prop type changes: prop_name → list of old→new type signature mappings.
    /// Captures cases where a prop's TypeScript type changed between versions
    /// (e.g., a callback signature gained an event parameter). `from` may be
    /// `None` when the old type is unknown (inherited prop that became explicit).
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub prop_type_changes: BTreeMap<String, Vec<MappingEntry>>,
    /// New imports needed after restructuring (child components to add).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub new_imports: Vec<String>,
    /// Imports to remove after restructuring (removed child components).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub removed_imports: Vec<String>,
    /// Import source package (e.g., "@patternfly/react-core").
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub import_source: Option<String>,

    // ── Deprecated migration context ──────────────────────────────────
    // Populated for families where a deprecated component has a v6 replacement.
    // Provides the complete old→new prop mapping with type signatures.
    /// Deprecated → v6 migration context with prop mappings, types, and
    /// new/removed prop lists. Present when the family includes a deprecated
    /// component that has a detected migration target.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub deprecated_migration: Option<DeprecatedMigrationContext>,

    /// The npm package (or equivalent) whose analysis produced this strategy.
    ///
    /// Set by the semver-analyzer after strategy generation so the fix-engine
    /// can group entries by originating package at merge time. For example,
    /// entries from `@patternfly/react-drag-drop` analysis carry that as
    /// `source_package`, allowing the fix-engine to connect them with
    /// `EnsureDependency` rules that target the same package.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub source_package: Option<String>,

    /// Patterns to exclude from this fix strategy's automated replacement.
    ///
    /// Used with `CssVariablePrefix` to prevent blind prefix swaps on
    /// CSS classes that were removed in the target version. When the
    /// matched text contains any of these patterns, the edit is skipped.
    /// The incident is still reported (as Manual) by a separate dead-class rule.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub exclude_patterns: Vec<String>,
}

impl FixStrategyEntry {
    /// Create a new strategy entry with only the strategy type set.
    pub fn new(strategy: &str) -> Self {
        Self {
            strategy: strategy.into(),
            ..Default::default()
        }
    }

    /// Create a Rename strategy with a single from/to pair.
    pub fn rename(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            strategy: "Rename".into(),
            from: Some(from.into()),
            to: Some(to.into()),
            ..Default::default()
        }
    }

    /// Create a strategy with from/to and a named strategy type.
    pub fn with_from_to(strategy: &str, from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            strategy: strategy.into(),
            from: Some(from.into()),
            to: Some(to.into()),
            ..Default::default()
        }
    }

    /// Create a RemoveProp strategy.
    pub fn remove_prop(component: impl Into<String>, prop: impl Into<String>) -> Self {
        Self {
            strategy: "RemoveProp".into(),
            component: Some(component.into()),
            prop: Some(prop.into()),
            ..Default::default()
        }
    }

    /// Create an LlmAssisted strategy enriched with structural migration data.
    pub fn structural_migration(
        removed_symbol: &str,
        replacement_symbol: &str,
        member_mappings: Vec<MemberMappingEntry>,
        removed_members: Vec<String>,
        overlap_ratio: f64,
    ) -> Self {
        Self {
            strategy: "LlmAssisted".into(),
            from: Some(removed_symbol.into()),
            to: Some(replacement_symbol.into()),
            member_mappings,
            removed_members,
            replacement: Some(replacement_symbol.into()),
            overlap_ratio: Some(overlap_ratio),
            ..Default::default()
        }
    }

    /// Create an EnsureDependency strategy: add if missing, update version if present.
    pub fn ensure_dependency(package: impl Into<String>, new_version: impl Into<String>) -> Self {
        Self {
            strategy: "EnsureDependency".into(),
            package: Some(package.into()),
            new_version: Some(new_version.into()),
            ..Default::default()
        }
    }

    /// Create an EnsureDependency strategy with old_package for proactive matching.
    ///
    /// `old_package` is used by the fix engine to search manifest files when
    /// kantra doesn't produce incidents for dependency conditions.
    pub fn ensure_dependency_with_old(
        package: impl Into<String>,
        new_version: impl Into<String>,
        old_package: impl Into<String>,
    ) -> Self {
        Self {
            strategy: "EnsureDependency".into(),
            package: Some(package.into()),
            new_version: Some(new_version.into()),
            old_package: Some(old_package.into()),
            ..Default::default()
        }
    }

    /// Convert to a MappingEntry (extracting the single mapping).
    pub fn to_mapping(&self) -> MappingEntry {
        MappingEntry {
            from: self.from.clone(),
            to: self.to.clone(),
            component: self.component.clone(),
            prop: self.prop.clone(),
        }
    }
}

// ── Summary computation ─────────────────────────────────────────────────

/// Compute fix summary counts from the final strategies map.
///
/// Categorizes each strategy by type:
/// - **auto_fixable**: deterministic codemods (Rename, CssVariablePrefix,
///   RemoveProp, ImportPathChange, EnsureDependency, PropValueChange)
/// - **needs_review**: partially automated or LLM-assisted fixes
///   (LlmAssisted, FamilyMigration, PropTypeChange, AnnotationParamRewrite)
/// - **manual_only**: everything else (Manual, unknown strategy types)
pub fn compute_fix_summary(strategies: &HashMap<String, FixStrategyEntry>) -> FixSummary {
    let total_fixes = strategies.len();
    let mut auto_fixable = 0usize;
    let mut needs_review = 0usize;
    let mut manual_only = 0usize;

    for entry in strategies.values() {
        match entry.strategy.as_str() {
            "Rename" | "CssVariablePrefix" | "RemoveProp" | "ImportPathChange"
            | "EnsureDependency" | "PropValueChange" => auto_fixable += 1,
            "LlmAssisted" | "FamilyMigration" | "PropTypeChange"
            | "AnnotationParamRewrite" => needs_review += 1,
            _ => manual_only += 1,
        }
    }

    FixSummary {
        total_fixes,
        auto_fixable,
        needs_review,
        manual_only,
    }
}

// ── IO helpers ──────────────────────────────────────────────────────────

/// Extract fix strategies from the final (post-consolidation) rules.
pub fn extract_fix_strategies(
    rules: &[crate::rule::KonveyorRule],
) -> HashMap<String, FixStrategyEntry> {
    rules
        .iter()
        .filter_map(|r| {
            r.fix_strategy
                .as_ref()
                .map(|s| (r.rule_id.clone(), s.clone()))
        })
        .collect()
}

/// Write fix strategies JSON to the fix-guidance directory.
pub fn write_fix_strategies(
    fix_dir: &Path,
    strategies: &HashMap<String, FixStrategyEntry>,
) -> Result<()> {
    let path = fix_dir.join("fix-strategies.json");
    let json =
        serde_json::to_string_pretty(strategies).context("Failed to serialize fix strategies")?;
    std::fs::write(&path, &json).with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

/// Priority for fix strategy type. Higher = more actionable.
pub fn strategy_priority(strategy: &str) -> u8 {
    match strategy {
        "Rename" => 5,
        "RemoveProp" => 4,
        "CssVariablePrefix" => 4,
        "AnnotationParamRewrite" => 4,
        "ImportPathChange" => 3,
        "PropValueChange" => 2,
        "PropTypeChange" => 2,
        "LlmAssisted" => 1,
        _ => 0,
    }
}
