import {
  createContext,
  useContext,
  useEffect,
  useRef,
  useState,
} from "react";
import {
  ACCEPTED_FILES,
  appendUniqueByKey,
  basenameForAttach,
  browserFileToAttachment,
  buildTextContextAttachment,
  classifyBrowserFile,
  estimateComposerContextTokens,
  getComposerActionAvailability,
  hasComposerDraft,
  hasNativeBinaryAttachment,
  MAX_TEXT_INLINE,
  planComposerSubmit,
  remainingTextAfterAcceptedDispatch,
  removeAcceptedItems,
  resolveComposerEnterAction,
  selectionToComposerAttachment,
  upsertComposerAttachment,
  type ComposerAction,
  type ComposerActionAvailability,
  type ComposerFileAttachment,
} from "@altai/agent-ui";
import { useWhisperRecording } from "../hooks/useWhisperRecording";
import { type Snippet } from "../lib/snippets";
import { tryRunSlashCommand, type SlashCommandMeta } from "./slashCommands";
import { native } from "./native";
import { sendMessage, stop as stopAgent, useChatStore } from "../store/chatStore";
import { useAgentRunsStore } from "../store/agentRunsStore";
import { useSnippetsStore } from "../store/snippetsStore";

/** @deprecated Prefer ComposerFileAttachment from @altai/agent-ui. */
export type FileAttachment = ComposerFileAttachment;

export { MAX_TEXT_INLINE, ACCEPTED_FILES };

type Voice = ReturnType<typeof useWhisperRecording>;

export type { ComposerAction, ComposerActionAvailability };
export {
  getComposerActionAvailability,
  remainingTextAfterAcceptedDispatch,
  resolveComposerEnterAction,
};

type ComposerCtx = {
  textareaRef: React.RefObject<HTMLTextAreaElement | null>;
  value: string;
  setValue: React.Dispatch<React.SetStateAction<string>>;
  files: FileAttachment[];
  addFiles: (list: FileList | null) => Promise<void>;
  /** Attach a file by absolute path — used by the file explorer's "Attach to Agent". */
  attachFileByPath: (path: string) => Promise<void>;
  attachFolderByPath: (path: string) => Promise<void>;
  /** Add a bounded, visible piece of runtime context (terminal output or diff). */
  addTextContext: (input: {
    kind: "terminal" | "diff" | "folder";
    name: string;
    text: string;
  }) => void;
  removeFile: (id: string) => void;
  pickedSnippets: Snippet[];
  addSnippet: (s: Snippet) => void;
  removeSnippet: (id: string) => void;
  pickedCommands: SlashCommandMeta[];
  addCommand: (c: SlashCommandMeta) => void;
  removeCommand: (name: string) => void;
  isBusy: boolean;
  isRunning: boolean;
  isCancelling: boolean;
  submit: () => void;
  steer: () => void;
  queueNext: () => void;
  stop: () => void;
  voice: Voice;
  canSend: boolean;
  canSteer: boolean;
  canQueue: boolean;
  actionAvailability: ComposerActionAvailability;
  contextTokenEstimate: number;
};

const Ctx = createContext<ComposerCtx | null>(null);

export function useComposer(): ComposerCtx {
  const ctx = useContext(Ctx);
  if (!ctx)
    throw new Error("useComposer must be used inside <AiComposerProvider>");
  return ctx;
}

type ProviderProps = {
  children: React.ReactNode;
};

