export type TaskOutcomeProps = {
  changesCount: number;
  checksPassed: number;
  checksFailed: number;
};

/**
 * Compact outcome summary shown at the bottom of a task run card: file change
 * count and verification check result. Purely presentational; the host
 * computes the counts from its run data.
 */
export function TaskOutcome({
  changesCount,
  checksPassed,
  checksFailed,
}: TaskOutcomeProps) {
  return (
    <div className="mt-2 flex flex-wrap items-center gap-x-2 gap-y-1 text-[9px] text-muted-foreground">
      {changesCount ? (
        <span>
          {changesCount} file{changesCount === 1 ? "" : "s"} changed
        </span>
      ) : (
        <span>No file changes</span>
      )}
      <span aria-hidden="true">·</span>
      {checksFailed ? (
        <span className="font-medium text-destructive">
          {checksFailed} check{checksFailed === 1 ? "" : "s"} failed
        </span>
      ) : checksPassed ? (
        <span className="font-medium text-success">
          {checksPassed} check{checksPassed === 1 ? "" : "s"} passed
        </span>
      ) : (
        <span>No checks reported</span>
      )}
    </div>
  );
}
