import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import {
  graphTitleAnchor,
  layoutWorkGraph,
  type GraphNodeLayout,
} from "./lib/canvasGraphProjection";
import type { WorkBoardRow } from "./lib/workBoardProjection";

type Props = {
  rows: WorkBoardRow[];
  onOpenWork: (id: string) => void;
};

const NODE_FILL = "#0b0b0d";
const NODE_STROKE = "#2a2a30";
const EDGE_STROKE = "#2a2a30";
const TITLE_FILL = "#ededf0";
const STATUS_FILL = "#8f8f99";
const TITLE_FONT = "500 12px ui-sans-serif, system-ui, sans-serif";
const STATUS_FONT = "400 10px ui-sans-serif, system-ui, sans-serif";
const TITLE_MAX_CHARS = 26;

function roundedRect(
  ctx: CanvasRenderingContext2D,
  node: GraphNodeLayout,
  radius: number,
): void {
  ctx.beginPath();
  ctx.roundRect(node.x, node.y, node.width, node.height, radius);
}

function hitTest(
  nodes: readonly GraphNodeLayout[],
  point: { x: number; y: number },
): GraphNodeLayout | null {
  for (let index = nodes.length - 1; index >= 0; index -= 1) {
    const node = nodes[index]!;
    if (
      point.x >= node.x &&
      point.x <= node.x + node.width &&
      point.y >= node.y &&
      point.y <= node.y + node.height
    ) {
      return node;
    }
  }
  return null;
}

/**
 * The 2D canvas Work board (package 068, PR 1): the board's rows laid out
 * as a layered parent/child graph. The geometry lives in the tested
 * `canvasGraphProjection`; this component only draws it DPR-aware and
 * hit-tests clicks. It stays a pointer surface — the column board is the
 * keyboard-accessible view of the same rows, one toggle away.
 */
export function WorkCanvasBoard({ rows, onOpenWork }: Props) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const [hoveredId, setHoveredId] = useState<string | null>(null);
  const layout = useMemo(() => layoutWorkGraph(rows), [rows]);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const context = canvas.getContext("2d");
    if (!context) return;
    const scale = window.devicePixelRatio || 1;
    canvas.width = Math.max(1, Math.round(layout.width * scale));
    canvas.height = Math.max(1, Math.round(layout.height * scale));
    context.setTransform(scale, 0, 0, scale, 0, 0);
    context.clearRect(0, 0, layout.width, layout.height);

    context.strokeStyle = EDGE_STROKE;
    context.lineWidth = 1;
    for (const edge of layout.edges) {
      context.beginPath();
      context.moveTo(edge.path.from.x, edge.path.from.y);
      context.bezierCurveTo(
        edge.path.c1.x,
        edge.path.c1.y,
        edge.path.c2.x,
        edge.path.c2.y,
        edge.path.to.x,
        edge.path.to.y,
      );
      context.stroke();
    }

    for (const node of layout.nodes) {
      const hovered = node.id === hoveredId;
      context.fillStyle = NODE_FILL;
      roundedRect(context, node, 8);
      context.fill();
      context.strokeStyle = hovered ? TITLE_FILL : NODE_STROKE;
      context.stroke();

      const anchor = graphTitleAnchor(node);
      context.fillStyle = TITLE_FILL;
      context.font = TITLE_FONT;
      context.textBaseline = "alphabetic";
      const title =
        node.title.length > TITLE_MAX_CHARS
          ? `${node.title.slice(0, TITLE_MAX_CHARS - 1)}…`
          : node.title;
      context.fillText(title, anchor.x, anchor.y);

      context.fillStyle = STATUS_FILL;
      context.font = STATUS_FONT;
      context.fillText(
        node.statusLabel,
        anchor.x,
        node.y + node.height - 9,
      );
    }
  }, [layout, hoveredId]);

  const toCanvasPoint = useCallback(
    (event: { clientX: number; clientY: number }) => {
      const canvas = canvasRef.current;
      if (!canvas) return null;
      const bounds = canvas.getBoundingClientRect();
      return {
        x: event.clientX - bounds.left,
        y: event.clientY - bounds.top,
      };
    },
    [],
  );

  const onPointerMove = useCallback(
    (event: React.PointerEvent<HTMLCanvasElement>) => {
      const point = toCanvasPoint(event);
      const hit = point ? hitTest(layout.nodes, point) : null;
      setHoveredId(hit?.id ?? null);
    },
    [layout.nodes, toCanvasPoint],
  );

  const onPointerLeave = useCallback(() => setHoveredId(null), []);

  const onClick = useCallback(
    (event: React.MouseEvent<HTMLCanvasElement>) => {
      const point = toCanvasPoint(event);
      const hit = point ? hitTest(layout.nodes, point) : null;
      if (hit) onOpenWork(hit.id);
    },
    [layout.nodes, onOpenWork, toCanvasPoint],
  );

  if (layout.nodes.length === 0) return null;

  return (
    <div className="h-full w-full overflow-auto p-3">
      <canvas
        ref={canvasRef}
        role="img"
        aria-label={`Work graph: ${layout.nodes.length} items across ${Math.max(
          ...layout.nodes.map((node) => node.depth),
        ) + 1} levels. Switch to the board view for keyboard access.`}
        style={{
          width: layout.width,
          height: layout.height,
          cursor: hoveredId ? "pointer" : "default",
          display: "block",
        }}
        onPointerMove={onPointerMove}
        onPointerLeave={onPointerLeave}
        onClick={onClick}
      />
    </div>
  );
}
