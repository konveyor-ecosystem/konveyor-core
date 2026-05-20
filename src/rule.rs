//! Konveyor rule definition types.
//!
//! These types represent the YAML rule format that Konveyor/kantra consumes.
//! They are used by the semver-analyzer to generate migration rules and can
//! be consumed by any tool that produces Konveyor-compatible rulesets.

use crate::fix::FixStrategyEntry;
use serde::{Deserialize, Serialize};

/// Ruleset metadata (written to `ruleset.yaml`).
#[derive(Debug, Serialize, Deserialize)]
pub struct KonveyorRuleset {
    pub name: String,
    pub description: String,
    pub labels: Vec<String>,
}

/// A single Konveyor rule.
#[derive(Debug, Serialize, Deserialize)]
pub struct KonveyorRule {
    #[serde(rename = "ruleID")]
    pub rule_id: String,
    pub labels: Vec<String>,
    pub effort: u32,
    pub category: String,
    pub description: String,
    pub message: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub links: Vec<KonveyorLink>,
    pub when: KonveyorCondition,
    /// Fix strategy for this rule. Not serialized to kantra YAML -- written
    /// separately to fix-strategies.json after consolidation.
    #[serde(skip)]
    pub fix_strategy: Option<FixStrategyEntry>,
}

/// A hyperlink attached to a rule.
#[derive(Debug, Serialize, Deserialize)]
pub struct KonveyorLink {
    pub url: String,
    pub title: String,
}

/// A Konveyor `when` condition.
///
/// Supports `builtin.filecontent` (regex), `builtin.json` (xpath),
/// `frontend.referenced` (AST-level, requires a frontend-analyzer-provider),
/// and `or`/`and` combinators.
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum KonveyorCondition {
    FileContent {
        #[serde(rename = "builtin.filecontent")]
        filecontent: FileContentFields,
    },
    Json {
        #[serde(rename = "builtin.json")]
        json: JsonFields,
    },
    FrontendReferenced {
        #[serde(rename = "frontend.referenced")]
        referenced: FrontendReferencedFields,
    },
    FrontendCssClass {
        #[serde(rename = "frontend.cssclass")]
        cssclass: FrontendPatternFields,
    },
    FrontendCssVar {
        #[serde(rename = "frontend.cssvar")]
        cssvar: FrontendPatternFields,
    },
    FrontendDependency {
        #[serde(rename = "frontend.dependency")]
        dependency: FrontendDependencyFields,
    },
    JavaReferenced {
        #[serde(rename = "javav2.referenced", alias = "java.referenced")]
        referenced: JavaReferencedFields,
    },
    JavaDependency {
        #[serde(rename = "javav2.dependency", alias = "java.dependency")]
        dependency: JavaDependencyFields,
    },
    Or {
        or: Vec<KonveyorCondition>,
    },
    And {
        and: Vec<KonveyorCondition>,
    },
    /// Negated `builtin.filecontent`: matches when the pattern is NOT found.
    FileContentNegated {
        #[serde(rename = "not")]
        negated: bool,
        #[serde(rename = "builtin.filecontent")]
        filecontent: FileContentFields,
    },
}

/// Fields for `frontend.cssclass` and `frontend.cssvar` conditions.
#[derive(Debug, Serialize, Deserialize)]
pub struct FrontendPatternFields {
    pub pattern: String,
    /// File path regex filter. Only scan files whose path matches this pattern.
    #[serde(
        rename = "filePattern",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub file_pattern: Option<String>,
}

/// Fields for a `frontend.dependency` condition.
///
/// Matches dependencies in package.json by name and optional version bounds.
/// The provider checks `dependencies`, `devDependencies`, and `peerDependencies`.
#[derive(Debug, Serialize, Deserialize)]
pub struct FrontendDependencyFields {
    /// Exact dependency name (e.g., `@patternfly/react-core`).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<String>,
    /// Regex pattern for dependency name.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub nameregex: Option<String>,
    /// Match dependencies with version <= this bound.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub upperbound: Option<String>,
    /// Match dependencies with version >= this bound.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub lowerbound: Option<String>,
}

/// Fields for a `builtin.filecontent` condition.
#[derive(Debug, Serialize, Deserialize)]
pub struct FileContentFields {
    pub pattern: String,
    #[serde(rename = "filePattern")]
    pub file_pattern: String,
}

/// Fields for a `builtin.json` condition.
#[derive(Debug, Serialize, Deserialize)]
pub struct JsonFields {
    pub xpath: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub filepaths: Option<Vec<String>>,
}

