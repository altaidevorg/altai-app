//! Schema-driven plugin UI declarations (package 073, PR 1).
//!
//! A plugin's UI is **a schema the host renders, not code the host
//! runs**. The vocabulary below is closed: a surface is a tree of typed
//! nodes, and the only interactive node (`Action`) names a host-side
//! operation, never a script. Everything a malicious or buggy plugin
//! could try ends at the vocabulary's edge — there is nothing in a
//! [`PluginUiNode`] to execute.
//!
//! Validation happens at registration, against the manifest's declared
//! capabilities: an action that would invoke a job is refused unless the
//! plugin declared `Jobs`, and a declaration at all is refused without
//! `PluginUi`. That is the static half of the 073 gate ("UI cannot
//! bypass worker capability checks"); the runtime half — the dispatch
//! path refusing what the capability check refuses, whatever the client
//! sent — is PR 2.
//!
//! Bounds make a declaration unambiguously renderable and cheap to
//! trust: a bounded tree cannot be a layout bomb, and a table whose
//! rows match its columns cannot render crooked. They are constants,
//! not configuration — a host that wants different limits is changing
//! the contract, not tuning it.

use crate::PluginCapability;
use serde::{Deserialize, Serialize};

/// How many surfaces one plugin may declare.
pub const MAX_SURFACES: usize = 16;
/// Deepest node nesting, counting the root as depth 1.
pub const MAX_DEPTH: usize = 8;
/// Total nodes one surface's tree may contain.
pub const MAX_NODES_PER_SURFACE: usize = 256;
/// Columns one table node may declare.
pub const MAX_TABLE_COLUMNS: usize = 16;
/// Rows one table node may contain.
pub const MAX_TABLE_ROWS: usize = 500;

/// A plugin's declared UI: named surfaces, each a tree of nodes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginUiDeclaration {
    pub surfaces: Vec<PluginUiSurface>,
}

/// One named, titled surface — the unit a host shows or hides.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginUiSurface {
    pub surface_id: String,
    pub title: String,
    /// The root of this surface's node tree.
    pub root: PluginUiNode,
}

/// One node of a surface tree. The closed vocabulary: rendering hints
/// and data, plus exactly one interactive node whose payload names a
/// host-side action. No markup, no scripts, no URLs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PluginUiNode {
    /// A titled group of children.
    Section {
        title: String,
        children: Vec<PluginUiNode>,
    },
    /// One read-only key/value line.
    Text {
        label: String,
        value: String,
    },
    /// A read-only table; every row must be as wide as `columns`.
    Table {
        columns: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    /// A labeled control that asks the host to run an action. The only
    /// interactive node; the host — never the schema — executes it.
    Action {
        label: String,
        action: PluginUiAction,
    },
}

/// What an [`Action`](PluginUiNode::Action) node asks the host to do.
/// Each variant names a capability the manifest must declare.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PluginUiAction {
    /// Invoke one background job by id (requires `Jobs`). The dispatch
    /// itself travels the 072 worker path — at most once, ever.
    InvokeJob { job_id: String },
}

impl PluginUiAction {
    /// The capability this action needs declared. Static, per variant:
    /// there is no action whose requirement is computed.
    pub fn required_capability(&self) -> PluginCapability {
        match self {
            Self::InvokeJob { .. } => PluginCapability::Jobs,
        }
    }
}

/// Typed UI-declaration validation failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginUiError {
    /// A declaration exists but the manifest did not declare `PluginUi`.
    MissingPluginUiCapability,
    /// A declaration with no surfaces.
    EmptyDeclaration,
    TooManySurfaces { count: usize },
    EmptySurfaceId,
    DuplicateSurfaceId { surface_id: String },
    EmptyTitle,
    /// Node nesting deeper than [`MAX_DEPTH`]; `depth` is the offending
    /// depth, counting the root as 1.
    DepthExceeded { depth: usize },
    /// More nodes than [`MAX_NODES_PER_SURFACE`]; `count` is the total.
    TooManyNodes { count: usize },
    /// A table row whose width differs from its columns.
    RowWidthMismatch {
        row_index: usize,
        expected: usize,
        found: usize,
    },
    TooManyTableColumns { count: usize },
    TooManyTableRows { count: usize },
    EmptyActionLabel,
    /// An action's target (e.g. `job_id`) is empty.
    EmptyActionTarget,
    /// The action names an operation whose capability the manifest did
    /// not declare.
    ActionCapabilityMissing { capability: PluginCapability },
}

