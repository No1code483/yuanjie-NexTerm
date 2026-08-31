import { t } from "i18next";
import { useState, useEffect, useCallback, useRef } from 'react';
import styles from '../Knowledge.module.css';
import type { KbEntry, KbCategory, KbTag } from './types';
function getTypeLabel(type?: string) {
  switch (type) {
    case 'link':
      return t("components.TextEditor.k9");
    case 'file':
      return t("knowledge.GraphView.k1");
    case 'text':
      return t("knowledge.GraphView.k2");
    case 'video':
      return t("knowledge.GraphView.k3");
    case 'image':
      return t("knowledge.GraphView.k4");
    case 'audio':
      return t("knowledge.GraphView.k5");
    case 'document':
      return t("knowledge.GraphView.k6");
    default:
      return t("components.GroupChatOrchestrationPanel.k35");
  }
}
interface GraphViewProps {
  allEntries: KbEntry[];
  categories: KbCategory[];
  entryTags: Map<number, KbTag[]>;
  selectedId: {
    type: 'category' | 'entry';
    id: number;
  } | null;
  onClose: () => void;
  onOpenFileViewer: (entry: KbEntry) => void;
  onSelectedIdChange: (id: {
    type: 'category' | 'entry';
    id: number;
  }) => void;
  onShowStatus: (type: 'success' | 'error', text: string) => void;
}
export default function GraphView({
  allEntries,
  categories,
  entryTags,
  selectedId,
  onClose,
  onOpenFileViewer,
  onSelectedIdChange,
  onShowStatus
}: GraphViewProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const graphContainerRef = useRef<HTMLDivElement>(null);
  const [graphPan, setGraphPan] = useState({
    x: 0,
    y: 0
  });
  const [graphZoom, setGraphZoom] = useState(1);
  const [graphSearchQuery, setGraphSearchQuery] = useState('');
  const [graphFilteredNodeIds, setGraphFilteredNodeIds] = useState<Set<number> | null>(null);
  const [graphDragging, setGraphDragging] = useState(false);
  const [graphDragStart, setGraphDragStart] = useState({
    x: 0,
    y: 0
  });
  const [graphHoveredNode, setGraphHoveredNode] = useState<KbEntry | null>(null);
  const [graphExpandedCatIds, setGraphExpandedCatIds] = useState<Set<number>>(new Set());
  const [graphHoveredCatId, setGraphHoveredCatId] = useState<number | null>(null);
  const [graphHighlightType, setGraphHighlightType] = useState<string | null>(null);
  const simRef = useRef<{
    positions: {
      id: number;
      x: number;
      y: number;
      anchorX: number;
      anchorY: number;
      isCategory: boolean;
    }[];
    velocities: {
      vx: number;
      vy: number;
    }[];
    alpha: number;
    draggingId: number | null;
  } | null>(null);
  const animFrameRef = useRef<number>(0);
  const renderGraphRef = useRef<() => void>(() => {});
  const prevDragPos = useRef({
    x: 0,
    y: 0
  });
  const dragSnapshotRef = useRef<Array<{
    id: number;
    x: number;
    y: number;
  }>>([]);
  const getTagConnections = useCallback((): Map<number, number[]> => {
    const tagToEntries = new Map<number, number[]>();
    for (const [entryId, tagList] of entryTags) {
      for (const tag of tagList) {
        const list = tagToEntries.get(tag.id) || [];
        list.push(entryId);
        tagToEntries.set(tag.id, list);
      }
    }
    const connections = new Map<number, number[]>();
    for (const [, entries] of tagToEntries) {
      for (let i = 0; i < entries.length; i++) {
        for (let j = i + 1; j < entries.length; j++) {
          const a = entries[i],
            b = entries[j];
          if (!connections.has(a)) connections.set(a, []);
          if (!connections.has(b)) connections.set(b, []);
          if (!connections.get(a)!.includes(b)) connections.get(a)!.push(b);
          if (!connections.get(b)!.includes(a)) connections.get(b)!.push(a);
        }
      }
    }
    return connections;
  }, [entryTags]);
  const getGraphVisibleNodes = useCallback((): {
    visibleEntries: KbEntry[];
    visibleCats: KbCategory[];
  } => {
    if (selectedId) {
      const visibleCatIds = new Set<number>();
      const visibleEntryIds = new Set<number>();
      if (selectedId.type === 'entry') {
        const entry = allEntries.find(e => e.id === selectedId.id);
        if (entry) {
          visibleEntryIds.add(entry.id);
          if (entry.category_id) {
            const parentCat = categories.find(c => c.id === entry.category_id);
            if (parentCat) {
              visibleCatIds.add(parentCat.id);
              let ancestorId = parentCat.parent_id;
              while (ancestorId) {
                visibleCatIds.add(ancestorId);
                ancestorId = categories.find(c => c.id === ancestorId)?.parent_id ?? null;
              }
              for (const e of allEntries) {
                if (e.category_id === entry.category_id) visibleEntryIds.add(e.id);
              }
            }
          }
        }
      } else if (selectedId.type === 'category') {
        const cat = categories.find(c => c.id === selectedId.id);
        if (cat) {
          visibleCatIds.add(cat.id);
          let ancestorId = cat.parent_id;
          while (ancestorId) {
            visibleCatIds.add(ancestorId);
            ancestorId = categories.find(c => c.id === ancestorId)?.parent_id ?? null;
          }
          const collectDescendants = (parentId: number) => {
            for (const c of categories) {
              if (c.parent_id === parentId) {
                visibleCatIds.add(c.id);
                collectDescendants(c.id);
              }
            }
          };
          collectDescendants(cat.id);
          for (const e of allEntries) {
            if (e.category_id && visibleCatIds.has(e.category_id)) visibleEntryIds.add(e.id);
          }
        }
      }
      return {
        visibleEntries: allEntries.filter(e => visibleEntryIds.has(e.id)),
        visibleCats: categories.filter(c => visibleCatIds.has(c.id))
      };
    }
    const expandedIds = graphExpandedCatIds;
    const visibleCatIds = new Set<number>();
    const visibleEntryIds = new Set<number>();
    for (const c of categories) {
      if (!c.parent_id) visibleCatIds.add(c.id);
    }
    for (const e of allEntries) {
      if (!e.category_id || !categories.some(c => c.id === e.category_id)) {
        visibleEntryIds.add(e.id);
      }
    }
    for (const catId of expandedIds) {
      visibleCatIds.add(catId);
      let ancestorId = categories.find(c => c.id === catId)?.parent_id;
      while (ancestorId) {
        visibleCatIds.add(ancestorId);
        ancestorId = categories.find(c => c.id === ancestorId)?.parent_id ?? null;
      }
      for (const c of categories) {
        if (c.parent_id === catId) visibleCatIds.add(c.id);
      }
      for (const e of allEntries) {
        if (e.category_id === catId) visibleEntryIds.add(e.id);
      }
    }
    for (const catId of visibleCatIds) {
      for (const e of allEntries) {
        if (e.category_id === catId) visibleEntryIds.add(e.id);
      }
    }
    return {
      visibleEntries: allEntries.filter(e => visibleEntryIds.has(e.id)),
      visibleCats: categories.filter(c => visibleCatIds.has(c.id))
    };
  }, [graphExpandedCatIds, selectedId, allEntries, categories]);
  const getGraphNodePositions = useCallback((w: number, h: number) => {
    const catRingRadius = Math.min(w, h) * 0.14;
    const graphData = getGraphVisibleNodes();
    const activeEntries = graphData?.visibleEntries ?? allEntries;
    const activeCategories = graphData?.visibleCats ?? categories;
    const allCats = activeCategories.filter(c => activeEntries.some(e => e.category_id === c.id));
    const rootCats = allCats.filter(c => !c.parent_id);
    const subCats = allCats.filter(c => c.parent_id);
    const catNodes: {
      id: number;
      name: string;
      x: number;
      y: number;
      anchorX: number;
      anchorY: number;
    }[] = [];
    rootCats.forEach((cat, i) => {
      const angle = i / Math.max(rootCats.length, 1) * Math.PI * 2 - Math.PI / 2;
      const cx = Math.cos(angle) * catRingRadius;
      const cy = Math.sin(angle) * catRingRadius;
      catNodes.push({
        id: -(cat.id + 1),
        name: cat.name,
        x: cx,
        y: cy,
        anchorX: cx,
        anchorY: cy
      });
    });
    const placedCatIds = new Set(rootCats.map(c => c.id));
    let remaining = [...subCats];
    while (remaining.length > 0) {
      const nextRound: typeof subCats = [];
      for (const subCat of remaining) {
        if (subCat.parent_id && placedCatIds.has(subCat.parent_id)) {
          const parentNode = catNodes.find(n => -(subCat.parent_id! + 1) === n.id);
          if (parentNode) {
            const siblingCount = catNodes.filter(n => {
              const realId = -(n.id + 1);
              const cat = allCats.find(c => c.id === realId);
              return cat?.parent_id === subCat.parent_id;
            }).length;
            const subRingRadius = 130 + Math.floor(siblingCount / 4) * 45;
            const subAngle = siblingCount % 4 * (Math.PI / 2) + Math.PI / 4;
            const sx = parentNode.x + Math.cos(subAngle) * subRingRadius;
            const sy = parentNode.y + Math.sin(subAngle) * subRingRadius;
            catNodes.push({
              id: -(subCat.id + 1),
              name: subCat.name,
              x: sx,
              y: sy,
              anchorX: sx,
              anchorY: sy
            });
            placedCatIds.add(subCat.id);
          } else {
            nextRound.push(subCat);
          }
        } else {
          nextRound.push(subCat);
        }
      }
      if (nextRound.length === remaining.length) break;
      remaining = nextRound;
    }
    remaining.forEach((cat, i) => {
      const angle = i / Math.max(remaining.length, 1) * Math.PI * 2 + Math.PI;
      const cx = Math.cos(angle) * (catRingRadius + 200);
      const cy = Math.sin(angle) * (catRingRadius + 200);
      catNodes.push({
        id: -(cat.id + 1),
        name: cat.name,
        x: cx,
        y: cy,
        anchorX: cx,
        anchorY: cy
      });
    });
    const uncategorized = activeEntries.filter(e => !e.category_id || !activeCategories.some(c => c.id === e.category_id));
    if (uncategorized.length > 0) {
      catNodes.push({
        id: -1,
        name: t("knowledge.GraphView.k7"),
        x: 0,
        y: catRingRadius + 220,
        anchorX: 0,
        anchorY: catRingRadius + 220
      });
    }
    const catById = new Map(catNodes.map(c => [c.id, c]));
    const fileNodes: {
      id: number;
      x: number;
      y: number;
      parentCatId: number;
    }[] = [];
    for (const entry of activeEntries) {
      let catId: number | null = null;
      if (entry.category_id && allCats.some(c => c.id === entry.category_id)) {
        catId = -(entry.category_id + 1);
      } else if (uncategorized.length > 0) {
        catId = -1;
      }
      if (catId === null) continue;
      const catPos = catById.get(catId);
      if (!catPos) continue;
      const siblings = fileNodes.filter(f => f.parentCatId === catId);
      const perRing = 14;
      const ringIdx = Math.floor(siblings.length / perRing);
      const posInRing = siblings.length % perRing;
      const ringRadius = 70 + ringIdx * 55;
      const ringTotal = Math.min(perRing, siblings.length + 1);
      const angle = ringTotal <= 1 ? 0 : posInRing / ringTotal * Math.PI * 2 - Math.PI / 2;
      fileNodes.push({
        id: entry.id,
        x: catPos.x + Math.cos(angle) * ringRadius + (Math.random() - 0.5) * 30,
        y: catPos.y + Math.sin(angle) * ringRadius + (Math.random() - 0.5) * 30,
        parentCatId: catId
      });
    }
    return {
      catNodes,
      fileNodes
    };
  }, [allEntries, categories, getGraphVisibleNodes]);
  const initForceSim = useCallback(() => {
    const {
      catNodes,
      fileNodes
    } = getGraphNodePositions(800, 600);
    const positions: {
      id: number;
      x: number;
      y: number;
      anchorX: number;
      anchorY: number;
      isCategory: boolean;
    }[] = [];
    for (const c of catNodes) {
      positions.push({
        id: c.id,
        x: c.x,
        y: c.y,
        anchorX: c.anchorX,
        anchorY: c.anchorY,
        isCategory: true
      });
    }
    for (const f of fileNodes) {
      const cat = catNodes.find(c => c.id === f.parentCatId);
      positions.push({
        id: f.id,
        x: f.x,
        y: f.y,
        anchorX: cat?.x ?? f.x,
        anchorY: cat?.y ?? f.y,
        isCategory: false
      });
    }
    simRef.current = {
      positions,
      velocities: positions.map(() => ({
        vx: (Math.random() - 0.5) * 0.4,
        vy: (Math.random() - 0.5) * 0.4
      })),
      alpha: 0.25,
      draggingId: null
    };
  }, [getGraphNodePositions]);
  const applyForceStep = useCallback(() => {
    const sim = simRef.current;
    if (!sim || sim.alpha < 0.005) return;
    const positions = sim.positions;
    const n = positions.length;
    const kRepulse = 1200;
    const kAnchor = 0.012;
    const kCatAnchor = 0.08;
    const damping = 0.88;
    const maxSpeed = 3.5;
    const centerGravity = 0.0008;
    for (let i = 0; i < n; i++) {
      for (let j = i + 1; j < n; j++) {
        const dx = positions[j].x - positions[i].x;
        const dy = positions[j].y - positions[i].y;
        const distSq = dx * dx + dy * dy;
        if (distSq > 50000) continue;
        const dist = Math.sqrt(distSq) || 1;
        const force = kRepulse / (dist * dist + 15);
        const fx = dx / dist * force;
        const fy = dy / dist * force;
        sim.velocities[i].vx -= fx;
        sim.velocities[i].vy -= fy;
        sim.velocities[j].vx += fx;
        sim.velocities[j].vy += fy;
      }
    }
    for (let i = 0; i < n; i++) {
      if (sim.draggingId === positions[i].id) {
        sim.velocities[i].vx = 0;
        sim.velocities[i].vy = 0;
        continue;
      }
      const k = positions[i].isCategory ? kCatAnchor : kAnchor;
      const dx = positions[i].anchorX - positions[i].x;
      const dy = positions[i].anchorY - positions[i].y;
      sim.velocities[i].vx += dx * k;
      sim.velocities[i].vy += dy * k;
      sim.velocities[i].vx -= positions[i].x * centerGravity;
      sim.velocities[i].vy -= positions[i].y * centerGravity;
      sim.velocities[i].vx *= damping;
      sim.velocities[i].vy *= damping;
      const spd = Math.sqrt(sim.velocities[i].vx ** 2 + sim.velocities[i].vy ** 2);
      if (spd > maxSpeed) {
        sim.velocities[i].vx = sim.velocities[i].vx / spd * maxSpeed;
        sim.velocities[i].vy = sim.velocities[i].vy / spd * maxSpeed;
      }
      positions[i].x += sim.velocities[i].vx * sim.alpha;
      positions[i].y += sim.velocities[i].vy * sim.alpha;
    }
    sim.alpha *= 0.9985;
    if (sim.alpha < 0.04) sim.alpha = 0.04;
  }, []);
  const renderGraph = useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    const container = graphContainerRef.current;
    if (!container) return;
    const rect = container.getBoundingClientRect();
    const dpr = window.devicePixelRatio;
    canvas.width = rect.width * dpr;
    canvas.height = rect.height * dpr;
    canvas.style.width = `${rect.width}px`;
    canvas.style.height = `${rect.height}px`;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    const w = rect.width;
    const h = rect.height;
    ctx.fillStyle = 'rgba(5, 5, 10, 0.95)';
    ctx.fillRect(0, 0, w, h);
    const worldCx = w / 2 + graphPan.x;
    const worldCy = h / 2 + graphPan.y;
    ctx.save();
    ctx.translate(worldCx, worldCy);
    ctx.scale(graphZoom, graphZoom);
    const SIZE_CAP_ZOOM = 1.8;
    const sizeScale = graphZoom > SIZE_CAP_ZOOM ? SIZE_CAP_ZOOM / graphZoom : 1;
    const graphData = getGraphVisibleNodes();
    const graphEntries = graphData?.visibleEntries ?? allEntries;
    const graphCats = graphData?.visibleCats ?? categories;
    applyForceStep();
    const sim = simRef.current;
    const simPositions = sim?.positions || [];
    const catPosMap = new Map<number, {
      x: number;
      y: number;
    }>();
    const filePosMap = new Map<number, {
      x: number;
      y: number;
    }>();
    for (const sp of simPositions) {
      if (sp.id < 0) {
        catPosMap.set(sp.id, {
          x: sp.x,
          y: sp.y
        });
      } else {
        filePosMap.set(sp.id, {
          x: sp.x,
          y: sp.y
        });
      }
    }
    const entryById = new Map(graphEntries.map(e => [e.id, e]));
    const tagConnections = getTagConnections();
    const typeColor: Record<string, string> = {
      text: '#00F0FF',
      link: '#FFA500',
      file: '#00FF41',
      video: '#B026FF',
      image: '#FF6B9D',
      audio: '#FFD700'
    };
    const hlType = graphHighlightType;
    const isCatDimmed = hlType !== null && hlType !== 'category';
    const DIM_NODE_FILL = '#151515';
    const DIM_NODE_STROKE = '#2a2a2a';
    const DIM_TEXT_COLOR = '#333';
    const DIM_LINE_COLOR = 'rgba(60,60,60,0.12)';
    const hoverNeighbors = new Set<number>();
    const filteredIds = graphFilteredNodeIds;
    if (graphHoveredNode) {
      hoverNeighbors.add(graphHoveredNode.id);
      const neighbors = tagConnections.get(graphHoveredNode.id) || [];
      for (const nid of neighbors) hoverNeighbors.add(nid);
    }
    const drawnTagEdges = new Set<string>();
    for (const [idA, neighbors] of tagConnections) {
      const posA = filePosMap.get(idA);
      if (!posA) continue;
      for (const idB of neighbors) {
        const edgeKey = idA < idB ? `${idA}-${idB}` : `${idB}-${idA}`;
        if (drawnTagEdges.has(edgeKey)) continue;
        drawnTagEdges.add(edgeKey);
        const posB = filePosMap.get(idB);
        if (!posB) continue;
        const isHL = hoverNeighbors.has(idA) && hoverNeighbors.has(idB);
        const entryA = entryById.get(idA);
        const entryB = entryById.get(idB);
        const aDimmed = hlType !== null && hlType !== 'category' && entryA && entryA.entry_type !== hlType;
        const bDimmed = hlType !== null && hlType !== 'category' && entryB && entryB.entry_type !== hlType;
        const lineDimmed = aDimmed || bDimmed;
        ctx.beginPath();
        ctx.moveTo(posA.x, posA.y);
        ctx.lineTo(posB.x, posB.y);
        if (lineDimmed) {
          ctx.strokeStyle = DIM_LINE_COLOR;
          ctx.lineWidth = 0.4;
        } else {
          ctx.strokeStyle = isHL ? 'rgba(0,240,255,0.25)' : 'rgba(255,255,255,0.04)';
          ctx.lineWidth = isHL ? 1.2 : 0.4;
        }
        ctx.stroke();
      }
    }
    const drawnTreeEdges = new Set<string>();
    for (const [fileId, filePos] of filePosMap) {
      const entry = entryById.get(fileId);
      if (!entry) continue;
      let catNodeId: number | null = null;
      if (entry.category_id && graphCats.some(c => c.id === entry.category_id)) {
        catNodeId = -(entry.category_id + 1);
      } else {
        catNodeId = -1;
      }
      const catPos = catPosMap.get(catNodeId);
      if (!catPos) continue;
      const edgeKey = `${catNodeId}->${fileId}`;
      if (drawnTreeEdges.has(edgeKey)) continue;
      drawnTreeEdges.add(edgeKey);
      const entryType = entry.entry_type;
      const lineColor = typeColor[entryType] || '#888';
      const isCatHL = graphHoveredNode && entry.id === graphHoveredNode.id;
      const fileAlpha = isCatHL ? 0.7 : 0.22;
      const edgeDimmedByHl = hlType !== null && (hlType === 'category' || entryType !== hlType);
      ctx.beginPath();
      ctx.moveTo(catPos.x, catPos.y);
      ctx.lineTo(filePos.x, filePos.y);
      if (edgeDimmedByHl) {
        ctx.strokeStyle = DIM_LINE_COLOR;
        ctx.lineWidth = 0.4;
      } else {
        ctx.strokeStyle = lineColor.replace(')', `, ${fileAlpha})`).replace('rgb', 'rgba');
        if (!lineColor.startsWith('#')) {
          ctx.strokeStyle = `rgba(136,136,136,${fileAlpha})`;
        }
        ctx.lineWidth = isCatHL ? 1.8 : 0.6;
      }
      ctx.stroke();
    }
    const catRadius = 26 * sizeScale;
    const fileBaseRadius = 12 * sizeScale;
    const drawnCatEdges = new Set<string>();
    for (const [, childCatPos] of catPosMap) {
      const childCatNodeId = simPositions.find(sp => sp.id < 0 && sp.x === childCatPos.x && sp.y === childCatPos.y)?.id;
      if (childCatNodeId == null) continue;
      const childRealId = -(childCatNodeId + 1);
      const childCat = graphCats.find(c => c.id === childRealId);
      if (!childCat?.parent_id) continue;
      const parentCatNodeId = -(childCat.parent_id + 1);
      const parentCatPos = catPosMap.get(parentCatNodeId);
      if (!parentCatPos) continue;
      const edgeKey = `${parentCatNodeId}->${childCatNodeId}`;
      if (drawnCatEdges.has(edgeKey)) continue;
      drawnCatEdges.add(edgeKey);
      const dx = childCatPos.x - parentCatPos.x;
      const dy = childCatPos.y - parentCatPos.y;
      const dist = Math.sqrt(dx * dx + dy * dy);
      const startR = catRadius;
      const endR = catRadius + 2;
      const ax = parentCatPos.x + dx / dist * startR;
      const ay = parentCatPos.y + dy / dist * startR;
      const bx = childCatPos.x - dx / dist * endR;
      const by = childCatPos.y - dy / dist * endR;
      ctx.beginPath();
      ctx.moveTo(ax, ay);
      ctx.lineTo(bx, by);
      if (isCatDimmed) {
        ctx.strokeStyle = DIM_LINE_COLOR;
        ctx.lineWidth = 0.4;
      } else if (hlType === 'category') {
        ctx.strokeStyle = 'rgba(255,180,0,0.85)';
        ctx.lineWidth = 2.2 * sizeScale;
        ctx.shadowColor = 'rgba(255,165,0,0.5)';
        ctx.shadowBlur = 6;
      } else {
        ctx.strokeStyle = 'rgba(255,165,0,0.55)';
        ctx.lineWidth = 1.4;
      }
      ctx.stroke();
      ctx.shadowBlur = 0;
      const arrowSize = isCatDimmed ? 3 : hlType === 'category' ? 10 : 7;
      const angle = Math.atan2(dy, dx);
      ctx.beginPath();
      ctx.moveTo(bx, by);
      ctx.lineTo(bx - arrowSize * Math.cos(angle - Math.PI / 6), by - arrowSize * Math.sin(angle - Math.PI / 6));
      ctx.lineTo(bx - arrowSize * Math.cos(angle + Math.PI / 6), by - arrowSize * Math.sin(angle + Math.PI / 6));
      ctx.closePath();
      ctx.fillStyle = 'rgba(255,165,0,0.65)';
      ctx.fill();
    }
    for (const [, catPos] of catPosMap) {
      const catEntry = simPositions.find(sp => sp.id < 0 && sp.x === catPos.x && sp.y === catPos.y);
      const catId = catEntry?.id;
      const catName = catId != null ? catId === -1 ? t("knowledge.GraphView.k7") : categories.find(c => -(c.id + 1) === catId)?.name || t("CommandManual.k223") : t("CommandManual.k223");
      const realCatId = catId != null ? catId === -1 ? -1 : -(catId + 1) : -1;
      const isCatExpanded = realCatId !== -1 && graphExpandedCatIds.has(realCatId);
      const isCatHovered = realCatId !== -1 && graphHoveredCatId === realCatId;
      const isCatSelected = selectedId?.type === 'category' && catId != null && -(selectedId.id + 1) === catId;
      const catHlDimmed = isCatDimmed;
      const totalChildren = (() => {
        if (realCatId === -1) {
          return allEntries.filter(e => !e.category_id || !categories.some(c => c.id === e.category_id)).length;
        }
        const subCats = categories.filter(c => c.parent_id === realCatId).length;
        const subFiles = allEntries.filter(e => e.category_id === realCatId).length;
        return subCats + subFiles;
      })();
      const baseR = catRadius;
      const r = baseR + Math.min(totalChildren * 0.5, 12);
      if (isCatSelected || isCatExpanded || isCatHovered) {
        const pulseR = r * 2.5 + Math.sin(Date.now() * 0.004) * 6;
        const highlightColor = isCatSelected ? 'rgba(255,50,50,0.35)' : isCatExpanded ? 'rgba(0,240,255,0.25)' : 'rgba(0,240,255,0.15)';
        const selGrad = ctx.createRadialGradient(catPos.x, catPos.y, r * 0.3, catPos.x, catPos.y, pulseR);
        selGrad.addColorStop(0, highlightColor);
        selGrad.addColorStop(isCatSelected ? 0.5 : 1, isCatSelected ? 'rgba(255,50,50,0.1)' : 'rgba(0,0,0,0)');
        selGrad.addColorStop(1, 'rgba(0,0,0,0)');
        ctx.beginPath();
        ctx.arc(catPos.x, catPos.y, pulseR, 0, Math.PI * 2);
        ctx.fillStyle = selGrad;
        ctx.fill();
      }
      if (hlType === 'category' && !catHlDimmed && !isCatSelected && !isCatExpanded && !isCatHovered) {
        const hlGlowR = r + 4 + Math.sin(Date.now() * 0.003) * 2;
        const catHlGrad = ctx.createRadialGradient(catPos.x, catPos.y, r * 0.8, catPos.x, catPos.y, hlGlowR);
        catHlGrad.addColorStop(0, 'rgba(255,180,0,0.2)');
        catHlGrad.addColorStop(1, 'rgba(255,165,0,0)');
        ctx.beginPath();
        ctx.arc(catPos.x, catPos.y, hlGlowR, 0, Math.PI * 2);
        ctx.fillStyle = catHlGrad;
        ctx.fill();
      }
      const glowGrad = ctx.createRadialGradient(catPos.x, catPos.y, r * 0.5, catPos.x, catPos.y, r * 2.2);
      glowGrad.addColorStop(0, isCatSelected ? 'rgba(255,60,60,0.18)' : isCatExpanded ? 'rgba(0,240,255,0.18)' : 'rgba(0,240,255,0.08)');
      glowGrad.addColorStop(1, 'rgba(0,0,0,0)');
      ctx.beginPath();
      ctx.arc(catPos.x, catPos.y, r * 2.2, 0, Math.PI * 2);
      ctx.fillStyle = glowGrad;
      ctx.fill();
      ctx.beginPath();
      ctx.arc(catPos.x, catPos.y, r, 0, Math.PI * 2);
      ctx.fillStyle = catHlDimmed ? DIM_NODE_FILL : 'rgba(0,0,0,0.85)';
      ctx.fill();
      ctx.strokeStyle = catHlDimmed ? DIM_NODE_STROKE : isCatSelected ? 'rgba(255,50,50,0.9)' : isCatExpanded ? 'rgba(0,240,255,0.9)' : isCatHovered ? 'rgba(0,240,255,0.7)' : hlType === 'category' ? 'rgba(255,180,0,0.75)' : 'rgba(0,240,255,0.45)';
      ctx.lineWidth = ((isCatSelected || isCatExpanded) && !catHlDimmed ? 2.5 : (isCatHovered || hlType === 'category') && !catHlDimmed ? 2 : catHlDimmed ? 1 : 1.5) * sizeScale;
      ctx.setLineDash([]);
      ctx.stroke();
      ctx.fillStyle = catHlDimmed ? DIM_TEXT_COLOR : isCatSelected ? '#FF4444' : isCatExpanded ? '#00F0FF' : hlType === 'category' ? '#ffb833' : 'rgba(0,240,255,0.85)';
      ctx.font = `bold ${((isCatSelected || isCatExpanded) && !catHlDimmed ? 11 : 10) * sizeScale}px "Cascadia Code", monospace`;
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      const label = catName.length > 6 ? catName.slice(0, 5) + '…' : catName;
      ctx.fillText(label, catPos.x, catPos.y + 1);
      ctx.fillStyle = catHlDimmed ? DIM_TEXT_COLOR : isCatSelected ? 'rgba(255,68,68,0.55)' : isCatExpanded ? 'rgba(0,240,255,0.6)' : 'rgba(0,240,255,0.35)';
      ctx.font = `${8 * sizeScale}px "Cascadia Code", monospace`;
      ctx.fillText(`${totalChildren}`, catPos.x, catPos.y + r + 11);
      if (totalChildren > 0) {
        ctx.fillStyle = catHlDimmed ? DIM_TEXT_COLOR : isCatSelected ? 'rgba(255,68,68,0.7)' : isCatExpanded ? 'rgba(0,240,255,0.8)' : 'rgba(0,240,255,0.4)';
        ctx.font = `${9 * sizeScale}px "Cascadia Code", monospace`;
        const indicator = isCatExpanded ? '▼' : '▶';
        ctx.fillText(indicator, catPos.x + r + 8, catPos.y);
      }
    }
    const allDegrees = new Map<number, number>();
    for (const [entryId] of filePosMap) {
      allDegrees.set(entryId, tagConnections.get(entryId)?.length || 0);
    }
    const maxDegree = Math.max(1, ...allDegrees.values());
    const sortedFileIds = [...filePosMap.entries()].sort(([, a], [, b]) => {
      const aHL = hoverNeighbors.has(a as any) ? 1 : 0;
      const bHL = hoverNeighbors.has(b as any) ? 1 : 0;
      return bHL - aHL;
    });
    for (const [entryId, pos] of sortedFileIds) {
      const entry = entryById.get(entryId);
      if (!entry) continue;
      const isHovered = graphHoveredNode?.id === entry.id;
      const isSelected = !isHovered && selectedId?.type === 'entry' && entry.id === selectedId.id;
      const isNeighbor = !isHovered && !isSelected && hoverNeighbors.has(entry.id);
      const hasTagConn = (tagConnections.get(entry.id)?.length || 0) > 0;
      const isFiltered = filteredIds && !filteredIds.has(entry.id);
      const entryHlDimmed = hlType !== null && (hlType === 'category' || entry.entry_type !== hlType);
      const color = typeColor[entry.entry_type] || '#888';
      const degree = allDegrees.get(entry.id) || 0;
      const sizeMul = 0.7 + degree / maxDegree * 0.9;
      const r = fileBaseRadius * (isHovered ? 1.6 : isSelected ? 1.5 : isNeighbor ? 1.25 : 1) * sizeMul;
      if (isSelected) {
        const pulseR = r + 10 + Math.sin(Date.now() * 0.005) * 5;
        const selGrad = ctx.createRadialGradient(pos.x, pos.y, r, pos.x, pos.y, pulseR);
        selGrad.addColorStop(0, 'rgba(255,50,50,0.3)');
        selGrad.addColorStop(1, 'rgba(255,50,50,0)');
        ctx.beginPath();
        ctx.arc(pos.x, pos.y, pulseR, 0, Math.PI * 2);
        ctx.fillStyle = selGrad;
        ctx.fill();
      }
      if (!entryHlDimmed && hlType !== null && !isHovered && !isSelected) {
        const hlGlowR = r + 5 + Math.sin(Date.now() * 0.003) * 2;
        const hlGrad = ctx.createRadialGradient(pos.x, pos.y, r * 0.8, pos.x, pos.y, hlGlowR);
        const hlColor = typeColor[entry.entry_type] || '#888';
        hlGrad.addColorStop(0, `${hlColor}30`);
        hlGrad.addColorStop(1, `${hlColor}00`);
        ctx.beginPath();
        ctx.arc(pos.x, pos.y, hlGlowR, 0, Math.PI * 2);
        ctx.fillStyle = hlGrad;
        ctx.fill();
      }
      if (isHovered) {
        const pulseR = r + 8 + Math.sin(Date.now() * 0.005) * 4;
        const pulseGrad = ctx.createRadialGradient(pos.x, pos.y, r, pos.x, pos.y, pulseR);
        pulseGrad.addColorStop(0, `rgba(0,240,255,0.25)`);
        pulseGrad.addColorStop(1, 'rgba(0,0,0,0)');
        ctx.beginPath();
        ctx.arc(pos.x, pos.y, pulseR, 0, Math.PI * 2);
        ctx.fillStyle = pulseGrad;
        ctx.fill();
      }
      if (isNeighbor && hasTagConn) {
        ctx.beginPath();
        ctx.arc(pos.x, pos.y, r + 3, 0, Math.PI * 2);
        ctx.strokeStyle = 'rgba(255,255,255,0.2)';
        ctx.lineWidth = 1;
        ctx.stroke();
      }
      const nodeGrad = ctx.createRadialGradient(pos.x - r * 0.3, pos.y - r * 0.3, r * 0.05, pos.x, pos.y, r);
      if (entryHlDimmed) {
        nodeGrad.addColorStop(0, '#1a1a1a');
        nodeGrad.addColorStop(0.6, DIM_NODE_FILL);
        nodeGrad.addColorStop(1, '#0a0a0a');
      } else {
        nodeGrad.addColorStop(0, isHovered ? '#fff' : isSelected ? '#FFD0D0' : 'rgba(255,255,255,0.6)');
        nodeGrad.addColorStop(0.6, isSelected ? '#FF3333' : color);
        nodeGrad.addColorStop(1, 'rgba(0,0,0,0.6)');
      }
      ctx.beginPath();
      ctx.arc(pos.x, pos.y, r, 0, Math.PI * 2);
      ctx.fillStyle = nodeGrad;
      if (isFiltered) ctx.globalAlpha = 0.12;
      ctx.fill();
      ctx.globalAlpha = 1;
      const borderCol = entryHlDimmed ? DIM_NODE_STROKE : isSelected ? '#FF4444' : isHovered ? '#fff' : 'rgba(255,255,255,0.15)';
      ctx.strokeStyle = borderCol;
      ctx.lineWidth = (entryHlDimmed ? 0.5 : isSelected ? 2 : isHovered ? 1.5 : 0.5) * sizeScale;
      ctx.stroke();
      ctx.fillStyle = entryHlDimmed ? DIM_TEXT_COLOR : isHovered ? '#000' : '#fff';
      ctx.font = `${((isSelected || isHovered) && !entryHlDimmed ? 10 : 8) * sizeScale}px "Cascadia Code", monospace`;
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      const label = entry.name.length > 8 ? entry.name.slice(0, 7) + '…' : entry.name;
      ctx.fillText(label, pos.x, pos.y + 1);
      if (isHovered) {
        const tooltipY = pos.y - r - 14;
        const tooltipText = entry.name.length > 22 ? entry.name.slice(0, 21) + '…' : entry.name;
        const textW = ctx.measureText(tooltipText).width + 16;
        ctx.fillStyle = 'rgba(0,0,0,0.88)';
        ctx.beginPath();
        ctx.roundRect(pos.x - textW / 2, tooltipY - 10, textW, 20, 4);
        ctx.fill();
        ctx.strokeStyle = color;
        ctx.lineWidth = 1;
        ctx.stroke();
        ctx.fillStyle = color;
        ctx.font = `${10 * sizeScale}px "Cascadia Code", monospace`;
        ctx.fillText(tooltipText, pos.x, tooltipY + 1);
      }
    }
    ctx.restore();
  }, [allEntries, categories, getTagConnections, getGraphVisibleNodes, graphPan, graphZoom, graphHoveredNode, graphFilteredNodeIds, applyForceStep, graphExpandedCatIds, graphHoveredCatId, selectedId, graphHighlightType]);
  renderGraphRef.current = renderGraph;
  useEffect(() => {
    initForceSim();
    const loop = () => {
      renderGraphRef.current();
      animFrameRef.current = requestAnimationFrame(loop);
    };
    animFrameRef.current = requestAnimationFrame(loop);
    return () => {
      if (animFrameRef.current) {
        cancelAnimationFrame(animFrameRef.current);
        animFrameRef.current = 0;
      }
    };
  }, [initForceSim, graphExpandedCatIds, selectedId]);
  const handleGraphMouseDown = (e: React.MouseEvent) => {
    const container = graphContainerRef.current;
    if (!container) return;
    const rect = container.getBoundingClientRect();
    const cx = rect.width / 2 + graphPan.x;
    const cy = rect.height / 2 + graphPan.y;
    const mx = (e.clientX - rect.left - cx) / graphZoom;
    const my = (e.clientY - rect.top - cy) / graphZoom;
    prevDragPos.current = {
      x: mx,
      y: my
    };
    const sim = simRef.current;
    if (sim) {
      const catRadius = 26;
      const fileRadius = 12;
      const tagConnections = getTagConnections();
      for (const sp of sim.positions) {
        const isCat = sp.isCategory;
        let baseR: number;
        if (isCat) {
          const realCatId = sp.id === -1 ? -1 : -(sp.id + 1);
          let totalChildren = 0;
          if (realCatId === -1) {
            totalChildren = allEntries.filter(e => !e.category_id || !categories.some(c => c.id === e.category_id)).length;
          } else {
            totalChildren = categories.filter(c => c.parent_id === realCatId).length + allEntries.filter(e => e.category_id === realCatId).length;
          }
          baseR = catRadius + Math.min(totalChildren * 0.5, 12);
        } else {
          const degree = tagConnections.get(sp.id)?.length || 0;
          baseR = fileRadius * (0.7 + degree * 0.05);
        }
        const dist = Math.sqrt((mx - sp.x) ** 2 + (my - sp.y) ** 2);
        if (dist < Math.max(baseR * 1.3, 16)) {
          sim.draggingId = sp.id;
          dragSnapshotRef.current = sim.positions.map(p => ({
            id: p.id,
            x: p.x,
            y: p.y
          }));
          setGraphDragging(true);
          setGraphDragStart({
            x: mx - sp.x,
            y: my - sp.y
          });
          return;
        }
      }
    }
    setGraphDragging(true);
    setGraphDragStart({
      x: e.clientX - graphPan.x,
      y: e.clientY - graphPan.y
    });
  };
  const handleGraphMouseMove = (e: React.MouseEvent) => {
    const container = graphContainerRef.current;
    if (!container) return;
    const rect = container.getBoundingClientRect();
    const cx = rect.width / 2 + graphPan.x;
    const cy = rect.height / 2 + graphPan.y;
    const mx = (e.clientX - rect.left - cx) / graphZoom;
    const my = (e.clientY - rect.top - cy) / graphZoom;
    const sim = simRef.current;
    if (graphDragging) {
      if (sim && sim.draggingId !== null) {
        const pos = sim.positions.find(p => p.id === sim.draggingId);
        if (pos) {
          pos.x = mx - graphDragStart.x;
          pos.y = my - graphDragStart.y;
          const deltaX = mx - prevDragPos.current.x;
          const deltaY = my - prevDragPos.current.y;
          prevDragPos.current = {
            x: mx,
            y: my
          };
          if (Math.abs(deltaX) > 0.01 || Math.abs(deltaY) > 0.01) {
            const tagConns = getTagConnections();
            const draggedId = sim.draggingId;
            const snapshot = dragSnapshotRef.current;
            const snapMap = new Map(snapshot.map(s => [s.id, s]));
            const aSnap = snapMap.get(draggedId);
            if (!aSnap) return;
            const dragDelta = Math.sqrt((pos.x - aSnap.x) ** 2 + (pos.y - aSnap.y) ** 2);
            const buildAdjList = (): Map<number, number[]> => {
              const adj = new Map<number, number[]>();
              for (const sp of sim.positions) {
                adj.set(sp.id, []);
              }
              for (const entry of allEntries) {
                const eid = entry.id;
                const cid = entry.category_id ? -(entry.category_id + 1) : -1;
                if (adj.has(eid) && adj.has(cid)) {
                  adj.get(eid)!.push(cid);
                  adj.get(cid)!.push(eid);
                }
              }
              for (const cat of categories) {
                const cid = -(cat.id + 1);
                const pid = cat.parent_id ? -(cat.parent_id + 1) : -1;
                if (adj.has(cid) && adj.has(pid)) {
                  adj.get(cid)!.push(pid);
                  adj.get(pid)!.push(cid);
                }
              }
              for (const [srcId, targets] of tagConns) {
                for (const tid of targets) {
                  if (adj.has(srcId) && adj.has(tid)) {
                    if (!adj.get(srcId)!.includes(tid)) adj.get(srcId)!.push(tid);
                    if (!adj.get(tid)!.includes(srcId)) adj.get(tid)!.push(srcId);
                  }
                }
              }
              return adj;
            };
            const adjList = buildAdjList();
            const visited = new Set<number>([draggedId]);
            let currentLevel = [draggedId];
            const MAX_DEPTH = 4;
            for (let depth = 1; depth <= MAX_DEPTH; depth++) {
              const nextLevel: number[] = [];
              const levelAttenuation = Math.pow(1 / 3, depth - 1);
              for (const nodeId of currentLevel) {
                const neighbors = adjList.get(nodeId) || [];
                for (const neighborId of neighbors) {
                  if (visited.has(neighborId)) continue;
                  visited.add(neighborId);
                  nextLevel.push(neighborId);
                  const nPos = sim.positions.find(p => p.id === neighborId);
                  const nSnap = snapMap.get(neighborId);
                  if (!nPos || !nSnap) continue;
                  const origDx = aSnap.x - nSnap.x;
                  const origDy = aSnap.y - nSnap.y;
                  const origDist = Math.sqrt(origDx * origDx + origDy * origDy) || 1;
                  const curDx = pos.x - nPos.x;
                  const curDy = pos.y - nPos.y;
                  const curDist = Math.sqrt(curDx * curDx + curDy * curDy) || 1;
                  const allowedStretch = dragDelta / 3 * levelAttenuation;
                  const actualStretch = curDist - origDist;
                  if (actualStretch > allowedStretch && actualStretch > 0.5) {
                    const excess = (actualStretch - allowedStretch) * 0.6;
                    const pullX = curDx / curDist * excess;
                    const pullY = curDy / curDist * excess;
                    const perpX = -curDy / curDist;
                    const perpY = curDx / curDist;
                    const freq = 8 / Math.pow(1.8, depth);
                    const swayAmp = excess * 0.5 * levelAttenuation;
                    const phase = nPos.id * 13.7 + Date.now() * 0.003;
                    const swayOffset = Math.sin(phase * freq) * swayAmp;
                    nPos.x += pullX + perpX * swayOffset;
                    nPos.y += pullY + perpY * swayOffset;
                    const velIdx = sim.positions.indexOf(nPos);
                    sim.velocities[velIdx].vx *= 0.3;
                    sim.velocities[velIdx].vy *= 0.3;
                  }
                }
              }
              currentLevel = nextLevel;
              if (currentLevel.length === 0) break;
            }
            const now = Date.now();
            for (const sp of sim.positions) {
              if (sp.id === draggedId) continue;
              const nodePhase = sp.id * 7.31 + now * 0.006;
              const nodeAmp = sp.isCategory ? 1.8 : 1.0;
              const ax = sp.anchorX - sp.x;
              const ay = sp.anchorY - sp.y;
              const aDist = Math.sqrt(ax * ax + ay * ay) || 1;
              const perpX = -ay / aDist;
              const perpY = ax / aDist;
              const floatOffset = Math.sin(nodePhase) * nodeAmp;
              sp.x += perpX * floatOffset;
              sp.y += perpY * floatOffset;
            }
          }
        }
        return;
      }
      setGraphPan({
        x: e.clientX - graphDragStart.x,
        y: e.clientY - graphDragStart.y
      });
      return;
    }
    e.stopPropagation();
    const fileRadius = 12;
    const catRadius = 26;
    const tagConnections = getTagConnections();
    let found: KbEntry | null = null;
    for (const sp of sim?.positions || []) {
      if (sp.isCategory) continue;
      const entry = allEntries.find(e => e.id === sp.id);
      if (!entry) continue;
      const degree = tagConnections.get(sp.id)?.length || 0;
      const r = fileRadius * (0.7 + degree * 0.05);
      const dist = Math.sqrt((mx - sp.x) ** 2 + (my - sp.y) ** 2);
      if (dist < Math.max(r * 1.3, 14)) {
        found = entry;
        break;
      }
    }
    setGraphHoveredNode(found);
    let foundCatId: number | null = null;
    if (!found) {
      for (const sp of sim?.positions || []) {
        if (!sp.isCategory) continue;
        const realCatId = sp.id === -1 ? -1 : -(sp.id + 1);
        let totalChildren = 0;
        if (realCatId === -1) {
          totalChildren = allEntries.filter(e => !e.category_id || !categories.some(c => c.id === e.category_id)).length;
        } else {
          const subCats = categories.filter(c => c.parent_id === realCatId).length;
          const subFiles = allEntries.filter(e => e.category_id === realCatId).length;
          totalChildren = subCats + subFiles;
        }
        const r = catRadius + Math.min(totalChildren * 0.6, 10);
        const dist = Math.sqrt((mx - sp.x) ** 2 + (my - sp.y) ** 2);
        if (dist < Math.max(r * 1.2, 20)) {
          foundCatId = realCatId;
          break;
        }
      }
    }
    setGraphHoveredCatId(foundCatId);
    container.style.cursor = found || foundCatId !== null ? 'pointer' : 'grab';
  };
  const handleGraphMouseUp = () => {
    const sim = simRef.current;
    if (sim) sim.draggingId = null;
    setGraphDragging(false);
  };
  const handleGraphWheel = (e: React.WheelEvent) => {
    e.preventDefault();
    setGraphZoom(prev => Math.max(0.15, Math.min(4, prev * (e.deltaY > 0 ? 0.92 : 1.08))));
  };
  const handleGraphClick = () => {
    if (graphHoveredCatId !== null) {
      setGraphExpandedCatIds(prev => {
        const next = new Set(prev);
        if (next.has(graphHoveredCatId)) {
          next.delete(graphHoveredCatId);
        } else {
          next.add(graphHoveredCatId);
        }
        return next;
      });
    }
  };
  const handleGraphDoubleClick = useCallback(() => {
    if (graphHoveredNode) {
      onClose();
      onSelectedIdChange({
        type: 'entry',
        id: graphHoveredNode.id
      });
      onOpenFileViewer(graphHoveredNode);
    }
    if (graphHoveredCatId !== null) {
      const sim = simRef.current;
      if (!sim) return;
      const catEntry = sim.positions.find(p => p.id === -(graphHoveredCatId + 1));
      if (catEntry) {
        setGraphPan({
          x: -catEntry.x,
          y: -catEntry.y
        });
        const catName = categories.find(c => c.id === graphHoveredCatId)?.name || t("knowledge.GraphView.k8");
        onShowStatus('success', t("knowledge.GraphView.k9", {
          catName: catName
        }));
      }
    }
  }, [graphHoveredNode, graphHoveredCatId, categories, onClose, onOpenFileViewer, onSelectedIdChange, onShowStatus]);
  return <div className={styles.graphOverlay}>
      <div className={styles.graphHeader}>
        <span className={styles.graphTitle}>{t("knowledge.GraphView.k10")}</span>
        <input className={styles.graphSearchInput} placeholder={t("knowledge.GraphView.k11")} value={graphSearchQuery} onChange={e => {
        const q = e.target.value;
        setGraphSearchQuery(q);
        if (!q.trim()) {
          setGraphFilteredNodeIds(null);
        } else {
          const filtered = new Set<number>();
          const lower = q.toLowerCase();
          const searchData = getGraphVisibleNodes().visibleEntries;
          for (const entry of searchData) {
            if (entry.name.toLowerCase().includes(lower)) filtered.add(entry.id);
          }
          setGraphFilteredNodeIds(filtered);
        }
      }} onKeyDown={e => e.stopPropagation()} />
        <div className={styles.graphLegend}>
          <span className={`${styles.legendBtn}${graphHighlightType === 'text' ? ` ${styles.legendBtnActive}` : ''}`} onClick={() => setGraphHighlightType(prev => prev === 'text' ? null : 'text')} title={t("knowledge.GraphView.k12")}><span className={styles.legendDot} style={{
            background: '#00F0FF'
          }} /> {t("knowledge.GraphView.k2")}</span>
          <span className={`${styles.legendBtn}${graphHighlightType === 'link' ? ` ${styles.legendBtnActive}` : ''}`} onClick={() => setGraphHighlightType(prev => prev === 'link' ? null : 'link')} title={t("knowledge.GraphView.k13")}><span className={styles.legendDot} style={{
            background: '#FFA500'
          }} /> {t("components.TextEditor.k9")}</span>
          <span className={`${styles.legendBtn}${graphHighlightType === 'file' ? ` ${styles.legendBtnActive}` : ''}`} onClick={() => setGraphHighlightType(prev => prev === 'file' ? null : 'file')} title={t("knowledge.GraphView.k14")}><span className={styles.legendDot} style={{
            background: '#00FF41'
          }} /> {t("knowledge.GraphView.k1")}</span>
          <span className={`${styles.legendBtn}${graphHighlightType === 'video' ? ` ${styles.legendBtnActive}` : ''}`} onClick={() => setGraphHighlightType(prev => prev === 'video' ? null : 'video')} title={t("knowledge.GraphView.k15")}><span className={styles.legendDot} style={{
            background: '#B026FF'
          }} /> {t("knowledge.GraphView.k3")}</span>
          <span className={`${styles.legendBtn}${graphHighlightType === 'category' ? ` ${styles.legendBtnActive}` : ''}`} onClick={() => setGraphHighlightType(prev => prev === 'category' ? null : 'category')} title={t("knowledge.GraphView.k16")}><span className={styles.legendArrow}>→</span> {t("knowledge.GraphView.k17")}</span>
        </div>
        <div className={styles.graphActions}>
          <button className={styles.btnSm} onClick={() => setGraphZoom(prev => Math.min(4, prev * 1.2))} title={t("knowledge.GraphView.k18")}>🔍+</button>
          <button className={styles.btnSm} onClick={() => setGraphZoom(prev => Math.max(0.15, prev * 0.8))} title={t("knowledge.GraphView.k19")}>🔍-</button>
          <button className={styles.btnSm} onClick={() => {
          setGraphPan({
            x: 0,
            y: 0
          });
          setGraphZoom(1);
          setGraphSearchQuery('');
          setGraphFilteredNodeIds(null);
          setGraphHighlightType(null);
          setGraphExpandedCatIds(new Set());
        }} title={t("common.reset")}>↺</button>
          <button className={styles.btnSm} onClick={() => {
          onClose();
          setGraphSearchQuery('');
          setGraphFilteredNodeIds(null);
          setGraphHighlightType(null);
        }}>{t("components.FloatingXin.k28")}</button>
        </div>
      </div>
      <div className={styles.graphInfoBar}>
        {(() => {
        const localData = getGraphVisibleNodes();
        const expandedCount = graphExpandedCatIds.size;
        if (selectedId) {
          const selName = selectedId.type === 'entry' ? allEntries.find(e => e.id === selectedId.id)?.name : categories.find(c => c.id === selectedId.id)?.name;
          return <>
                <span className={styles.graphInfoHighlight}>{t("knowledge.GraphView.k20")}{selName}」</span>
                <span className={styles.graphInfoItem}>{t("home.TodoPanel.k16")} {localData.visibleCats.length} {t("knowledge.GraphView.k21")} {localData.visibleEntries.length}</span>
                <span className={styles.graphInfoItem}>{t("knowledge.GraphView.k22")}</span>
              </>;
        }
        return <>
              <span className={styles.graphInfoItem}>{t("home.TodoPanel.k16")} {localData.visibleCats.length} {t("knowledge.GraphView.k21")} {localData.visibleEntries.length}</span>
              {graphHighlightType ? <span className={styles.graphInfoHighlight}>
                  {t("knowledge.GraphView.k23")} {graphHighlightType === 'category' ? t("knowledge.GraphView.k8") : getTypeLabel(graphHighlightType)}
                </span> : expandedCount > 0 ? <span className={styles.graphInfoHighlight}>{t("knowledge.GraphView.k24")} {expandedCount} {t("knowledge.GraphView.k25")}</span> : <span className={styles.graphInfoItem}>{t("knowledge.GraphView.k26")}</span>}
              <span className={styles.graphInfoItem}>{t("knowledge.GraphView.k27")}</span>
            </>;
      })()}
        {graphHoveredNode && <span className={styles.graphInfoHighlight}>
            🔗 {graphHoveredNode.name} · {getTypeLabel(graphHoveredNode.entry_type)}
            {(getTagConnections().get(graphHoveredNode.id)?.length ?? 0) > 0 && <> {t("knowledge.GraphView.k28")} {getTagConnections().get(graphHoveredNode.id)!.length} {t("knowledge.GraphView.k29")}</>}
          </span>}
        {graphHoveredCatId !== null && !graphHoveredNode && <span className={styles.graphInfoHighlight}>
            📁 {categories.find(c => c.id === graphHoveredCatId)?.name || t("knowledge.GraphView.k8")}
            {graphExpandedCatIds.has(graphHoveredCatId) ? t("knowledge.GraphView.k30") : t("knowledge.GraphView.k31")}
          </span>}
        {!graphHoveredNode && graphHoveredCatId === null && graphSearchQuery && graphFilteredNodeIds && <span className={styles.graphInfoHighlight}>{t("knowledge.GraphView.k32")} {graphFilteredNodeIds.size} {t("knowledge.GraphView.k29")}</span>}
      </div>
      <div ref={graphContainerRef} className={styles.graphCanvas} onMouseDown={handleGraphMouseDown} onMouseMove={handleGraphMouseMove} onMouseUp={handleGraphMouseUp} onMouseLeave={handleGraphMouseUp} onWheel={handleGraphWheel} onClick={handleGraphClick} onDoubleClick={handleGraphDoubleClick}>
        <canvas ref={canvasRef} className={styles.graphCanvasEl} />
        {graphHoveredNode && <div className={styles.graphTooltip}>
            <strong>{graphHoveredNode.name}</strong>
            <span className={styles.graphTooltipType}>{getTypeLabel(graphHoveredNode.entry_type)}</span>
          </div>}
      </div>
    </div>;
}