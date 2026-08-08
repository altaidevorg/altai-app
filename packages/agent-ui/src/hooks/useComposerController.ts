/**
 * Headless ports-first composer controller (A6.36).
 * Owns draft state + submit dispatch via executeComposerSubmit.
 * Hosts inject send/steer/cancel; native attach/voice stay host-side.
 */

import {
  useCallback,
  useMemo,
  useRef,
  useState,
  type Dispatch,
  type SetStateAction,
} from "react";
import {
  buildTextContextAttachment,
  estimateComposerContextTokens,
  hasComposerDraft,
  hasNativeBinaryAttachment,
  upsertComposerAttachment,
  type ComposerFileAttachment,
} from "../lib/composerAttachments.js";
import { appendUniqueByKey } from "../lib/composerDraft.js";
import {
  getComposerActionAvailability,
  type ComposerAction,
  type ComposerActionAvailability,
} from "../lib/composerEnterAction.js";
import type { ComposerSnippet } from "../lib/composerSnippets.js";
import { clearComposerDraftAfterAccept } from "../lib/composerDraftClear.js";
import {
  executeComposerSubmit,
  type ComposerSubmitHostHandlers,
} from "../lib/composerSubmitExecute.js";
import type { ComposerSlashResolver } from "../lib/composerSubmitPlan.js";

export type ComposerCommandPick = { name: string };

export type UseComposerControllerOptions = {
  status: string;
  sessionId: string | null | undefined;
  runId: string | null | undefined;
  catalog: readonly ComposerSnippet[];
  resolveSlash?: ComposerSlashResolver;
  host: ComposerSubmitHostHandlers;
  /** When cancel is requested while already cancelling, host is not called. */
  cancel?: () => void;
};

export type ComposerController = {
  value: string;
  setValue: Dispatch<SetStateAction<string>>;
  files: ComposerFileAttachment[];
  setFiles: Dispatch<SetStateAction<ComposerFileAttachment[]>>;
  removeFile: (id: string) => void;
  addTextContext: (input: {
    kind: "terminal" | "diff" | "folder";
    name: string;
    text: string;
  }) => void;
  pickedSnippets: ComposerSnippet[];
  addSnippet: (s: ComposerSnippet) => void;
  removeSnippet: (id: string) => void;
  pickedCommands: ComposerCommandPick[];
  addCommand: (c: ComposerCommandPick) => void;
  removeCommand: (name: string) => void;
  submitting: boolean;
  actionAvailability: ComposerActionAvailability;
  contextTokenEstimate: number;
  canSend: boolean;
  canSteer: boolean;
  canQueue: boolean;
  isBusy: boolean;
  isRunning: boolean;
  isCancelling: boolean;
  submit: () => void;
  steer: () => void;
  queueNext: () => void;
  stop: () => void;
  dispatch: (action: ComposerAction) => Promise<void>;
};

