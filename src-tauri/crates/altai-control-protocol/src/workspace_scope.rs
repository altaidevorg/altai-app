//! Fail-closed repository scoping. A [`RepositoryScope`] is an explicit,
//! org-scoped allowlist entry naming one repository URL the organization has
//! permitted. It has no identity of its own: the grant *is* the
//! (organization, URL) pair, so grants are insert-only and idempotent.
//! Absence of a grant is the denying condition — nothing is permitted by
//! default.

use crate::OrganizationId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryScope {
    pub organization_id: OrganizationId,
    pub repository_url: String,
    pub granted_at_unix_seconds: u64,
}
