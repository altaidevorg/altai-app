import { emitAppEvent, listenAppEvent } from "@/lib/appEvent";
import { create } from "zustand";
import {
  loadSnippets,
  newSnippetId,
  saveSnippets,
  type Snippet,
} from "../lib/snippets";

const CHANGED_EVENT = "altai://ai-snippets-changed";

type State = {
  hydrated: boolean;
  snippets: Snippet[];
  hydrate: () => Promise<void>;
  upsert: (snippet: Snippet) => void;
  remove: (id: string) => void;
};

let initialized = false;

export const useSnippetsStore = create<State>((set, get) => ({
  hydrated: false,
  snippets: [],
  hydrate: async () => {
    if (initialized) return;
    initialized = true;
    set({ snippets: await loadSnippets(), hydrated: true });
    void listenAppEvent(CHANGED_EVENT, async () => {
      set({ snippets: await loadSnippets() });
    });
  },
  upsert: (snippet) => {
    const list = get().snippets;
    const idx = list.findIndex((s) => s.id === snippet.id);
    const next =
      idx === -1 ? [...list, snippet] : list.map((s) => (s.id === snippet.id ? snippet : s));
    set({ snippets: next });
    void saveSnippets(next).then(() => emitAppEvent(CHANGED_EVENT));
  },
  remove: (id) => {
    const next = get().snippets.filter((s) => s.id !== id);
    set({ snippets: next });
    void saveSnippets(next).then(() => emitAppEvent(CHANGED_EVENT));
  },
}));

export { newSnippetId };
