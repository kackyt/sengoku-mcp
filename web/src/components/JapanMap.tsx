import { useEffect, useRef, useState, type PointerEvent } from "react";
import { neighborNames, neighborsOf } from "../adjacency";
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
 * 地図は表示幅に合わせて拡大縮小されるため、文字や点の大きさを SVG 座標で固定すると
 * 小さな画面・低解像度で読めなくなる。画面上の大きさを固定し、縮尺で SVG 座標に換算する。
 */
const SCREEN_PX = {
  name: 14,
  owner: 12,
  nameCompact: 12,
  ownerCompact: 10,
  nodeRadius: 4.5,
  nodeRadiusActive: 6.5,
  labelGap: 4,
};

/** 地図の表示幅がこれより狭い場合はラベルを小さめにする（CSSピクセル） */
const COMPACT_WIDTH = 520;

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
  const scale = Math.min(size.width / japanMap.width, size.height / japanMap.height);
  return { scale, compact: japanMap.width * scale < COMPACT_WIDTH };
}

interface Tooltip {
  x: number;
  y: number;
  territory: Territory;
}

interface Props {
  /** 国ID → 勢力情報 */
  territories: Map<number, Territory>;
  /** 大名ID → 領地数 */
  ownerCounts: Map<number, number>;
  /** 強調表示中の大名ID（同じ大名の領地をまとめて強調する） */
  highlightedOwnerId: number | null;
  onHighlightOwner: (ownerId: number | null) => void;
}

/**
 * 勢力図を日本地図で表示する
 *
 * - 12家の大名を色で塗り分けると判別できないため、自領のみをアクセント色、他家の領地は無彩色で塗り、
 *   どの大名の領地かは国名の下のラベル・ホバー・一覧表で示す。
 * - 隣接する（行き来できる）国同士を、国の中心点を結ぶ接続線で示す。海を挟む接続も含む。
 * - 国にホバー（タップ・フォーカス）すると、その国の接続線と隣接国、同じ大名の領地を強調する。
 */
export function JapanMap({ territories, ownerCounts, highlightedOwnerId, onHighlightOwner }: Props) {
  const containerRef = useRef<HTMLDivElement>(null);
  const svgRef = useRef<SVGSVGElement>(null);
  const [tooltip, setTooltip] = useState<Tooltip | null>(null);
  const [activeKuniId, setActiveKuniId] = useState<number | null>(null);
  const { scale, compact } = useMapScale(svgRef);

  // 画面上の大きさ（px）を SVG 座標に換算する
  const toSvg = (px: number) => px / scale;
  const nameSize = toSvg(compact ? SCREEN_PX.nameCompact : SCREEN_PX.name);
  const ownerSize = toSvg(compact ? SCREEN_PX.ownerCompact : SCREEN_PX.owner);

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
        <title id="map-title">勢力図（日本地図）。線で結ばれた国同士が隣接しています。</title>

        {/* 1. 国の領域 */}
        {MAP_KUNIS.map(({ kuniId, d }) => {
          const territory = territories.get(kuniId);
          const classes = [
            "kuni",
            territory?.mine ? "mine" : "other",
            territory && territory.owner.id === highlightedOwnerId ? "highlighted" : "",
            activeNeighbors.has(kuniId) ? "neighbor" : "",
          ];
          return (
            <path
              key={kuniId}
              d={d}
              className={classes.join(" ").trim()}
              data-kuni-id={kuniId}
              tabIndex={territory ? 0 : -1}
              aria-label={
                territory
                  ? `${territory.kuniName}：${territory.owner.name}の領地。隣接：${neighborNames(kuniId, territories)}`
                  : undefined
              }
              onPointerMove={territory ? (e) => onPointer(e, territory) : undefined}
              onPointerDown={territory ? (e) => onPointer(e, territory) : undefined}
              onPointerLeave={deactivate}
              onFocus={territory ? () => activate(territory) : undefined}
              onBlur={deactivate}
            />
          );
        })}

        {/* 2. 強調中の大名の領地の輪郭（隣国に隠れないよう最前面に重ねる） */}
        {MAP_KUNIS.filter(
          ({ kuniId }) =>
            highlightedOwnerId !== null && territories.get(kuniId)?.owner.id === highlightedOwnerId,
        ).map(({ kuniId, d }) => (
          <path key={kuniId} d={d} className="kuni-outline" aria-hidden="true" />
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
          const territory = territories.get(kuniId)!;
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
                className={`node ${territory.mine ? "mine" : ""} ${active ? "active" : ""}`}
                cx={node[0]}
                cy={node[1]}
                r={toSvg(active ? SCREEN_PX.nodeRadiusActive : SCREEN_PX.nodeRadius)}
              />
            </g>
          );
        })}

        {/* 5. 国名と大名名のラベル（通常は中心点の直下、一部は海上） */}
        {MAP_KUNIS.map(({ kuniId, node, label }) => {
          const territory = territories.get(kuniId);
          if (!territory) return null;
          const [x, y] = label ?? [
            node[0],
            node[1] + toSvg(SCREEN_PX.nodeRadius + SCREEN_PX.labelGap),
          ];
          const halo = label ? "in-sea" : territory.mine ? "on-mine" : "";
          return (
            <text
              key={kuniId}
              x={x}
              y={y}
              className={`kuni-label ${halo}`.trim()}
              style={{ strokeWidth: toSvg(3) }}
              aria-hidden="true"
            >
              <tspan x={x} dy={nameSize * 0.9} className="kuni-name" fontSize={nameSize}>
                {territory.kuniName}
              </tspan>
              <tspan x={x} dy={ownerSize * 1.2} className="kuni-owner" fontSize={ownerSize}>
                {territory.owner.name}
              </tspan>
            </text>
          );
        })}
      </svg>
      {tooltip && (
        <div className="tooltip" style={{ left: tooltip.x, top: tooltip.y }} role="status">
          <strong>{tooltip.territory.kuniName}</strong>
          <span>
            {tooltip.territory.owner.name}家の領地
            {tooltip.territory.mine ? "（自領）" : ""}
          </span>
          <span className="muted">
            {tooltip.territory.owner.name}家の領地数：
            {ownerCounts.get(tooltip.territory.owner.id) ?? 0}国
          </span>
          <span className="muted">
            隣接：{neighborNames(tooltip.territory.kuniId, territories)}
          </span>
        </div>
      )}
    </div>
  );
}
