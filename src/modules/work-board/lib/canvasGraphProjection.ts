/**
 * Canvas graph layout (package 068, PR 1). The board's rows already carry
 * each Work's status, phase and attention as distinct fields (package 062);
 * this projection adds only the spatial arrangement a 2D canvas needs:
 * depth by parent chain, columns per depth, deterministic stacking within
 * a column, and edge paths from each parent's right edge to its child.
 *
 * Pure and testable: the canvas component consumes the layout, it never
 * computes geometry itself.
 */

import type { WorkBoardRow } from "./workBoardProjection";

export const GRAPH_NODE_WIDTH = 168;
export const GRAPH_NODE_HEIGHT = 44;
export const GRAPH_COLUMN_GAP = 56;
export const GRAPH_ROW_GAP = 12;
export const GRAPH_PADDING = 12;
const TITLE_INSET_X = 10;
const TITLE_INSET_Y = 15;

export type GraphNodeLayout = {
  id: string;
  title: string;
  statusLabel: string;
  /** Longest parent chain above this node; roots sit at depth 0. */
  depth: number;
  x: number;
  y: number;
  width: number;
  height: number;
};

export type GraphEdgeLayout = {
  fromId: string;
  toId: string;
  /** Cubic-bezier control points, parent right edge to child left edge. */
  path: {
    from: { x: number; y: number };
    c1: { x: number; y: number };
    c2: { x: number; y: number };
    to: { x: number; y: number };
  };
};

export type GraphLayout = {
  nodes: GraphNodeLayout[];
  edges: GraphEdgeLayout[];
  width: number;
  height: number;
};

export type GraphRow = Pick<WorkBoardRow, "id" | "title" | "statusLabel"> & {
  parentWorkId?: string | null;
};

/** Rows whose parent is absent (or not on the board) anchor depth 0. */
function isGraphRoot(row: GraphRow, ids: ReadonlySet<string>): boolean {
  return !row.parentWorkId || !ids.has(row.parentWorkId);
}

/**
 * Depth = 1 + the deepest parent's depth, computed recursively with a
 * resolving set so malformed cycles (a→b→a) settle instead of hanging;
 * a node visited while its own depth is still being resolved is treated
 * as resolved at its current stack depth, which keeps every node placed.
 */
function depthsById(rows: readonly GraphRow[]): Map<string, number> {
  const byId = new Map(rows.map((row) => [row.id, row] as const));
  const ids = new Set(byId.keys());
  const depths = new Map<string, number>();
  const resolving = new Set<string>();

  const resolve = (row: GraphRow, stack: number): number => {
    const known = depths.get(row.id);
    if (known !== undefined) return known;
    if (resolving.has(row.id)) return stack;
    resolving.add(row.id);
    let depth = 0;
    if (!isGraphRoot(row, ids)) {
      const parent = byId.get(row.parentWorkId as string);
      if (parent) depth = resolve(parent, stack + 1) + 1;
    }
    resolving.delete(row.id);
    depths.set(row.id, depth);
    return depth;
  };

  for (const row of rows) resolve(row, 0);
  return depths;
}

/**
 * Lay the board's Work out as a layered graph: one column per depth,
 * nodes stacked in stable (title, id) order within a column. Sizes come
 * from the module constants so the component and the tests agree.
 */
export function layoutWorkGraph(rows: readonly GraphRow[]): GraphLayout {
  const depths = depthsById(rows);

  const byDepth = new Map<number, GraphRow[]>();
  for (const row of rows) {
    const depth = depths.get(row.id) ?? 0;
    const column = byDepth.get(depth) ?? [];
    column.push(row);
    byDepth.set(depth, column);
  }

  const nodesById = new Map<string, GraphNodeLayout>();
  for (const [depth, column] of byDepth) {
    const ordered = [...column].sort(
      (a, b) => a.title.localeCompare(b.title) || a.id.localeCompare(b.id),
    );
    ordered.forEach((row, index) => {
      nodesById.set(row.id, {
        id: row.id,
        title: row.title,
        statusLabel: row.statusLabel,
        depth,
        x: GRAPH_PADDING + depth * (GRAPH_NODE_WIDTH + GRAPH_COLUMN_GAP),
        y: GRAPH_PADDING + index * (GRAPH_NODE_HEIGHT + GRAPH_ROW_GAP),
        width: GRAPH_NODE_WIDTH,
        height: GRAPH_NODE_HEIGHT,
      });
    });
  }

  const nodes = [...nodesById.values()].sort(
    (a, b) => a.depth - b.depth || a.y - b.y || a.id.localeCompare(b.id),
  );

  const ids = new Set(nodesById.keys());
  const edges: GraphEdgeLayout[] = [];
  for (const row of rows) {
    if (!row.parentWorkId || !ids.has(row.parentWorkId)) continue;
    const from = nodesById.get(row.parentWorkId);
    const to = nodesById.get(row.id);
    if (!from || !to) continue;
    const midX = (from.x + from.width + to.x) / 2;
    edges.push({
      fromId: from.id,
      toId: to.id,
      path: {
        from: { x: from.x + from.width, y: from.y + from.height / 2 },
        c1: { x: midX, y: from.y + from.height / 2 },
        c2: { x: midX, y: to.y + to.height / 2 },
        to: { x: to.x, y: to.y + to.height / 2 },
      },
    });
  }

  const columnCount = byDepth.size;
  const tallestColumn = Math.max(
    0,
    ...[...byDepth.values()].map((column) => column.length),
  );
  return {
    nodes,
    edges,
    width:
      columnCount === 0
        ? 0
        : GRAPH_PADDING * 2 +
          columnCount * GRAPH_NODE_WIDTH +
          (columnCount - 1) * GRAPH_COLUMN_GAP,
    height:
      tallestColumn === 0
        ? 0
        : GRAPH_PADDING * 2 +
          tallestColumn * GRAPH_NODE_HEIGHT +
          (tallestColumn - 1) * GRAPH_ROW_GAP,
  };
}

/** Top-left anchor for a node's title text. */
export function graphTitleAnchor(node: GraphNodeLayout): {
  x: number;
  y: number;
} {
  return { x: node.x + TITLE_INSET_X, y: node.y + TITLE_INSET_Y };
}
