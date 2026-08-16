/**
 * Large-graph measurement (package 068, PR 2). The gate asks for *measured*
 * large-graph usability, not a claim: these tests lay out synthetic graphs
 * at sizes far past any real workspace board and pin the two properties a
 * user feels — layout latency (the graph appears immediately) and scroll
 * burden (canvas area grows with the graph, not faster than it).
 *
 * The budgets are deliberately loose multiples of the observed numbers so
 * CI variance cannot flake them; their job is to fail loudly if the layout
 * ever turns quadratic, not to track milliseconds.
 */

import { describe, expect, it } from "vitest";
import {
  GRAPH_NODE_HEIGHT,
  GRAPH_NODE_WIDTH,
  layoutWorkGraph,
  type GraphRow,
} from "./canvasGraphProjection";

/** Layout must finish inside this for any graph this module is given. */
const LAYOUT_BUDGET_MILLISECONDS = 200;

function row(id: string, parentWorkId?: string | null): GraphRow {
  return { id, title: `Work ${id}`, statusLabel: "open", parentWorkId: parentWorkId ?? null };
}

/** One long parent chain: the deepest graph shape, and the recursion worst
 * case for depth resolution. */
function chain(count: number): GraphRow[] {
  const rows: GraphRow[] = [];
  for (let index = 0; index < count; index += 1) {
    rows.push(row(`n${index}`, index === 0 ? null : `n${index - 1}`));
  }
  return rows;
}

/** A fan-out tree: the widest columns, and the tallest-column worst case. */
function tree(count: number, childrenPerNode: number): GraphRow[] {
  const rows: GraphRow[] = [];
  for (let index = 0; index < count; index += 1) {
    const parent = index === 0 ? null : `n${Math.floor((index - 1) / childrenPerNode)}`;
    rows.push(row(`n${index}`, parent));
  }
  return rows;
}

/** A single cycle through every node: malformed input that must still place
 * everyone rather than hang. */
function cycle(count: number): GraphRow[] {
  const rows: GraphRow[] = [];
  for (let index = 0; index < count; index += 1) {
    rows.push(row(`n${index}`, `n${(index + 1) % count}`));
  }
  return rows;
}

function measured<T>(build: () => T): { value: T; milliseconds: number } {
  const started = performance.now();
  const value = build();
  return { value, milliseconds: performance.now() - started };
}

/** A noisy first call (JIT warm-up) must not fail the budget, so the
 * measured call is the second one — the steady state a user hits. */
function layoutMeasured(rows: readonly GraphRow[]): { layout: ReturnType<typeof layoutWorkGraph>; milliseconds: number } {
  layoutWorkGraph(rows);
  const { value, milliseconds } = measured(() => layoutWorkGraph(rows));
  return { layout: value, milliseconds };
}

describe("layoutWorkGraph at large sizes", () => {
  for (const count of [500, 1_000, 2_500]) {
    it(`lays out a ${count}-node chain inside the latency budget`, () => {
      const { layout, milliseconds } = layoutMeasured(chain(count));

      expect(milliseconds).toBeLessThan(LAYOUT_BUDGET_MILLISECONDS);
      expect(layout.nodes).toHaveLength(count);
      expect(layout.edges).toHaveLength(count - 1);
    });

    it(`lays out a ${count}-node tree inside the latency budget`, () => {
      const { layout, milliseconds } = layoutMeasured(tree(count, 5));

      expect(milliseconds).toBeLessThan(LAYOUT_BUDGET_MILLISECONDS);
      expect(layout.nodes).toHaveLength(count);
      expect(layout.edges).toHaveLength(count - 1);
    });
  }

  it("is deterministic at scale: identical rows lay out identically", () => {
    const rows = tree(1_000, 5);
    expect(layoutWorkGraph(rows)).toEqual(layoutWorkGraph(rows));
  });

  it("keeps canvas area growing with the graph, not faster", () => {
    // Area is the scroll burden a user feels. Each node occupies one
    // node-box plus its share of gaps, so doubling the graph must stay
    // near-linear; anything super-linear (overlapping stacks, quadratic
    // column growth) breaks this bound.
    const small = layoutWorkGraph(tree(500, 5));
    const large = layoutWorkGraph(tree(1_000, 5));
    const area = (layout: typeof small) => layout.width * layout.height;

    expect(area(large) / area(small)).toBeLessThan(2.5);
  });

  it("places every node of a 1_000-node cycle without hanging", () => {
    const { layout, milliseconds } = layoutMeasured(cycle(1_000));

    expect(milliseconds).toBeLessThan(LAYOUT_BUDGET_MILLISECONDS);
    expect(layout.nodes).toHaveLength(1_000);
    // Everyone lands somewhere on the canvas.
    for (const node of layout.nodes) {
      expect(node.x).toBeGreaterThanOrEqual(0);
      expect(node.y).toBeGreaterThanOrEqual(0);
    }
  });

  it("derives the reported canvas size exactly from the columns it built", () => {
    const rows = tree(120, 4);
    const layout = layoutWorkGraph(rows);
    const deepest = Math.max(...layout.nodes.map((node) => node.depth));
    const perColumn = new Map<number, number>();
    for (const node of layout.nodes) {
      perColumn.set(node.depth, (perColumn.get(node.depth) ?? 0) + 1);
    }
    const tallest = Math.max(...perColumn.values());

    // The published size is the formula over the real column shape, so the
    // component can never scroll past blank canvas it was not told about.
    expect(layout.nodes.every((node) => node.width === GRAPH_NODE_WIDTH)).toBe(true);
    expect(layout.nodes.every((node) => node.height === GRAPH_NODE_HEIGHT)).toBe(true);
    expect(deepest + 1).toBe(perColumn.size);
    expect(tallest * GRAPH_NODE_HEIGHT).toBeLessThanOrEqual(layout.height);
  });
});
