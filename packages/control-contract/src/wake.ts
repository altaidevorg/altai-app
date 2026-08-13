import type { TypedId } from "./ids.js";
export type WakeSource = "assignment" | "comment" | "mention" | "routine" | "approval_result" | "retry" | "recovery" | "manual";
export type WakeRequest = { readonly id: string; readonly work_item_id: TypedId; readonly sources: readonly WakeSource[]; readonly requested_at: string; readonly claimed_at: string | null };
/** Lease ownership is distinct from executor run binding and expires independently. */
export type WorkCheckoutLease = { readonly work_item_id: TypedId; readonly owner_agent_instance_id: TypedId; readonly attempt_id: TypedId; readonly expires_at: string };
