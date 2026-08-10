/**
 * Pure Composer workspace-context menu and tool titles (A6.256).
 */

export type ComposerContextActionCopy = {
  label: string;
  detail: string;
};

export const COMPOSER_ATTACH_FILE_TITLE = "Attach file or image";

export const COMPOSER_ADD_WORKSPACE_CONTEXT_LABEL = "Add workspace context";

export const COMPOSER_RESEARCH_SEMBLE_TITLE = "Research with Semble Scout";

export const COMPOSER_CONTEXT_ACTIVE_FILE: ComposerContextActionCopy = {
  label: "Active file",
  detail: "Attach the file open in the editor",
};

export const COMPOSER_CONTEXT_WORKSPACE_MAP: ComposerContextActionCopy = {
  label: "Workspace file map",
  detail: "Attach a compact folder manifest",
};

export const COMPOSER_CONTEXT_ACTIVE_TERMINAL: ComposerContextActionCopy = {
  label: "Active terminal",
  detail: "Attach the latest non-private output",
};

export const COMPOSER_CONTEXT_WORKING_DIFF: ComposerContextActionCopy = {
  label: "Working tree diff",
  detail: "Attach unstaged Git changes",
};
