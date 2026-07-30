import { useEffect } from "react";
import { useWorkspaceFolderStore } from "./folder";

/**
 * Hydrates optional project state before the agent-first shell mounts. A local
 * workspace is no longer required: chats can start project-free and attach a
 * local folder or GitHub repository only when the user chooses one.
 */
export function WorkspaceGate({ children }: { children: React.ReactNode }) {
  const hydrated = useWorkspaceFolderStore((s) => s.hydrated);
  const hydrate = useWorkspaceFolderStore((s) => s.hydrate);

  useEffect(() => {
    void hydrate();
  }, [hydrate]);

  // Keep the app visibly alive while we read persisted workspace state. If the
  // native store is slow or unavailable, folder.ts guarantees this resolves to
  // the project-free shell rather than leaving a black window behind.
  if (!hydrated) {
    return (
      <main className="flex h-screen w-screen items-center justify-center bg-background text-[13px] text-muted-foreground">
        Starting ALTAI…
      </main>
    );
  }

  return <>{children}</>;
}