export function AiComposerProvider({ children }: ProviderProps) {
  const sessionId = useChatStore((s) => s.activeSessionId);
  const status = useChatStore((s) => s.agentMeta.status);
  const runId = useAgentRunsStore((s) =>
    sessionId ? (s.runs[sessionId]?.runId ?? null) : null,
  );

  const [value, setValueState] = useState("");
  const valueRevision = useRef(0);
  const setValue: React.Dispatch<React.SetStateAction<string>> = (next) => {
    valueRevision.current += 1;
    setValueState(next);
  };
  const [files, setFiles] = useState<FileAttachment[]>([]);
  const [pickedSnippets, setPickedSnippets] = useState<Snippet[]>([]);
  const [pickedCommands, setPickedCommands] = useState<SlashCommandMeta[]>([]);
  const [submitting, setSubmitting] = useState(false);
  const submittingRef = useRef(false);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const addFiles = async (list: FileList | null) => {
    if (!list) return;
    const next: FileAttachment[] = [];
    for (const f of Array.from(list)) {
      const att = await readAttachment(f);
      if (att) next.push(att);
    }
    if (next.length) setFiles((prev) => [...prev, ...next]);
  };

  const focusSignal = useChatStore((s) => s.focusSignal);
  const pendingPrefill = useChatStore((s) => s.pendingPrefill);
  const consumePrefill = useChatStore((s) => s.consumePrefill);
  const pendingSelections = useChatStore((s) => s.pendingSelections);
  const consumeSelections = useChatStore((s) => s.consumeSelections);

  useEffect(() => {
    if (focusSignal === 0) return;
    textareaRef.current?.focus();
    if (pendingPrefill != null) {
      const text = consumePrefill();
      if (text) setValue((v) => (v ? `${text}${v}` : text));
    }
  }, [focusSignal, pendingPrefill, consumePrefill]);

  // Listen for explorer's "Attach to Agent" event.
  useEffect(() => {
    const onAttach = (e: Event) => {
      const path = (e as CustomEvent<string>).detail;
      if (typeof path === "string" && path.length > 0) {
        void attachFileByPath(path);
      }
    };
    window.addEventListener("altai:ai-attach-file", onAttach);
    return () => window.removeEventListener("altai:ai-attach-file", onAttach);
    // attachFileByPath is stable for our purposes (closes over setFiles only)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Tauri's webview surfaces OS file drags as normal HTML5 `FileList`s. Keep
  // this listener at document scope so a drop in either the workspace or the
  // chat attaches the files instead of letting the webview navigate away.
  useEffect(() => {
    const hasFiles = (event: DragEvent) =>
      Array.from(event.dataTransfer?.types ?? []).includes("Files");
    const onDragOver = (event: DragEvent) => {
      if (hasFiles(event)) event.preventDefault();
    };
    const onDrop = (event: DragEvent) => {
      if (!hasFiles(event) || !event.dataTransfer?.files.length) return;
      event.preventDefault();
      void addFiles(event.dataTransfer.files);
      useChatStore.getState().openMini();
      useChatStore.getState().focusInput();
    };
    document.addEventListener("dragover", onDragOver);
    document.addEventListener("drop", onDrop);
    return () => {
      document.removeEventListener("dragover", onDragOver);
      document.removeEventListener("drop", onDrop);
    };
    // addFiles only closes over React's stable setter.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    const onAttach = (e: Event) => {
      const path = (e as CustomEvent<string>).detail;
      if (typeof path === "string" && path.length > 0) void attachFolderByPath(path);
    };
    window.addEventListener("altai:ai-attach-folder", onAttach);
    return () => window.removeEventListener("altai:ai-attach-folder", onAttach);
    // attachFolderByPath only closes over stable setters.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (pendingSelections.length === 0) return;
    const drained = consumeSelections();
    if (drained.length === 0) return;
    setFiles((prev) => {
      const existing = new Set(prev.map((f) => f.id));
      const next: FileAttachment[] = [];
      for (const sel of drained) {
        if (existing.has(sel.id)) continue;
        next.push(
          selectionToComposerAttachment({
            id: sel.id,
            source: sel.source,
            text: sel.text,
          }),
        );
      }
      return next.length ? [...prev, ...next] : prev;
    });
  }, [pendingSelections, consumeSelections]);

  const voice = useWhisperRecording({
    onResult: (transcript: string) => {
      setValue((v) => (v ? `${v} ${transcript}` : transcript));
      requestAnimationFrame(() => textareaRef.current?.focus());
    },
  });

  const removeFile = (id: string) =>
    setFiles((prev) => prev.filter((f) => f.id !== id));

  const addSnippet = (s: Snippet) =>
    setPickedSnippets((prev) => appendUniqueByKey(prev, s, (x) => x.id));
  const removeSnippet = (id: string) =>
    setPickedSnippets((prev) => prev.filter((s) => s.id !== id));

  const addCommand = (cmd: SlashCommandMeta) =>
    setPickedCommands((prev) => appendUniqueByKey(prev, cmd, (x) => x.name));
  const removeCommand = (name: string) =>
    setPickedCommands((prev) => prev.filter((c) => c.name !== name));

  const attachFileByPath = async (path: string) => {
    try {
      if (/\.pdf$/i.test(path)) {
        // Workspace files are still converted to text so they remain readable
        // without a provider-specific file API. Browser-uploaded PDFs below
        // keep their original bytes and go to document-capable models directly.
        const result = await native.extractPdfPath(path);
        const name = basenameForAttach(path);
        const id = `path-${path}`;
        setFiles((prev) => prev.some((f) => f.id === id) ? prev : [...prev, {
          id, name, kind: "text", mediaType: "application/pdf",
          text: result.content, size: result.content.length,
        }]);
        useChatStore.getState().openMini();
        useChatStore.getState().focusInput();
        return;
      }
      const result = await native.readFile(path);
      if (result.kind !== "text") {
        // Binary/oversize files: skip (could surface a toast in future).
        console.warn("attachFileByPath: skipped non-text file", path, result);
        return;
      }
      const name = basenameForAttach(path);
      const id = `path-${path}`;
      setFiles((prev) => {
        if (prev.some((f) => f.id === id)) return prev;
        const att: FileAttachment = {
          id,
          name,
          kind: "text",
          mediaType: "text/plain",
          text: result.content,
          size: result.size,
        };
        return [...prev, att];
      });
      // Open the AI panel before focusing: on narrow windows it is an overlay,
      // so focusing a hidden composer made “Attach to Agent” look unresponsive.
      useChatStore.getState().openMini();
      useChatStore.getState().focusInput();
    } catch (e) {
      console.error("attachFileByPath failed:", e);
    }
  };

  const attachFolderByPath = async (path: string) => {
    try {
      const result = await native.listWorkspaceFiles(path);
      const files = result.files.slice(0, 500);
      const manifest = files.length ? files.map((file) => `- ${file}`).join("\n") : "(No files found)";
      const suffix = result.truncated ? "\n…[file list truncated]" : "";
      const name = basenameForAttach(path);
      addTextContext({ kind: "folder", name, text: `${manifest}${suffix}` });
    } catch (error) {
      console.error("attachFolderByPath failed:", error);
    }
  };

  const addTextContext = (input: {
    kind: "terminal" | "diff" | "folder";
    name: string;
    text: string;
  }) => {
    const attachment = buildTextContextAttachment(input);
    if (!attachment) return;
    setFiles((prev) => upsertComposerAttachment(prev, attachment));
    useChatStore.getState().openMini();
    useChatStore.getState().focusInput();
  };

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
    runId,
    submitting,
  });

  const clearAcceptedSnapshot = (snapshot: {
    valueRevision: number;
    value: string;
    files: FileAttachment[];
    snippets: Snippet[];
    commands: SlashCommandMeta[];
  }) => {
    setValueState((current) =>
      remainingTextAfterAcceptedDispatch(
        current,
        snapshot.value,
        valueRevision.current === snapshot.valueRevision,
      ),
    );
    setFiles((current) => removeAcceptedItems(current, snapshot.files));
    setPickedSnippets((current) =>
      removeAcceptedItems(current, snapshot.snippets),
    );
    setPickedCommands((current) =>
      removeAcceptedItems(current, snapshot.commands),
    );
  };

  const dispatch = async (action: ComposerAction) => {
    if (submittingRef.current) return;

    const snapshot = {
      valueRevision: valueRevision.current,
      value,
      files,
      snippets: pickedSnippets,
      commands: pickedCommands,
    };

    const plan = planComposerSubmit({
      action,
      availability: actionAvailability,
      draft: {
        value,
        files,
        snippets: pickedSnippets,
        commands: pickedCommands,
      },
      catalog: useSnippetsStore.getState().snippets,
      resolveSlash: tryRunSlashCommand,
    });

    if (plan.kind === "noop") return;
    if (plan.kind === "handled") {
      clearAcceptedSnapshot(snapshot);
      if (plan.toast) console.info(plan.toast);
      return;
    }

    if (!sessionId) return;
    const store = useChatStore.getState();
    const { composed, multimodal } = plan;
    const { imageUrls, documents } = multimodal;

    submittingRef.current = true;
    setSubmitting(true);
    try {
      let accepted: boolean;
      if (plan.action === "steer") {
        if (!runId) return;
        const acknowledgement = await native.agentSteer(
          sessionId,
          runId,
          composed,
        );
        if (
          acknowledgement.chatId !== sessionId ||
          acknowledgement.runId !== runId
        ) {
          throw new Error("The runtime acknowledged a different agent run");
        }
        store.appendNativeMessage(composed, "user");
        store.addActivity({
          label: "Steering queued for the active run",
          detail: "It will be applied at the next safe boundary",
          kind: "agent",
          tone: "success",
        });
        accepted = true;
      } else {
        accepted = await sendMessage(
          composed,
          imageUrls.length ? imageUrls : undefined,
          documents.length ? documents : undefined,
          { queue: plan.action === "queue" },
        );
      }

      if (accepted) {
        if (!store.mini.open) store.openMini();
        clearAcceptedSnapshot(snapshot);
      }
    } catch (error) {
      store.addActivity({
        label:
          plan.action === "steer"
            ? "Could not steer the active run"
            : "Task could not be queued",
        detail: error instanceof Error ? error.message : String(error),
        kind: "agent",
        tone: "error",
      });
    } finally {
      submittingRef.current = false;
      setSubmitting(false);
    }
  };

  const submit = () => void dispatch("send");
  const steer = () => void dispatch("steer");
  const queueNext = () => void dispatch("queue");

  const stop = () => {
    if (!actionAvailability.isCancelling) stopAgent();
  };

  const { isBusy, isRunning, isCancelling, canSend, canSteer, canQueue } =
    actionAvailability;
  const contextTokenEstimate = estimateComposerContextTokens({
    files,
    snippets: pickedSnippets,
  });

  const ctx: ComposerCtx = {
    textareaRef,
    value,
    setValue,
    files,
    addFiles,
    attachFileByPath,
    attachFolderByPath,
    addTextContext,
    removeFile,
    pickedSnippets,
    addSnippet,
    removeSnippet,
    pickedCommands,
    addCommand,
    removeCommand,
    isBusy,
    isRunning,
    isCancelling,
    submit,
    steer,
    queueNext,
    stop,
    voice,
    canSend,
    canSteer,
    canQueue,
    actionAvailability,
    contextTokenEstimate,
  };

  return <Ctx.Provider value={ctx}>{children}</Ctx.Provider>;
}

async function readAttachment(file: File): Promise<FileAttachment | null> {
  const cls = classifyBrowserFile(file);
  if (!cls.ok) return null;
  if (cls.kind === "image" || cls.kind === "pdf") {
    const url = await readAsDataURL(file);
    return browserFileToAttachment(cls, file.name, {
      url,
      size: file.size,
    });
  }
  const text = await file.text();
  return browserFileToAttachment(cls, file.name, {
    text,
    size: file.size,
  });
}

function readAsDataURL(file: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result ?? ""));
    reader.onerror = () => reject(reader.error);
    reader.readAsDataURL(file);
  });
}
