import { initialChatState, reduceChat, type ChatState } from "../views/chatModel.js";
import { isChatHostMessage, type ChatWebviewMessage } from "../views/chatMessages.js";

type VsCodeApi = { postMessage(message: ChatWebviewMessage): void; getState(): unknown; setState(state: unknown): void };

declare function acquireVsCodeApi(): VsCodeApi;

const vscode = acquireVsCodeApi();
const transcript = requiredElement<HTMLElement>("transcript");
const context = requiredElement<HTMLElement>("context");
const composer = requiredElement<HTMLTextAreaElement>("composer");
const send = requiredElement<HTMLButtonElement>("send");
const stop = requiredElement<HTMLButtonElement>("stop");
const status = requiredElement<HTMLElement>("status");
const logs = requiredElement<HTMLButtonElement>("logs");
let state = restoreState(vscode.getState());

send.addEventListener("click", submit);
stop.addEventListener("click", () => {
  if (state.activeRunId) vscode.postMessage({ type: "chat/stop", runId: state.activeRunId });
});
logs.addEventListener("click", () => vscode.postMessage({ type: "chat/openLogs" }));
composer.addEventListener("keydown", (event) => {
  if (event.key === "Enter" && !event.shiftKey && !event.isComposing) {
    event.preventDefault();
    submit();
  }
});
composer.addEventListener("input", updateControls);
window.addEventListener("message", (event: MessageEvent<unknown>) => {
  if (!isChatHostMessage(event.data)) return;
  state = reduceChat(state, { type: "hostMessage", message: event.data });
  render();
});

render();

function submit(): void {
  const prompt = composer.value.trim();
  if (!prompt || state.activeRunId || state.awaitingRun) return;
  composer.value = "";
  state = reduceChat(state, { type: "promptSent", prompt });
  vscode.postMessage({ type: "chat/send", prompt });
  render();
}

function render(): void {
  transcript.replaceChildren(...state.entries.map(renderEntry));
  context.replaceChildren(...state.contextItems.map(renderContextChip));
  if (state.draft !== undefined) composer.value = state.draft;
  status.textContent = state.status;
  status.dataset.tone = state.statusTone;
  updateControls();
  vscode.setState(state);
  transcript.scrollTop = transcript.scrollHeight;
}

function renderContextChip(item: ChatState["contextItems"][number]): HTMLElement {
  const chip = document.createElement("div");
  chip.className = "chip";
  chip.title = item.uri;
  const label = document.createElement("span");
  label.textContent = item.label;
  const remove = document.createElement("button");
  remove.type = "button";
  remove.textContent = "×";
  remove.setAttribute("aria-label", `Remove ${item.label} from context`);
  remove.addEventListener("click", () => vscode.postMessage({ type: "chat/removeContext", id: item.id }));
  chip.append(label, remove);
  return chip;
}

function updateControls(): void {
  const running = Boolean(state.awaitingRun || state.activeRunId);
  composer.disabled = running;
  send.disabled = running || !composer.value.trim();
  stop.disabled = !state.activeRunId;
}

function renderEntry(item: ChatState["entries"][number]): HTMLElement {
  const article = document.createElement("article");
  article.className = `message message-${item.kind}`;
  const label = document.createElement("div");
  label.className = "message-label";
  label.textContent = item.kind === "user" ? "You" : item.kind === "assistant" ? "ALTAI" : item.kind === "thinking" ? "Thinking" : "Activity";
  const content = document.createElement("div");
  content.className = "message-content";
  content.textContent = item.content;
  article.append(label, content);
  return article;
}

function restoreState(value: unknown): ChatState {
  // Persist only after reducer-owned values are created; an invalid previous
  // state must never become trusted input in a webview.
  if (typeof value !== "object" || value === null || !Array.isArray((value as { entries?: unknown }).entries)) return initialChatState;
  return initialChatState;
}

function requiredElement<T extends HTMLElement>(id: string): T {
  const element = document.getElementById(id);
  if (!element) throw new Error(`Missing element: ${id}`);
  return element as T;
}
