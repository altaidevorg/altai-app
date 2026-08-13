//! Postgres implementation of the CP-07 wake queue and checkout lease port.

use crate::{WakeError, WakeRepository};
use altai_control_protocol::{AttemptId, WakeRequest, WakeSource, WorkCheckoutLease, WorkItemId};
use postgres::{Client, NoTls};
use std::sync::Mutex;

/// Durable wake and checkout implementation for deployed control-plane mode.
///
/// `work_item_id` is the natural uniqueness boundary for both tables. The
/// enqueue statement merges sources inside one Postgres statement, so racing
/// wake producers cannot create duplicate wake rows. Checkout uses an atomic
/// insert with a primary-key conflict, so exactly one live owner is recorded.
pub struct PostgresWakeRepository {
    client: Mutex<Client>,
}

impl PostgresWakeRepository {
    pub fn connect(url: &str) -> Result<Self, String> {
        let mut client = Client::connect(url, NoTls).map_err(|error| error.to_string())?;
        client
            .batch_execute(
                "
                CREATE TABLE IF NOT EXISTS control_plane_wake_requests (
                    work_item_id TEXT PRIMARY KEY,
                    id TEXT NOT NULL UNIQUE,
                    sources JSONB NOT NULL,
                    requested_at TEXT NOT NULL,
                    claimed_at TEXT NULL
                );
                CREATE TABLE IF NOT EXISTS control_plane_work_checkout_leases (
                    work_item_id TEXT PRIMARY KEY,
                    owner_agent_instance_id TEXT NOT NULL,
                    attempt_id TEXT NOT NULL,
                    expires_at TEXT NOT NULL
                );
                ",
            )
            .map_err(|error| error.to_string())?;
        Ok(Self {
            client: Mutex::new(client),
        })
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Client>, WakeError> {
        self.client.lock().map_err(|_| WakeError::Internal {
            reason: "postgres wake lock poisoned".to_string(),
        })
    }

    fn database_error(error: postgres::Error) -> WakeError {
        WakeError::Internal {
            reason: error.to_string(),
        }
    }

    fn wake_from_row(
        row: &postgres::Row,
        work_item_id: WorkItemId,
    ) -> Result<WakeRequest, WakeError> {
        let sources: Vec<WakeSource> =
            serde_json::from_value(row.get("sources")).map_err(|error| WakeError::Internal {
                reason: format!("invalid persisted wake sources: {error}"),
            })?;
        Ok(WakeRequest {
            id: row.get("id"),
            work_item_id,
            sources,
            requested_at: row.get("requested_at"),
            claimed_at: row.get("claimed_at"),
        })
    }
}

impl WakeRepository for PostgresWakeRepository {
    fn enqueue(
        &self,
        work_item_id: WorkItemId,
        source: WakeSource,
        requested_at: String,
    ) -> Result<WakeRequest, WakeError> {
        let sources = serde_json::to_value(vec![source]).map_err(|error| WakeError::Internal {
            reason: error.to_string(),
        })?;
        let id = format!("wake_{}", work_item_id.value);
        let mut client = self.lock()?;
        let row = client
            .query_one(
                "
                INSERT INTO control_plane_wake_requests
                    (work_item_id, id, sources, requested_at, claimed_at)
                VALUES ($1, $2, $3, $4, NULL)
                ON CONFLICT (work_item_id) DO UPDATE
                SET sources = (
                    SELECT jsonb_agg(value ORDER BY value)
                    FROM (
                        SELECT DISTINCT value
                        FROM jsonb_array_elements_text(
                            control_plane_wake_requests.sources || EXCLUDED.sources
                        ) AS source(value)
                    ) AS distinct_sources
                )
                RETURNING id, sources, requested_at, claimed_at
                ",
                &[&work_item_id.value, &id, &sources, &requested_at],
            )
            .map_err(Self::database_error)?;
        Self::wake_from_row(&row, work_item_id)
    }

    fn checkout(&self, lease: WorkCheckoutLease) -> Result<(), WakeError> {
        let mut client = self.lock()?;
        let inserted = client
            .execute(
                "
                INSERT INTO control_plane_work_checkout_leases
                    (work_item_id, owner_agent_instance_id, attempt_id, expires_at)
                VALUES ($1, $2, $3, $4)
                ON CONFLICT (work_item_id) DO NOTHING
                ",
                &[
                    &lease.work_item_id.value,
                    &lease.owner_agent_instance_id.value,
                    &lease.attempt_id.value,
                    &lease.expires_at,
                ],
            )
            .map_err(Self::database_error)?;
        if inserted == 1 {
            Ok(())
        } else {
            Err(WakeError::ActiveCheckout {
                work_item_id: lease.work_item_id.value,
            })
        }
    }

    fn release_checkout(
        &self,
        work_item_id: &WorkItemId,
        attempt_id: &AttemptId,
    ) -> Result<(), WakeError> {
        let mut client = self.lock()?;
        let deleted = client
            .execute(
                "DELETE FROM control_plane_work_checkout_leases \
                 WHERE work_item_id = $1 AND attempt_id = $2",
                &[&work_item_id.value, &attempt_id.value],
            )
            .map_err(Self::database_error)?;
        if deleted == 1 {
            return Ok(());
        }
        let exists = client
            .query_opt(
                "SELECT 1 FROM control_plane_work_checkout_leases WHERE work_item_id = $1",
                &[&work_item_id.value],
            )
            .map_err(Self::database_error)?
            .is_some();
        Err(if exists {
            WakeError::ActiveCheckout {
                work_item_id: work_item_id.value.clone(),
            }
        } else {
            WakeError::NotFound {
                work_item_id: work_item_id.value.clone(),
            }
        })
    }
}
