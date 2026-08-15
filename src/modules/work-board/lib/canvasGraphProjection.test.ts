import { describe, expect, it } from "vitest";
import {
  GRAPH_COLUMN_GAP,
  GRAPH_NODE_HEIGHT,
  GRAPH_NODE_WIDTH,
  GRAPH_PADDING,
  GRAPH_ROW_GAP,
  graphTitleAnchor,
  layoutWorkGraph,
  type GraphRow,
} from "./canvasGraphProjection";

function row(
  id: string,
  title: string,
  parentWorkId?: string | null,
): GraphRow {
  return { id, title, statusLabel: "open", parentWorkId: parentWorkId ?? null };
}

function node(layout: ReturnType<typeof layoutWorkGraph>, id: string) {
  const found = layout.nodes.find((node) => node.id === id);
  if (!found) throw new Error(`missing node ${id}`);
  return found;
}

describe("layoutWorkGraph", () => {
  it("places roots at depth 0 and descendants one column per chain step", () => {
    const layout = layoutWorkGraph([
      row("epic", "Epic"),
      row("task", "Task", "epic"),
      row("sub", "Sub-task", "task"),
    ]);

    expect(node(layout, "epic").depth).toBe(0);
    expect(node(layout, "task").depth).toBe(1);
    expect(node(layout, "sub").depth).toBe(2);
    expect(node(layout, "task").x).toBe(
      GRAPH_PADDING + GRAPH_NODE_WIDTH + GRAPH_COLUMN_GAP,
    );
  });

  it("gives a node the depth of its deepest parent chain", () => {
    const layout = layoutWorkGraph([
      row("a", "Root a"),
      row("b", "Root b"),
      row("mid", "Middle", "a"),
      row("child", "Shared child", "mid"),
    ]);

    expect(node(layout, "mid").depth).toBe(1);
    expect(node(layout, "child").depth).toBe(2);
  });

  it("treats a parent that is not on the board as a root anchor", () => {
    const layout = layoutWorkGraph([row("orphan", "Orphan", "missing")]);
    expect(node(layout, "orphan").depth).toBe(0);
    expect(layout.edges).toHaveLength(0);
  });

  it("stacks a column in stable (title, id) order with row gaps", () => {
    const layout = layoutWorkGraph([
      row("z", "Zeta"),
      row("a", "Alpha"),
      row("m", "Mid"),
    ]);

    const ys = layout.nodes.map((n) => n.y);
    expect(ys[0]).toBe(GRAPH_PADDING);
    expect(ys[1]).toBe(GRAPH_PADDING + GRAPH_NODE_HEIGHT + GRAPH_ROW_GAP);
    expect(ys[2]).toBe(
      GRAPH_PADDING + 2 * (GRAPH_NODE_HEIGHT + GRAPH_ROW_GAP),
    );
    expect(layout.nodes.map((n) => n.id)).toEqual(["a", "m", "z"]);
  });

  it("draws one edge per on-board parent link, parent right edge to child left edge", () => {
    const layout = layoutWorkGraph([
      row("epic", "Epic"),
      row("task", "Task", "epic"),
    ]);

    expect(layout.edges).toHaveLength(1);
    const edge = layout.edges[0]!;
    expect(edge.fromId).toBe("epic");
    expect(edge.toId).toBe("task");
    expect(edge.path.from.x).toBe(
      GRAPH_PADDING + GRAPH_NODE_WIDTH,
    );
    expect(edge.path.to.x).toBe(
      GRAPH_PADDING + GRAPH_NODE_WIDTH + GRAPH_COLUMN_GAP,
    );
    expect(edge.path.from.y).toBe(GRAPH_PADDING + GRAPH_NODE_HEIGHT / 2);
  });

  it("sizes the canvas to the deepest column and the tallest stack", () => {
    const layout = layoutWorkGraph([
      row("root", "Root"),
      row("one", "One", "root"),
      row("two", "Two", "root"),
    ]);

    expect(layout.width).toBe(
      GRAPH_PADDING * 2 + 2 * GRAPH_NODE_WIDTH + GRAPH_COLUMN_GAP,
    );
    expect(layout.height).toBe(
      GRAPH_PADDING * 2 + 2 * GRAPH_NODE_HEIGHT + GRAPH_ROW_GAP,
    );
  });

  it("returns a zero-size layout for an empty board", () => {
    const layout = layoutWorkGraph([]);
    expect(layout.nodes).toEqual([]);
    expect(layout.edges).toEqual([]);
    expect(layout.width).toBe(0);
    expect(layout.height).toBe(0);
  });

  it("still places every node when the parent chain is cyclic", () => {
    const layout = layoutWorkGraph([
      row("a", "Cycle a", "b"),
      row("b", "Cycle b", "a"),
      row("c", "Outside", "a"),
    ]);

    expect(layout.nodes.map((n) => n.id).sort()).toEqual(["a", "b", "c"]);
    const placed = layout.nodes.every(
      (n) => Number.isFinite(n.x) && Number.isFinite(n.y),
    );
    expect(placed).toBe(true);
  });

  it("anchors each node's title inside its card", () => {
    const layout = layoutWorkGraph([row("solo", "Solo")]);
    const solo = node(layout, "solo");
    const anchor = graphTitleAnchor(solo);
    expect(anchor.x).toBeGreaterThan(solo.x);
    expect(anchor.y).toBeGreaterThan(solo.y);
    expect(anchor.y).toBeLessThan(solo.y + solo.height);
  });
});
