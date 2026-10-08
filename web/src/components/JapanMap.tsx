import { useRef, useState, type PointerEvent } from "react";
import japanMap from "../map/japanMap.json";
import type { Territory } from "../territory";

interface MapKuni {
  kuniId: number;
  d: string;
  /** ラベルの位置 */
  label: [number, number];
  /** 引き出し線の起点（ラベルを国の外に置く場合のみ） */
  anchor?: [number, number];
}

const MAP_KUNIS = japanMap.kunis as MapKuni[];

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
 * 12家の大名を色で塗り分けると判別できないため、自領のみをアクセント色、他家の領地は無彩色で塗り、
 * どの大名の領地かは国名の下のラベル・ホバー（同じ大名の領地をまとめて強調）・一覧表で示す。
 */
export function JapanMap({ territories, ownerCounts, highlightedOwnerId, onHighlightOwner }: Props) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [tooltip, setTooltip] = useState<Tooltip | null>(null);

  // ポインター位置にツールチップを表示し、同じ大名の領地を強調する
  const showTooltip = (event: PointerEvent<SVGPathElement>, territory: Territory) => {
    const rect = containerRef.current?.getBoundingClientRect();
    if (!rect) return;
    setTooltip({ x: event.clientX - rect.left, y: event.clientY - rect.top, territory });
    onHighlightOwner(territory.owner.id);
  };
  const hideTooltip = () => {
    setTooltip(null);
    onHighlightOwner(null);
  };

  return (
    <div className="map" ref={containerRef}>
      <svg
        viewBox={`0 0 ${japanMap.width} ${japanMap.height}`}
        role="img"
        aria-labelledby="map-title"
        className={highlightedOwnerId === null ? undefined : "has-highlight"}
      >
        <title id="map-title">勢力図（日本地図）</title>
        {MAP_KUNIS.map(({ kuniId, d }) => {
          const territory = territories.get(kuniId);
          const classes = [
            "kuni",
            territory?.mine ? "mine" : "other",
            territory && territory.owner.id === highlightedOwnerId ? "highlighted" : "",
          ];
          return (
            <path
              key={kuniId}
              d={d}
              className={classes.join(" ").trim()}
              data-kuni-id={kuniId}
              tabIndex={territory ? 0 : -1}
              aria-label={
                territory ? `${territory.kuniName}：${territory.owner.name}の領地` : undefined
              }
              onPointerMove={territory ? (e) => showTooltip(e, territory) : undefined}
              onPointerLeave={hideTooltip}
              onFocus={territory ? () => onHighlightOwner(territory.owner.id) : undefined}
              onBlur={() => onHighlightOwner(null)}
            />
          );
        })}
        {/* 強調中の大名の領地は、隣国に隠れないよう輪郭を最前面に重ねて描く */}
        {MAP_KUNIS.filter(
          ({ kuniId }) =>
            highlightedOwnerId !== null && territories.get(kuniId)?.owner.id === highlightedOwnerId,
        ).map(({ kuniId, d }) => (
          <path key={kuniId} d={d} className="kuni-outline" aria-hidden="true" />
        ))}
        {/* 国の外に置いたラベルの引き出し線 */}
        {MAP_KUNIS.filter((k) => k.anchor && territories.has(k.kuniId)).map(
          ({ kuniId, label: [lx, ly], anchor }) => (
            <g key={kuniId} className="leader" aria-hidden="true">
              <line x1={anchor![0]} y1={anchor![1]} x2={lx} y2={ly - 16} />
              <circle cx={anchor![0]} cy={anchor![1]} r={3} />
            </g>
          ),
        )}
        {MAP_KUNIS.map(({ kuniId, label: [x, y], anchor }) => {
          const territory = territories.get(kuniId);
          if (!territory) return null;
          return (
            <text
              key={kuniId}
              x={x}
              y={y}
              className={`kuni-label ${territory.mine && !anchor ? "on-mine" : ""} ${anchor ? "in-sea" : ""}`}
              aria-hidden="true"
            >
              <tspan x={x} className="kuni-name">
                {territory.kuniName}
              </tspan>
              <tspan x={x} dy="1.25em" className="kuni-owner">
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
        </div>
      )}
    </div>
  );
}
