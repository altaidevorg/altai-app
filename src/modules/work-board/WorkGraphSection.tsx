import type { WorkGraphModel, WorkGraphRef } from "./lib/workDetailProjection";

type Props = {
  graph: WorkGraphModel;
  onOpenWork: (workId: string) => void;
};

function Row({
  node,
  indent,
  onOpenWork,
}: {
  node: WorkGraphRef;
  indent: boolean;
  onOpenWork: (workId: string) => void;
}) {
  return (
    <li className={indent ? "ml-3" : undefined}>
      <button
        type="button"
        onClick={() => onOpenWork(node.id)}
        className="flex w-full items-baseline gap-2 rounded-md px-2 py-1.5 text-left transition-colors hover:bg-muted/50"
      >
        <span className="min-w-0 flex-1 truncate text-[12px] text-foreground">
          {node.title}
        </span>
        <span className="shrink-0 text-[10.5px] text-muted-foreground">
          {node.stateLabel}
        </span>
      </button>
    </li>
  );
}

/**
 * The graph slice of the Work detail surface (package 062, PR 2): the
 * focused Work's parent and children with each related item's status as its
 * own label. The 2D canvas board is package 068; this is the accessible
 * structural view.
 */
export function WorkGraphSection({ graph, onOpenWork }: Props) {
  if (!graph.parent && graph.children.length === 0) return null;
  return (
    <section className="shrink-0 border-t border-border-subtle px-3 py-3">
      <h3 className="mb-1 text-[10.5px] font-semibold uppercase tracking-wide text-muted-foreground">
        Graph
      </h3>
      <ul className="divide-y divide-border-subtle overflow-hidden rounded-lg border border-border">
        {graph.parent ? (
          <Row node={graph.parent} indent={false} onOpenWork={onOpenWork} />
        ) : null}
        {graph.children.map((child) => (
          <Row key={child.id} node={child} indent onOpenWork={onOpenWork} />
        ))}
      </ul>
    </section>
  );
}
