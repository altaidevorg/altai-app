/**
 * Actor identity for mutations.
 *
 * Every mutation command includes an `Actor` that identifies who performed
 * the change. An actor is never inferred from a session ID or chat name; it
 * is always explicitly supplied by the authenticated caller.
 */

import type { TypedId } from "./ids.js";
import { OrganizationId } from "./ids.js";

/** A user identity scoped to an organization. Distinct from agent IDs. */
export type UserId = {
  readonly organization_id: TypedId;
  readonly value: string;
};

export function createUserId(organizationId: TypedId, value: string): UserId {
  return { organization_id: organizationId, value };
}

export type ActorKind = "user" | "agent" | "system" | "external";

/** Who performed a mutation. */
export type Actor =
  | {
      readonly kind: "user";
      readonly id: UserId;
      /** Display name at the time of the mutation (denormalized for audit). */
      readonly display_name: string;
    }
  | {
      readonly kind: "agent";
      readonly id: TypedId;
      /** The attempt that authorized this action, if applicable. */
      readonly attempt_id?: string;
    }
  | {
      readonly kind: "system";
      /** Free-form component name for diagnostics. */
      readonly component: string;
    }
  | {
      readonly kind: "external";
      readonly integration: string;
      readonly external_actor_id: string;
    };

export function actorKind(actor: Actor): ActorKind {
  return actor.kind;
}

export function actorOrganization(actor: Actor): TypedId | undefined {
  return actor.kind === "user" ? actor.id.organization_id : undefined;
}

/** Parse an unknown value into an Actor, rejecting malformed input. */
export function parseActor(input: unknown): Actor {
  if (typeof input !== "object" || input === null || Array.isArray(input)) {
    throw new Error("actor must be an object");
  }
  const candidate = input as { kind?: unknown };
  if (typeof candidate.kind !== "string") {
    throw new Error("actor missing 'kind' field");
  }
  switch (candidate.kind) {
    case "user": {
      const a = input as { id?: unknown; display_name?: unknown };
      if (typeof a.id !== "object" || a.id === null) {
        throw new Error("user actor requires 'id'");
      }
      const id = a.id as { organization_id?: unknown; value?: unknown };
      if (typeof id.value !== "string") throw new Error("user id requires 'value'");
      const organizationId = OrganizationId.parse(id.organization_id);
      if (typeof a.display_name !== "string") {
        throw new Error("user actor requires 'display_name'");
      }
      return {
        kind: "user",
        id: { organization_id: organizationId, value: id.value },
        display_name: a.display_name,
      };
    }
    case "agent": {
      const a = input as { id?: unknown; attempt_id?: unknown };
      if (typeof a.id !== "object" || a.id === null) {
        throw new Error("agent actor requires 'id'");
      }
      return {
        kind: "agent",
        id: a.id as TypedId,
        attempt_id:
          typeof a.attempt_id === "string" ? a.attempt_id : undefined,
      };
    }
    case "system": {
      const a = input as { component?: unknown };
      if (typeof a.component !== "string") {
        throw new Error("system actor requires 'component'");
      }
      return { kind: "system", component: a.component };
    }
    case "external": {
      const a = input as { integration?: unknown; external_actor_id?: unknown };
      if (typeof a.integration !== "string" || typeof a.external_actor_id !== "string") {
        throw new Error("external actor requires 'integration' and 'external_actor_id'");
      }
      return {
        kind: "external",
        integration: a.integration,
        external_actor_id: a.external_actor_id,
      };
    }
    default:
      throw new Error(`unknown actor kind: ${candidate.kind}`);
  }
}