/// Fields for a `frontend.referenced` condition.
///
/// This condition requires a frontend-analyzer-provider gRPC server.
/// It performs AST-level symbol matching with location discriminators.
#[derive(Debug, Serialize, Deserialize)]
pub struct FrontendReferencedFields {
    /// Regex pattern for the symbol name.
    pub pattern: String,
    /// Where to look: IMPORT, JSX_COMPONENT, JSX_PROP, FUNCTION_CALL, TYPE_REFERENCE.
    pub location: String,
    /// Filter JSX props to only those on this component (regex).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub component: Option<String>,
    /// Filter JSX components to only those inside this parent (regex).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub parent: Option<String>,
    /// Negative parent filter: only match when parent does NOT match this pattern.
    /// Used for conformance rules (e.g., "ModalHeader must be inside Modal").
    #[serde(rename = "notParent", skip_serializing_if = "Option::is_none", default)]
    pub not_parent: Option<String>,
    /// Filter by the parent component's import source (regex).
    #[serde(
        rename = "parentFrom",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub parent_from: Option<String>,
    /// Positive child filter: only match the component (via `pattern`) if it
    /// has at least one direct JSX child whose name matches this regex. The
    /// incident is emitted on the parent component. Used for migration rules
    /// to detect old-style children still present (e.g., `child: ^ModalBox$`
    /// fires on `<Modal>` only when it still contains old internal components).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub child: Option<String>,
    /// Negative child filter: match the parent component (via `pattern`) and
    /// emit incidents for each direct JSX child whose name does NOT match this
    /// pattern. Used for "exclusive wrapper" rules (e.g., "all children of
    /// InputGroup must be InputGroupItem or InputGroupText").
    #[serde(rename = "notChild", skip_serializing_if = "Option::is_none", default)]
    pub not_child: Option<String>,
    /// Negative-existence child filter: match the component (via `pattern`)
    /// and emit an incident if NONE of its direct JSX children match this
    /// regex. Used for conformance rules like "AlertGroup must contain Alert."
    ///
    /// Inverse of `child` (which gates on existence). Complementary to
    /// `notChild` (which fires per non-matching child).
    #[serde(
        rename = "requiresChild",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub requires_child: Option<String>,
    /// Filter JSX prop values to only those matching this regex.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub value: Option<String>,
    /// Scope to imports from a specific package (e.g., `@patternfly/react-tokens`).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub from: Option<String>,
    /// File path regex filter. Only scan files whose path matches this pattern.
    /// e.g., `".*\\.(test|spec)\\.(ts|tsx|js|jsx)$"` to scope to test files.
    #[serde(
        rename = "filePattern",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub file_pattern: Option<String>,
}

/// Fields for a `java.referenced` condition.
///
/// All string fields accept standard regex patterns. The `scope` field
/// determines WHAT to search for (imports, type references, method calls,
/// etc.). All other fields act as composable filters that narrow results.
///
/// Requires a java-analyzer-provider gRPC server backed by the
/// `java-indexer` (tree-sitter-based AST indexing).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JavaReferencedFields {
    /// Regex pattern for the fully-qualified symbol name.
    /// e.g., `"org\\.springframework\\.boot\\.autoconfigure\\.cache\\..*"`
    pub pattern: String,

    /// Search scope: narrows WHERE to look. One of:
    ///   IMPORT, TYPE, METHOD_CALL, CONSTRUCTOR_CALL, ANNOTATION, DEFINITION.
    /// When absent, searches all scopes.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub scope: Option<String>,

    /// Filter: only match symbols imported from / belonging to this
    /// package (regex on FQN). e.g., `"org\\.springframework\\..*"`
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub from: Option<String>,

    /// Filter: only match symbols on/in types that extend this class
    /// (regex on FQN). For DEFINITION scope the matched type's own parent
    /// is checked; for usage scopes the target symbol's enclosing type's
    /// parent is checked.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub extends: Option<String>,

    /// Filter: only match symbols on/in types that implement this
    /// interface (regex on FQN).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub implements: Option<String>,

    /// Filter: only match symbols annotated with this annotation
    /// (regex on annotation FQN). Simple regex replacement for the
    /// previous `JavaAnnotatedFields` sub-condition.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub annotated: Option<String>,

    /// Filter: symbol kind — class, interface, record, enum, method,
    /// field, constructor, enum_member. Primarily narrows DEFINITION scope.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub kind: Option<String>,

    /// Filter: method return type (regex on type name/FQN).
    /// Only applies to method-related matches.
    #[serde(
        rename = "returnType",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub return_type: Option<String>,

    /// Filter: method parameter type (regex on type name/FQN).
    /// Matches if ANY parameter type matches the pattern.
    #[serde(
        rename = "parameterType",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub parameter_type: Option<String>,

    /// File path regex filter. Only scan files whose path matches.
    #[serde(
        rename = "filePattern",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub file_pattern: Option<String>,
}