export function useComposerController(
  options: UseComposerControllerOptions,
): ComposerController {
  const { status, runId } = options;

  const [value, setValueState] = useState("");
  const valueRevision = useRef(0);
  const setValue: Dispatch<SetStateAction<string>> = useCallback((next) => {
    valueRevision.current += 1;
    setValueState(next);
  }, []);

  const [files, setFiles] = useState<ComposerFileAttachment[]>([]);
  const [pickedSnippets, setPickedSnippets] = useState<ComposerSnippet[]>([]);
  const [pickedCommands, setPickedCommands] = useState<ComposerCommandPick[]>(
    [],
  );
  const [submitting, setSubmitting] = useState(false);
  const submittingRef = useRef(false);

  const valueRef = useRef(value);
  valueRef.current = value;
  const filesRef = useRef(files);
  filesRef.current = files;
  const snippetsRef = useRef(pickedSnippets);
  snippetsRef.current = pickedSnippets;
  const commandsRef = useRef(pickedCommands);
  commandsRef.current = pickedCommands;

  const optionsRef = useRef(options);
  optionsRef.current = options;

  const hasDraft = hasComposerDraft({
    value,
    files,
    snippets: pickedSnippets,
    commands: pickedCommands,
  });
  const actionAvailability = getComposerActionAvailability({
    status,
    hasDraft,
    hasNativeAttachment: hasNativeBinaryAttachment(files),
    runId: runId ?? null,
    submitting,
  });

  const clearAcceptedSnapshot = useCallback(
    (snapshot: {
      valueRevision: number;
      value: string;
      files: ComposerFileAttachment[];
      snippets: ComposerSnippet[];
      commands: ComposerCommandPick[];
    }) => {
      setValueState((current) =>
        clearComposerDraftAfterAccept(
          {
            valueRevision: valueRevision.current,
            value: current,
            files: snapshot.files,
            snippets: snapshot.snippets,
            commands: snapshot.commands,
          },
          snapshot,
        ).value,
      );
      setFiles((current) =>
        clearComposerDraftAfterAccept(
          {
            valueRevision: valueRevision.current,
            value: snapshot.value,
            files: current,
            snippets: snapshot.snippets,
            commands: snapshot.commands,
          },
          snapshot,
        ).files as ComposerFileAttachment[],
      );
      setPickedSnippets((current) =>
        clearComposerDraftAfterAccept(
          {
            valueRevision: valueRevision.current,
            value: snapshot.value,
            files: snapshot.files,
            snippets: current,
            commands: snapshot.commands,
          },
          snapshot,
        ).snippets as ComposerSnippet[],
      );
      setPickedCommands((current) =>
        clearComposerDraftAfterAccept(
          {
            valueRevision: valueRevision.current,
            value: snapshot.value,
            files: snapshot.files,
            snippets: snapshot.snippets,
            commands: current,
          },
          snapshot,
        ).commands as ComposerCommandPick[],
      );
    },
    [],
  );

  const dispatch = useCallback(
    async (action: ComposerAction) => {
      if (submittingRef.current) return;
      const snapshot = {
        valueRevision: valueRevision.current,
        value: valueRef.current,
        files: filesRef.current,
        snippets: snippetsRef.current,
        commands: commandsRef.current,
      };
      const o = optionsRef.current;
      const preflight = getComposerActionAvailability({
        status: o.status,
        hasDraft: hasComposerDraft(snapshot),
        hasNativeAttachment: hasNativeBinaryAttachment(snapshot.files),
        runId: o.runId ?? null,
        submitting: false,
      });

      submittingRef.current = true;
      setSubmitting(true);
      try {
        const result = await executeComposerSubmit({
          action,
          availability: preflight,
          draft: {
            value: snapshot.value,
            files: snapshot.files,
            snippets: snapshot.snippets,
            commands: snapshot.commands,
          },
          catalog: o.catalog,
          resolveSlash: o.resolveSlash,
          sessionId: o.sessionId,
          runId: o.runId,
          host: o.host,
        });
        if (result.kind === "handled" || result.kind === "accepted") {
          clearAcceptedSnapshot(snapshot);
        }
      } finally {
        submittingRef.current = false;
        setSubmitting(false);
      }
    },
    [clearAcceptedSnapshot],
  );

  const submit = useCallback(() => void dispatch("send"), [dispatch]);
  const steer = useCallback(() => void dispatch("steer"), [dispatch]);
  const queueNext = useCallback(() => void dispatch("queue"), [dispatch]);
  const stop = useCallback(() => {
    if (actionAvailability.isCancelling) return;
    optionsRef.current.cancel?.();
  }, [actionAvailability.isCancelling]);

  const removeFile = useCallback((id: string) => {
    setFiles((prev) => prev.filter((f) => f.id !== id));
  }, []);

  const addTextContext = useCallback(
    (input: {
      kind: "terminal" | "diff" | "folder";
      name: string;
      text: string;
    }) => {
      const attachment = buildTextContextAttachment(input);
      if (!attachment) return;
      setFiles((prev) => upsertComposerAttachment(prev, attachment));
    },
    [],
  );

  const addSnippet = useCallback((s: ComposerSnippet) => {
    setPickedSnippets((prev) => appendUniqueByKey(prev, s, (x) => x.id));
  }, []);
  const removeSnippet = useCallback((id: string) => {
    setPickedSnippets((prev) => prev.filter((s) => s.id !== id));
  }, []);
  const addCommand = useCallback((c: ComposerCommandPick) => {
    setPickedCommands((prev) => appendUniqueByKey(prev, c, (x) => x.name));
  }, []);
  const removeCommand = useCallback((name: string) => {
    setPickedCommands((prev) => prev.filter((c) => c.name !== name));
  }, []);

  const contextTokenEstimate = useMemo(
    () =>
      estimateComposerContextTokens({
        files,
        snippets: pickedSnippets,
      }),
    [files, pickedSnippets],
  );

  return {
    value,
    setValue,
    files,
    setFiles,
    removeFile,
    addTextContext,
    pickedSnippets,
    addSnippet,
    removeSnippet,
    pickedCommands,
    addCommand,
    removeCommand,
    submitting,
    actionAvailability,
    contextTokenEstimate,
    canSend: actionAvailability.canSend,
    canSteer: actionAvailability.canSteer,
    canQueue: actionAvailability.canQueue,
    isBusy: actionAvailability.isBusy,
    isRunning: actionAvailability.isRunning,
    isCancelling: actionAvailability.isCancelling,
    submit,
    steer,
    queueNext,
    stop,
    dispatch,
  };
}