impl PluginUiDeclaration {
    /// Validate the declaration against the capabilities of the
    /// manifest that carries it. Checks run in declaration order so
    /// Rust and the TS mirror report the same first error.
    pub fn validate(&self, capabilities: &[PluginCapability]) -> Result<(), PluginUiError> {
        if !capabilities.contains(&PluginCapability::PluginUi) {
            return Err(PluginUiError::MissingPluginUiCapability);
        }
        if self.surfaces.is_empty() {
            return Err(PluginUiError::EmptyDeclaration);
        }
        if self.surfaces.len() > MAX_SURFACES {
            return Err(PluginUiError::TooManySurfaces {
                count: self.surfaces.len(),
            });
        }
        let mut seen = std::collections::BTreeSet::new();
        for surface in &self.surfaces {
            if surface.surface_id.trim().is_empty() {
                return Err(PluginUiError::EmptySurfaceId);
            }
            if !seen.insert(surface.surface_id.clone()) {
                return Err(PluginUiError::DuplicateSurfaceId {
                    surface_id: surface.surface_id.clone(),
                });
            }
            if surface.title.trim().is_empty() {
                return Err(PluginUiError::EmptyTitle);
            }
            let count = node_count(&surface.root);
            if count > MAX_NODES_PER_SURFACE {
                return Err(PluginUiError::TooManyNodes { count });
            }
            validate_node(&surface.root, 1, capabilities)?;
        }
        Ok(())
    }
}

/// Total nodes in a surface's tree, root included.
fn node_count(node: &PluginUiNode) -> usize {
    1 + match node {
        PluginUiNode::Section { children, .. } => {
            children.iter().map(node_count).sum()
        }
        PluginUiNode::Text { .. } | PluginUiNode::Table { .. } | PluginUiNode::Action { .. } => 0,
    }
}