/// Fields for a `java.dependency` condition.
///
/// Checks whether the application has a Maven/Gradle dependency matching
/// the given name and optional version bounds.
#[derive(Debug, Serialize, Deserialize)]
pub struct JavaDependencyFields {
    /// Dependency coordinate (e.g., `org.springframework.boot.spring-boot-starter-web`).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<String>,
    /// Regex pattern for dependency name.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub nameregex: Option<String>,
    /// Match dependencies with version <= this bound.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub upperbound: Option<String>,
    /// Match dependencies with version >= this bound.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub lowerbound: Option<String>,
}

/// Extract all `FrontendReferencedFields` from a `KonveyorCondition`,
/// recursing into `Or`/`And` combinators.
pub fn extract_frontend_refs(condition: &KonveyorCondition) -> Vec<&FrontendReferencedFields> {
    match condition {
        KonveyorCondition::FrontendReferenced { referenced } => vec![referenced],
        KonveyorCondition::Or { or } => or.iter().flat_map(extract_frontend_refs).collect(),
        KonveyorCondition::And { and } => and.iter().flat_map(extract_frontend_refs).collect(),
        _ => vec![],
    }
}

/// Extract the file pattern from an existing condition (for reuse in consolidated rules).
pub fn extract_file_pattern_from_condition(condition: &KonveyorCondition) -> Option<String> {
    match condition {
        KonveyorCondition::FileContent { filecontent } => Some(filecontent.file_pattern.clone()),
        KonveyorCondition::Or { or } => or.first().and_then(extract_file_pattern_from_condition),
        _ => None,
    }
}

/// Deduplicate conditions by their JSON representation.
pub fn dedup_conditions(conditions: Vec<KonveyorCondition>) -> Vec<KonveyorCondition> {
    use std::collections::BTreeSet;
    let mut seen = BTreeSet::new();
    let mut unique = Vec::new();
    for cond in conditions {
        let key = serde_json::to_string(&cond).unwrap_or_default();
        if seen.insert(key) {
            unique.push(cond);
        }
    }
    unique
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frontend_dependency_serializes_to_yaml() {
        let condition = KonveyorCondition::FrontendDependency {
            dependency: FrontendDependencyFields {
                name: Some("@patternfly/react-core".into()),
                nameregex: None,
                upperbound: Some("5.99.99".into()),
                lowerbound: None,
            },
        };
        let yaml = serde_yaml::to_string(&condition).unwrap();
        assert!(
            yaml.contains("frontend.dependency"),
            "Should serialize with frontend.dependency key"
        );
        assert!(yaml.contains("@patternfly/react-core"));
        assert!(yaml.contains("5.99.99"));
        // Optional None fields should not appear
        assert!(!yaml.contains("nameregex"));
        assert!(!yaml.contains("lowerbound"));
    }

    #[test]
    fn test_frontend_dependency_roundtrips() {
        let condition = KonveyorCondition::FrontendDependency {
            dependency: FrontendDependencyFields {
                name: Some("@patternfly/react-core".into()),
                nameregex: None,
                upperbound: Some("5.99.99".into()),
                lowerbound: Some("4.0.0".into()),
            },
        };
        let yaml = serde_yaml::to_string(&condition).unwrap();
        let deserialized: KonveyorCondition = serde_yaml::from_str(&yaml).unwrap();
        match deserialized {
            KonveyorCondition::FrontendDependency { dependency } => {
                assert_eq!(dependency.name, Some("@patternfly/react-core".into()));
                assert_eq!(dependency.upperbound, Some("5.99.99".into()));
                assert_eq!(dependency.lowerbound, Some("4.0.0".into()));
                assert_eq!(dependency.nameregex, None);
            }
            _ => panic!("Should deserialize as FrontendDependency"),
        }
    }

    #[test]
    fn test_frontend_dependency_in_rule_yaml() {
        let rule = KonveyorRule {
            rule_id: "test-dep-rule".into(),
            labels: vec!["source=test".into()],
            effort: 1,
            category: "mandatory".into(),
            description: "Update dep".into(),
            message: "Update this dependency".into(),
            links: vec![],
            when: KonveyorCondition::FrontendDependency {
                dependency: FrontendDependencyFields {
                    name: Some("@patternfly/react-core".into()),
                    nameregex: None,
                    upperbound: Some("5.99.99".into()),
                    lowerbound: None,
                },
            },
            fix_strategy: None,
        };
        let yaml = serde_yaml::to_string(&rule).unwrap();
        assert!(yaml.contains("frontend.dependency:"));
        assert!(
            yaml.contains("name: '@patternfly/react-core'")
                || yaml.contains("name: \"@patternfly/react-core\"")
                || yaml.contains("name: '@patternfly/react-core'")
        );
    }
}
