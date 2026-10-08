import { useEffect, useRef, useState, type PointerEvent } from "react";
import { neighborsOf } from "../adjacency";
import { daimyoColor } from "../daimyoColors";
import japanMap from "../map/japanMap.json";
import type { Territory } from "../territory";

interface MapKuni {
  kuniId: number;
  d: string;
  /** 国の中心点（接続線の端点・ラベルの基準） */
  node: [number, number];
  /** 海上に置くラベルの位置（引き出し線で中心点と結ぶ国のみ） */
  label?: [number, number];
}

interface MapEdge {
  a: number;
  b: number;
  d: string;
}

const MAP_KUNIS = japanMap.kunis as MapKuni[];
const MAP_EDGES = japanMap.edges as MapEdge[];

/**
 * 画面上での大きさ（CSSピクセル）
 *
 * 地図は表示サイズに合わせて拡大縮小されるため、文字や点の大きさを SVG 座標で固定すると
 * 小さな画面・低解像度で読めなくなる。画面上の大きさを固定し、縮尺で SVG 座標に換算する。
 */
const SCREEN_PX = {
  name: 14,
  nodeRadius: 4,
  nodeRadiusActive: 6,
  labelGap: 4,
  halo: 3,
};

/**
 * SVG の表示サイズを監視し、SVG座標1単位あたりの画面ピクセル数を返す
 *
 * 地図は幅だけでなく画面の高さ（max-height）でも縮むため、幅・高さの両方から実際の縮尺を求める。
 */