/// Walk one node and its children: depth, table shape, and action
/// rules. Node count is checked before the walk, so this pass never
/// recurses beyond a bounded tree.
fn validate_node(
    node: &PluginUiNode,
    depth: usize,
    capabilities: &[PluginCapability],
) -> Result<(), PluginUiError> {
    if depth > MAX_DEPTH {
        return Err(PluginUiError::DepthExceeded { depth });
    }
    match node {
        PluginUiNode::Section { children, .. } => {
            for child in children {
                validate_node(child, depth + 1, capabilities)?;
            }
        }
        PluginUiNode::Text { .. } => {}
        PluginUiNode::Table { columns, rows } => {
            if columns.len() > MAX_TABLE_COLUMNS {
                return Err(PluginUiError::TooManyTableColumns {
                    count: columns.len(),
                });
            }
            if rows.len() > MAX_TABLE_ROWS {
                return Err(PluginUiError::TooManyTableRows { count: rows.len() });
            }
            for (row_index, row) in rows.iter().enumerate() {
                if row.len() != columns.len() {
                    return Err(PluginUiError::RowWidthMismatch {
                        row_index,
                        expected: columns.len(),
                        found: row.len(),
                    });
                }
            }
        }
        PluginUiNode::Action { label, action } => {
            if label.trim().is_empty() {
                return Err(PluginUiError::EmptyActionLabel);
            }
            let required = action.required_capability();
            if !capabilities.contains(&required) {
                return Err(PluginUiError::ActionCapabilityMissing {
                    capability: required,
                });
            }
            match action {
                PluginUiAction::InvokeJob { job_id } => {
                    if job_id.trim().is_empty() {
                        return Err(PluginUiError::EmptyActionTarget);
                    }
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ui_capabilities() -> Vec<PluginCapability> {
        vec![PluginCapability::PluginUi, PluginCapability::Jobs]
    }

    fn surface(surface_id: &str, root: PluginUiNode) -> PluginUiSurface {
        PluginUiSurface {
            surface_id: surface_id.into(),
            title: "Surface".into(),
            root,
        }
    }

    fn declaration(roots: Vec<PluginUiNode>) -> PluginUiDeclaration {
        PluginUiDeclaration {
            surfaces: roots
                .into_iter()
                .enumerate()
                .map(|(index, root)| surface(&format!("surface_{index}"), root))
                .collect(),
        }
    }

    fn job_action(job_id: &str) -> PluginUiNode {
        PluginUiNode::Action {
            label: "Run".into(),
            action: PluginUiAction::InvokeJob {
                job_id: job_id.into(),
            },
        }
    }

    fn nested(depth: usize, leaf: PluginUiNode) -> PluginUiNode {
        // A left-spine chain `depth` levels deep, ending in `leaf`.
        let mut node = leaf;
        for _ in 0..depth.saturating_sub(1) {
            node = PluginUiNode::Section {
                title: "Level".into(),
                children: vec![node],
            };
        }
        node
    }

    #[test]
    fn a_declaration_round_trips_with_snake_case_tags() {
        let declaration = PluginUiDeclaration {
            surfaces: vec![PluginUiSurface {
                surface_id: "main".into(),
                title: "Demo".into(),
                root: PluginUiNode::Section {
                    title: "Overview".into(),
                    children: vec![
                        PluginUiNode::Text {
                            label: "Status".into(),
                            value: "idle".into(),
                        },
                        PluginUiNode::Table {
                            columns: vec!["Queue".into(), "Depth".into()],
                            rows: vec![vec!["default".into(), "3".into()]],
                        },
                        job_action("job_refresh"),
                    ],
                },
            }],
        };
        let json = serde_json::to_string(&declaration).unwrap();
        assert_eq!(
            json,
            r#"{"surfaces":[{"surface_id":"main","title":"Demo","root":{"type":"section","title":"Overview","children":[{"type":"text","label":"Status","value":"idle"},{"type":"table","columns":["Queue","Depth"],"rows":[["default","3"]]},{"type":"action","label":"Run","action":{"type":"invoke_job","job_id":"job_refresh"}}]}}]}"#
        );
        let parsed: PluginUiDeclaration = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, declaration);
    }

    #[test]
    fn a_sound_declaration_validates() {
        declaration(vec![nested(4, job_action("job_1"))])
            .validate(&ui_capabilities())
            .unwrap();
    }

    #[test]
    fn a_declaration_without_the_capability_is_refused() {
        assert_eq!(
            declaration(vec![PluginUiNode::Text {
                label: "Hi".into(),
                value: "there".into()
            }])
            .validate(&[PluginCapability::Jobs]),
            Err(PluginUiError::MissingPluginUiCapability)
        );
    }

    #[test]
    fn an_empty_or_oversized_declaration_is_refused() {
        let capabilities = ui_capabilities();
        assert_eq!(
            PluginUiDeclaration { surfaces: vec![] }.validate(&capabilities),
            Err(PluginUiError::EmptyDeclaration)
        );
        let oversized = PluginUiDeclaration {
            surfaces: (0..=MAX_SURFACES)
                .map(|index| surface(&format!("s_{index}"), PluginUiNode::Text {
                    label: "Hi".into(),
                    value: "there".into()
                }))
                .collect(),
        };
        assert_eq!(
            oversized.validate(&capabilities),
            Err(PluginUiError::TooManySurfaces {
                count: MAX_SURFACES + 1
            })
        );
    }

    #[test]
    fn surface_ids_must_be_present_and_unique_and_titles_non_empty() {
        let capabilities = ui_capabilities();
        let empty_id = PluginUiDeclaration {
            surfaces: vec![surface("  ", PluginUiNode::Text {
                label: "Hi".into(),
                value: "there".into(),
            })],
        };
        assert_eq!(
            empty_id.validate(&capabilities),
            Err(PluginUiError::EmptySurfaceId)
        );

        let duplicate = PluginUiDeclaration {
            surfaces: vec![
                surface("same", PluginUiNode::Text {
                    label: "Hi".into(),
                    value: "there".into(),
                }),
                surface("same", PluginUiNode::Text {
                    label: "Hi".into(),
                    value: "again".into(),
                }),
            ],
        };
        assert_eq!(
            duplicate.validate(&capabilities),
            Err(PluginUiError::DuplicateSurfaceId {
                surface_id: "same".into()
            })
        );

        let untitled = PluginUiDeclaration {
            surfaces: vec![PluginUiSurface {
                surface_id: "main".into(),
                title: "  ".into(),
                root: PluginUiNode::Text {
                    label: "Hi".into(),
                    value: "there".into(),
                },
            }],
        };
        assert_eq!(untitled.validate(&capabilities), Err(PluginUiError::EmptyTitle));
    }

    #[test]
    fn depth_and_node_budgets_are_enforced() {
        let capabilities = ui_capabilities();
        let leaf = || PluginUiNode::Text {
            label: "Hi".into(),
            value: "there".into(),
        };
        // A chain one level too deep reports the offending depth.
        assert_eq!(
            declaration(vec![nested(MAX_DEPTH + 1, leaf())]).validate(&capabilities),
            Err(PluginUiError::DepthExceeded {
                depth: MAX_DEPTH + 1
            })
        );
        // Exactly at the limit is fine.
        declaration(vec![nested(MAX_DEPTH, leaf())])
            .validate(&capabilities)
            .unwrap();

        // One node over budget: a flat fan wider than the budget.
        let wide = PluginUiNode::Section {
            title: "Wide".into(),
            children: vec![leaf(); MAX_NODES_PER_SURFACE],
        };
        assert_eq!(
            declaration(vec![wide]).validate(&capabilities),
            Err(PluginUiError::TooManyNodes {
                count: MAX_NODES_PER_SURFACE + 1
            })
        );
        // One under budget passes (root + 255 children).
        let fits = PluginUiNode::Section {
            title: "Wide".into(),
            children: vec![leaf(); MAX_NODES_PER_SURFACE - 1],
        };
        declaration(vec![fits]).validate(&capabilities).unwrap();
    }

    #[test]
    fn tables_must_match_their_columns_and_their_bounds() {
        let capabilities = ui_capabilities();
        let crooked = PluginUiNode::Table {
            columns: vec!["A".into(), "B".into()],
            rows: vec![vec!["only-a".into()]],
        };
        assert_eq!(
            declaration(vec![crooked]).validate(&capabilities),
            Err(PluginUiError::RowWidthMismatch {
                row_index: 0,
                expected: 2,
                found: 1
            })
        );
        let too_wide = PluginUiNode::Table {
            columns: vec!["C".into(); MAX_TABLE_COLUMNS + 1],
            rows: vec![],
        };
        assert_eq!(
            declaration(vec![too_wide]).validate(&capabilities),
            Err(PluginUiError::TooManyTableColumns {
                count: MAX_TABLE_COLUMNS + 1
            })
        );
        let too_long = PluginUiNode::Table {
            columns: vec!["C".into()],
            rows: vec![vec!["r".into()]; MAX_TABLE_ROWS + 1],
        };
        assert_eq!(
            declaration(vec![too_long]).validate(&capabilities),
            Err(PluginUiError::TooManyTableRows {
                count: MAX_TABLE_ROWS + 1
            })
        );
    }

    #[test]
    fn actions_need_a_label_a_target_and_their_capability() {
        let capabilities = ui_capabilities();
        let unlabeled = PluginUiNode::Action {
            label: " ".into(),
            action: PluginUiAction::InvokeJob {
                job_id: "job_1".into(),
            },
        };
        assert_eq!(
            declaration(vec![unlabeled]).validate(&capabilities),
            Err(PluginUiError::EmptyActionLabel)
        );
        let targetless = PluginUiNode::Action {
            label: "Run".into(),
            action: PluginUiAction::InvokeJob {
                job_id: "".into(),
            },
        };
        assert_eq!(
            declaration(vec![targetless]).validate(&capabilities),
            Err(PluginUiError::EmptyActionTarget)
        );
        // PluginUi alone, no Jobs: the invoke-job action is refused even
        // though the declaration itself is otherwise sound.
        assert_eq!(
            declaration(vec![job_action("job_1")]).validate(&[PluginCapability::PluginUi]),
            Err(PluginUiError::ActionCapabilityMissing {
                capability: PluginCapability::Jobs
            })
        );
    }
}
