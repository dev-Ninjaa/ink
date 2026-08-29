import { WorkspaceInfo } from "../models/WorkspaceInfo";
import { DependencyGraph } from "../models/DependencyGraph";
import { escapeHtml, getNonce } from "../utils/webview";
import { baseStyles } from "./styles";

export function renderDependencyGraphPage(workspace: WorkspaceInfo, graph: DependencyGraph): string {
  const nonce = getNonce();
  const payload = JSON.stringify({ nodes: graph.nodes, edges: graph.edges }).replace(/</g, "\\u003c");
  const emptyState = graph.nodes.length === 0
    ? `<div class="graph-empty">No files were detected in this workspace.</div>`
    : "";
  const canvas = graph.nodes.length === 0 ? "" : `<svg id="canvas" xmlns="http://www.w3.org/2000/svg"></svg>`;

  return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'nonce-${nonce}'; script-src 'nonce-${nonce}';">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Ink Dependency Graph — ${escapeHtml(workspace.name)}</title>
  <style nonce="${nonce}">${baseStyles}</style>
  <style nonce="${nonce}">
    html, body {
      height: 100%;
      overflow: hidden;
      padding: 0;
    }
    .graph-shell {
      display: flex;
      flex-direction: column;
      height: 100vh;
      gap: 0;
    }
    .graph-toolbar {
      display: flex;
      justify-content: space-between;
      align-items: center;
      padding: 12px 20px;
      border-bottom: 1px solid var(--vscode-panel-border);
    }
    .legend {
      display: flex;
      gap: 16px;
      align-items: center;
      font-size: 12px;
      color: var(--vscode-descriptionForeground);
    }
    .dot {
      display: inline-block;
      width: 10px;
      height: 10px;
      border-radius: 50%;
      margin-right: 6px;
      vertical-align: middle;
    }
    .dot-entry { background: var(--vscode-charts-yellow); }
    .dot-file { background: var(--vscode-charts-blue); }
    #canvas {
      flex: 1;
      width: 100%;
      min-height: 0;
      cursor: grab;
    }
    #canvas.panning { cursor: grabbing; }
    .edge {
      stroke: var(--vscode-charts-line);
      stroke-opacity: 0.35;
      transition: stroke-opacity 0.15s ease, stroke 0.15s ease;
    }
    .edge.hot {
      stroke: var(--vscode-focusBorder);
      stroke-opacity: 1;
      stroke-width: 2;
    }
    .node-label {
      font-size: 10px;
      fill: var(--vscode-descriptionForeground);
      pointer-events: none;
      user-select: none;
    }
    .dim { opacity: 0.1; }
    .graph-empty {
      border: 1px solid var(--vscode-panel-border);
      border-radius: 6px;
      margin: 20px;
      padding: 18px;
      color: var(--vscode-descriptionForeground);
    }
  </style>
