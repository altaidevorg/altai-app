/**
 * Workflow Editor component (C3) — edit WORKFLOW.md v2 config with
 * readiness scan, context pack preview, and check gate evaluation.
 */

import { useEffect, useState } from "react";
import { native, type ReadinessReport } from "@/modules/ai/lib/native";
import { cn } from "@/lib/utils";

function scoreColor(score: number): string {
  if (score >= 75) return "text-green-600";
  if (score >= 50) return "text-yellow-600";
  return "text-red-600";
}

function errorMessage(error: unknown): string {
  if (error instanceof Error && error.message.trim()) return error.message;
  if (typeof error === "string" && error.trim()) return error;
  return "Readiness scan failed.";
}

export function WorkflowEditor({ repoPath }: { repoPath: string }) {
  const [readiness, setReadiness] = useState<ReadinessReport | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    native
      .orchestrationReadinessScan(repoPath)
      .then((report) => {
        if (cancelled) return;
        setReadiness(report);
      })
      .catch((cause) => {
        console.error("Readiness scan failed:", cause);
        if (cancelled) return;
        setReadiness(null);
        setError(errorMessage(cause));
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [repoPath]);

  return (
    <div className="flex h-full flex-col overflow-y-auto p-4">
      <h2 className="mb-3 text-sm font-medium">Workflow Editor</h2>

      {/* Readiness scan */}
      <section className="mb-6">
        <h3 className="mb-2 text-xs font-medium text-muted-foreground">
          Agent Readiness
        </h3>
        {loading && (
          <p className="text-xs text-muted-foreground">Scanning…</p>
        )}
        {error && !loading && (
          <p className="text-xs text-destructive" role="alert">
            {error}
          </p>
        )}
        {readiness && !loading && (
          <div className="space-y-2">
            <div className="flex items-center gap-2">
              <span className="text-2xl font-bold">
                {readiness.overallScore}
              </span>
              <span className="text-xs text-muted-foreground">/ 100</span>
            </div>
            <div className="grid grid-cols-3 gap-2">
              {readiness.categories.map((cat) => (
                <div
                  key={cat.category}
                  className="rounded-md border p-2"
                >
                  <div className="flex items-center justify-between">
                    <span className="text-[10px] capitalize text-muted-foreground">
                      {cat.category.replace(/_/g, " ")}
                    </span>
                    <span
                      className={cn(
                        "text-sm font-bold",
                        scoreColor(cat.score),
                      )}
                    >
                      {cat.score}
                    </span>
                  </div>
                  {cat.notes.length > 0 && (
                    <p className="mt-1 text-[10px] text-muted-foreground">
                      {cat.notes[0]}
                    </p>
                  )}
                </div>
              ))}
            </div>
            {readiness.recommendations.length > 0 && (
              <ul className="space-y-1">
                {readiness.recommendations.slice(0, 5).map((rec, i) => (
                  <li
                    key={i}
                    className="text-[10px] text-yellow-600"
                  >
                    • {rec}
                  </li>
                ))}
              </ul>
            )}
          </div>
        )}
        {!readiness && !loading && !error && (
          <p className="text-xs text-muted-foreground">
            No readiness data. Set a valid repo path.
          </p>
        )}
      </section>
    </div>
  );
}