function useMapScale(ref: React.RefObject<SVGSVGElement | null>) {
  const [size, setSize] = useState({ width: japanMap.width, height: japanMap.height });
  useEffect(() => {
    const element = ref.current;
    if (!element || typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(([entry]) => {
      const rect = entry?.contentRect;
      if (rect && rect.width > 0 && rect.height > 0) {
        setSize({ width: rect.width, height: rect.height });
      }
    });
    observer.observe(element);
    return () => observer.disconnect();
  }, [ref]);
  // viewBox は縦横比を保って収まるため、縮尺は幅・高さの小さい方で決まる
  return Math.min(size.width / japanMap.width, size.height / japanMap.height);
}

interface Tooltip {
  x: number;
  y: number;
  territory: Territory;
}

interface Props {
  /** 国ID → 勢力情報 */
  territories: Map<number, Territory>;
  /** 強調表示中の大名ID（同じ大名の領地をまとめて強調する） */
  highlightedOwnerId: number | null;
  onHighlightOwner: (ownerId: number | null) => void;
}

/**
 * 勢力図を日本地図で表示する
 *
 * - 国を支配大名の色で塗り、自領は太い輪郭で示す。地図上の文字は国名のみ。
 * - 隣接する（行き来できる）国同士を、国の中心点を結ぶ接続線で示す。海を挟む接続も含む。
 * - 国にホバー（タップ・フォーカス）すると、その国の接続線と隣接国、同じ大名の領地を強調し、
 *   支配大名をツールチップで示す。
 */
export function JapanMap({ territories, highlightedOwnerId, onHighlightOwner }: Props) {
  const containerRef = useRef<HTMLDivElement>(null);
  const svgRef = useRef<SVGSVGElement>(null);
  const [tooltip, setTooltip] = useState<Tooltip | null>(null);
  const [activeKuniId, setActiveKuniId] = useState<number | null>(null);
  const scale = useMapScale(svgRef);

  // 画面上の大きさ（px）を SVG 座標に換算する
  const toSvg = (px: number) => px / scale;
  const nameSize = toSvg(SCREEN_PX.name);
  const activeNeighbors = new Set(activeKuniId === null ? [] : neighborsOf(activeKuniId));

  // 国を選択状態にし、ツールチップを表示して同じ大名の領地を強調する
  const activate = (territory: Territory, clientX?: number, clientY?: number) => {
    setActiveKuniId(territory.kuniId);
    onHighlightOwner(territory.owner.id);
    const rect = containerRef.current?.getBoundingClientRect();
    if (rect && clientX !== undefined && clientY !== undefined) {
      setTooltip({ x: clientX - rect.left, y: clientY - rect.top, territory });
    }
  };
  const deactivate = () => {
    setActiveKuniId(null);
    setTooltip(null);
    onHighlightOwner(null);
  };
  const onPointer = (event: PointerEvent<SVGPathElement>, territory: Territory) =>
    activate(territory, event.clientX, event.clientY);

  // 自領と、強調中の大名の領地は輪郭を最前面に重ねて描く（隣国に隠れないように）
  const outlined = MAP_KUNIS.filter(({ kuniId }) => {
    const t = territories.get(kuniId);
    return t && (t.mine || t.owner.id === highlightedOwnerId);
  });

  const svgClasses = [
    highlightedOwnerId !== null ? "has-highlight" : "",
    activeKuniId !== null ? "has-active" : "",
  ];

  return (
    <div className="map" ref={containerRef}>
      <svg
        ref={svgRef}
        viewBox={`0 0 ${japanMap.width} ${japanMap.height}`}
        role="img"
        aria-labelledby="map-title"
        className={svgClasses.join(" ").trim() || undefined}
      >
        <title id="map-title">勢力図</title>

        {/* 1. 国の領域（支配大名の色） */}
        {MAP_KUNIS.map(({ kuniId, d }) => {
          const territory = territories.get(kuniId);
          const classes = [
            "kuni",
            territory?.mine ? "mine" : "",
            territory && territory.owner.id === highlightedOwnerId ? "highlighted" : "",
            activeNeighbors.has(kuniId) ? "neighbor" : "",
          ];
          return (
            <path
              key={kuniId}
              d={d}
              className={classes.join(" ").trim()}
              style={territory ? { fill: daimyoColor(territory.owner.id) } : undefined}
              data-kuni-id={kuniId}
              data-owner-id={territory?.owner.id}
              tabIndex={territory ? 0 : -1}
              aria-label={territory ? `${territory.kuniName}：${territory.owner.name}` : undefined}
              onPointerMove={territory ? (e) => onPointer(e, territory) : undefined}
              onPointerDown={territory ? (e) => onPointer(e, territory) : undefined}
              onPointerLeave={deactivate}
              onFocus={territory ? () => activate(territory) : undefined}
              onBlur={deactivate}
            />
          );
        })}

        {/* 2. 自領・強調中の大名の領地の輪郭 */}
        {outlined.map(({ kuniId, d }) => (
          <path
            key={kuniId}
            d={d}
            className={`kuni-outline ${territories.get(kuniId)?.mine ? "mine" : ""}`.trim()}
            aria-hidden="true"
          />
        ))}

        {/* 3. 隣接を表す接続線 */}
        <g className="edges" aria-hidden="true">
          {MAP_EDGES.map(({ a, b, d }) => {
            const active = activeKuniId === a || activeKuniId === b;
            return (
              <path
                key={`${a}-${b}`}
                d={d}
                className={`edge ${active ? "active" : ""}`.trim()}
                data-edge={`${a}-${b}`}
              />
            );
          })}
        </g>

        {/* 4. 国の中心点と、海上ラベルへの引き出し線 */}
        {MAP_KUNIS.filter((k) => territories.has(k.kuniId)).map(({ kuniId, node, label }) => {
          const active = kuniId === activeKuniId || activeNeighbors.has(kuniId);
          return (
            <g key={kuniId} aria-hidden="true">
              {label && (
                <line
                  className="leader"
                  x1={node[0]}
                  y1={node[1]}
                  x2={label[0]}
                  y2={label[1] - toSvg(SCREEN_PX.labelGap)}
                />
              )}
              <circle
                className={`node ${active ? "active" : ""}`.trim()}
                cx={node[0]}
                cy={node[1]}
                r={toSvg(active ? SCREEN_PX.nodeRadiusActive : SCREEN_PX.nodeRadius)}
              />
            </g>
          );
        })}

        {/* 5. 国名（通常は中心点の直下、一部は海上） */}
        {MAP_KUNIS.map(({ kuniId, node, label }) => {
          const territory = territories.get(kuniId);
          if (!territory) return null;
          const [x, y] = label ?? [
            node[0],
            node[1] + toSvg(SCREEN_PX.nodeRadius + SCREEN_PX.labelGap),
          ];
          return (
            <text
              key={kuniId}
              x={x}
              y={y}
              dy={nameSize * 0.9}
              className="kuni-label"
              fontSize={nameSize}
              style={{ strokeWidth: toSvg(SCREEN_PX.halo) }}
              aria-hidden="true"
            >
              {territory.kuniName}
            </text>
          );
        })}
      </svg>
      {tooltip && (
        <div className="tooltip" style={{ left: tooltip.x, top: tooltip.y }} role="status">
          <span
            className="swatch"
            style={{ background: daimyoColor(tooltip.territory.owner.id) }}
            aria-hidden="true"
          />
          <strong>{tooltip.territory.kuniName}</strong>
          <span>{tooltip.territory.owner.name}</span>
        </div>
      )}
    </div>
  );
}