</head>
<body>
  <div class="graph-shell">
    <div class="graph-toolbar">
      <h1>Dependency Graph
        <span class="muted" style="font-size:13px;font-weight:400;margin-left:10px;">${graph.nodes.length} nodes · ${graph.edges.length} edges · ${escapeHtml(workspace.name)}</span>
      </h1>
      <div class="legend">
        <span><span class="dot dot-entry"></span>entry point</span>
        <span><span class="dot dot-file"></span>file / module</span>
        <span class="muted">drag nodes · scroll to zoom · drag background to pan</span>
      </div>
    </div>
    ${canvas}
    ${emptyState}
  </div>
  <script nonce="${nonce}">
    const DATA = ${payload};
    (function () {
      if (!DATA.nodes.length) { return; }
      const svg = document.getElementById("canvas");
      const NS = "http://www.w3.org/2000/svg";

      const view = { x: 0, y: 0, w: Math.max(svg.clientWidth || 900, 300), h: Math.max(svg.clientHeight || 600, 200) };
      const applyView = () => svg.setAttribute("viewBox", view.x + " " + view.y + " " + view.w + " " + view.h);
      applyView();

      const count = DATA.nodes.length;
      const spread = Math.max(view.w, view.h) * 0.35;
      const nodes = DATA.nodes.map((n, i) => {
        const angle = (i / count) * Math.PI * 2;
        return {
          id: n.id,
          label: n.label,
          entry: n.kind === "service",
          x: view.x + view.w / 2 + Math.cos(angle) * spread + (Math.random() - 0.5) * 40,
          y: view.y + view.h / 2 + Math.sin(angle) * spread + (Math.random() - 0.5) * 40,
          vx: 0,
          vy: 0,
          degree: 0,
          el: null,
          labelEl: null
        };
      });
      const byId = new Map(nodes.map((n) => [n.id, n]));
      const edges = [];
      for (const e of DATA.edges) {
        const a = byId.get(e.from);
        const b = byId.get(e.to);
        if (a && b && a !== b) {
          edges.push({ a, b, el: null });
          a.degree += 1;
          b.degree += 1;
        }
      }

      const edgeLayer = document.createElementNS(NS, "g");
      const nodeLayer = document.createElementNS(NS, "g");
      const labelLayer = document.createElementNS(NS, "g");
      svg.append(edgeLayer, nodeLayer, labelLayer);

      const radiusOf = (n) => (n.entry ? 9 : 5.5) + Math.min(n.degree * 0.8, 6);
      for (const edge of edges) {
        const line = document.createElementNS(NS, "line");
        line.setAttribute("class", "edge");
        const tip = document.createElementNS(NS, "title");
        tip.textContent = edge.a.label + " -> " + edge.b.label;
        line.append(tip);
        edge.el = line;
        edgeLayer.append(line);
      }

      const shortLabel = (p) => {
        const parts = p.split("/");
        return parts.length > 1 ? parts.slice(-2).join("/") : p;
      };
      for (const node of nodes) {
        const circle = document.createElementNS(NS, "circle");
        circle.setAttribute("fill", node.entry ? "var(--vscode-charts-yellow)" : "var(--vscode-charts-blue)");
        circle.setAttribute("fill-opacity", node.entry ? "1" : "0.85");
        circle.style.cursor = "pointer";
        const tip = document.createElementNS(NS, "title");
        tip.textContent = node.label;
        circle.append(tip);
        node.el = circle;
        nodeLayer.append(circle);

        const text = document.createElementNS(NS, "text");
        text.setAttribute("class", "node-label");
        text.setAttribute("text-anchor", "middle");
        text.textContent = shortLabel(node.label);
        node.labelEl = text;
        labelLayer.append(text);
      }

      let alpha = 1;
      function tick() {
        for (let i = 0; i < nodes.length; i++) {
          for (let j = i + 1; j < nodes.length; j++) {
            const a = nodes[i];
            const b = nodes[j];
            let dx = b.x - a.x;
            let dy = b.y - a.y;
            let d2 = dx * dx + dy * dy;
            if (d2 < 1) { d2 = 1; dx = Math.random(); dy = Math.random(); }
            const minD = radiusOf(a) + radiusOf(b) + 24;
            if (d2 > minD * minD * 36) { continue; }
            const f = 2400 / d2;
            const fx = dx * f;
            const fy = dy * f;
            a.vx -= fx;
            a.vy -= fy;
            b.vx += fx;
            b.vy += fy;
          }
        }
        for (const edge of edges) {
          const dx = edge.b.x - edge.a.x;
          const dy = edge.b.y - edge.a.y;
          const d = Math.sqrt(dx * dx + dy * dy) || 1;
          const f = ((d - 110) / d) * 0.04 * (0.4 + alpha);
          edge.a.vx += dx * f;
          edge.a.vy += dy * f;
          edge.b.vx -= dx * f;
          edge.b.vy -= dy * f;
        }
        const cx = view.x + view.w / 2;
        const cy = view.y + view.h / 2;
        for (const node of nodes) {
          node.vx += (cx - node.x) * 0.0025;
          node.vy += (cy - node.y) * 0.0025;
          node.vx *= 0.82;
          node.vy *= 0.82;
          node.x += node.vx;
          node.y += node.vy;
        }
      }

      function position() {
        for (const node of nodes) {
          const r = radiusOf(node);
          node.el.setAttribute("cx", node.x);
          node.el.setAttribute("cy", node.y);
          node.el.setAttribute("r", r);
          node.labelEl.setAttribute("x", node.x);
          node.labelEl.setAttribute("y", node.y - r - 5);
        }
        for (const edge of edges) {
          edge.el.setAttribute("x1", edge.a.x);
          edge.el.setAttribute("y1", edge.a.y);
          edge.el.setAttribute("x2", edge.b.x);
          edge.el.setAttribute("y2", edge.b.y);
        }
      }

      function frame() {
        if (alpha > 0.02) {
          tick();
          alpha *= 0.99;
        }
        position();
        requestAnimationFrame(frame);
      }

      const neighbors = (node) => {
        const set = new Set([node]);
        for (const e of edges) {
          if (e.a === node) { set.add(e.b); }
          if (e.b === node) { set.add(e.a); }
        }
        return set;
      };

      function focus(node) {
        const keep = neighbors(node);
        for (const n of nodes) {
          n.el.classList.toggle("dim", !keep.has(n));
          n.labelEl.classList.toggle("dim", !keep.has(n));
        }
        for (const e of edges) {
          e.el.classList.toggle("hot", e.a === node || e.b === node);
        }
      }

      function unfocus() {
        for (const n of nodes) {
          n.el.classList.remove("dim");
          n.labelEl.classList.remove("dim");
        }
        for (const e of edges) {
          e.el.classList.remove("hot");
        }
      }

      let dragNode = null;
      let panning = null;

      svg.addEventListener("pointerdown", (event) => {
        const target = event.target;
        const node = nodes.find((n) => n.el === target);
        if (node) {
          dragNode = node;
          alpha = Math.max(alpha, 0.35);
          focus(node);
          svg.setPointerCapture(event.pointerId);
        } else {
          panning = { x: event.clientX, y: event.clientY, vx: view.x, vy: view.y };
          svg.classList.add("panning");
          svg.setPointerCapture(event.pointerId);
        }
      });

      svg.addEventListener("pointermove", (event) => {
        if (dragNode) {
          const rect = svg.getBoundingClientRect();
          dragNode.x = view.x + ((event.clientX - rect.left) / rect.width) * view.w;
          dragNode.y = view.y + ((event.clientY - rect.top) / rect.height) * view.h;
          dragNode.vx = 0;
          dragNode.vy = 0;
          return;
        }
        if (panning) {
          const rect = svg.getBoundingClientRect();
          view.x = panning.vx - ((event.clientX - panning.x) / rect.width) * view.w;
          view.y = panning.vy - ((event.clientY - panning.y) / rect.height) * view.h;
          applyView();
          return;
        }
        const node = nodes.find((n) => n.el === event.target);
        if (node) { focus(node); } else { unfocus(); }
      });

      svg.addEventListener("pointerup", (event) => {
        dragNode = null;
        panning = null;
        svg.classList.remove("panning");
        try { svg.releasePointerCapture(event.pointerId); } catch (_) {}
      });

      svg.addEventListener("pointerleave", () => {
        if (!dragNode) { unfocus(); }
      });

      svg.addEventListener("wheel", (event) => {
        event.preventDefault();
        const factor = event.deltaY > 0 ? 1.12 : 0.89;
        const rect = svg.getBoundingClientRect();
        const px = view.x + ((event.clientX - rect.left) / rect.width) * view.w;
        const py = view.y + ((event.clientY - rect.top) / rect.height) * view.h;
        view.w = Math.min(Math.max(view.w * factor, 200), 8000);
        view.h = Math.min(Math.max(view.h * factor, 140), 5600);
        view.x = px - ((px - view.x) / factor);
        view.y = py - ((py - view.y) / factor);
        applyView();
      }, { passive: false });

      requestAnimationFrame(frame);
    })();
  </script>
</body>
</html>`;
}
