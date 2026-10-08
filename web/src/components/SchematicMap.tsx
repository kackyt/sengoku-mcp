import { useState } from "react";
import { NEIGHBOR_PAIRS, neighborNames, neighborsOf } from "../adjacency";
import type { Territory } from "../territory";

/**
 * 接続図での各国の位置（SVG座標、国の箱の中心）
 *
 * 地理的なおおよその位置関係（北東→南西）を保ちつつ、小さな画面でも国の箱・接続線が
 * 重ならないよう等間隔に近づけて配置している。
 */
const LAYOUT: Record<number, [number, number]> = {
  1: [270, 40], // 蝦夷
  2: [270, 130], // 奥州
  3: [170, 180], // 越州
  5: [280, 240], // 武蔵
  4: [190, 260], // 甲信
  8: [95, 295], // 山城
  6: [265, 330], // 三河
  7: [180, 340], // 尾張
  9: [60, 380], // 安芸
  10: [155, 430], // 四国
  11: [55, 460], // 豊後
  12: [55, 535], // 薩摩
};

/** 海を挟む接続（破線で描く） */
const SEA_ROUTES = new Set(["1-2", "9-10", "9-11"]);

const VIEW_WIDTH = 340;
const VIEW_HEIGHT = 575;
const BOX = { width: 78, height: 42 };

interface Props {
  territories: Map<number, Territory>;
  ownerCounts: Map<number, number>;
  highlightedOwnerId: number | null;
  onHighlightOwner: (ownerId: number | null) => void;
}

/**
 * 勢力図を接続図（国を箱、隣接を線で表した模式図）で表示する
 *
 * 小さな画面・低解像度では日本地図上の国名が重なって読めないため、地理的な正確さより
 * 読みやすさを優先した配置で「どの国がどの大名の領地か」「どの国とどの国が隣接しているか」を示す。
 */
export function SchematicMap({ territories, ownerCounts, highlightedOwnerId, onHighlightOwner }: Props) {
  const [activeKuniId, setActiveKuniId] = useState<number | null>(null);
  const active = activeKuniId === null ? undefined : territories.get(activeKuniId);
  const activeNeighbors = new Set(activeKuniId === null ? [] : neighborsOf(activeKuniId));

  // タップ・クリックで選択を切り替える（タッチ操作ではホバーがないため）
  const toggle = (territory: Territory) => {
    const next = activeKuniId === territory.kuniId ? null : territory.kuniId;
    setActiveKuniId(next);
    onHighlightOwner(next === null ? null : territory.owner.id);
  };

  const svgClasses = [
    highlightedOwnerId !== null ? "has-highlight" : "",
    activeKuniId !== null ? "has-active" : "",
  ];

  return (
    <div className="schematic">
      <svg
        viewBox={`0 0 ${VIEW_WIDTH} ${VIEW_HEIGHT}`}
        role="img"
        aria-labelledby="schematic-title"
        className={svgClasses.join(" ").trim() || undefined}
      >
        <title id="schematic-title">
          勢力図（接続図）。線で結ばれた国同士が隣接しています。破線は海を挟む接続です。
        </title>

        {/* 1. 隣接を表す接続線 */}
        {NEIGHBOR_PAIRS.map(([a, b]) => {
          const [pa, pb] = [LAYOUT[a], LAYOUT[b]];
          if (!pa || !pb) return null;
          const key = `${a}-${b}`;
          const classes = [
            "edge",
            SEA_ROUTES.has(key) ? "sea" : "",
            activeKuniId === a || activeKuniId === b ? "active" : "",
          ];
          return (
            <line
              key={key}
              x1={pa[0]}
              y1={pa[1]}
              x2={pb[0]}
              y2={pb[1]}
              className={classes.join(" ").trim()}
              data-edge={key}
              aria-hidden="true"
            />
          );
        })}

        {/* 2. 国の箱（国名・大名名） */}
        {Object.entries(LAYOUT).map(([id, [cx, cy]]) => {
          const territory = territories.get(Number(id));
          if (!territory) return null;
          const classes = [
            "box",
            territory.mine ? "mine" : "other",
            territory.owner.id === highlightedOwnerId ? "highlighted" : "",
            activeNeighbors.has(territory.kuniId) ? "neighbor" : "",
            territory.kuniId === activeKuniId ? "active" : "",
          ];
          return (
            <g
              key={id}
              className={classes.join(" ").trim()}
              data-kuni-id={id}
              role="button"
              tabIndex={0}
              aria-pressed={territory.kuniId === activeKuniId}
              aria-label={`${territory.kuniName}：${territory.owner.name}の領地。隣接：${neighborNames(territory.kuniId, territories)}`}
              onClick={() => toggle(territory)}
              onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  toggle(territory);
                }
              }}
            >
              <rect
                x={cx - BOX.width / 2}
                y={cy - BOX.height / 2}
                width={BOX.width}
                height={BOX.height}
                rx={8}
              />
              <text x={cx} y={cy - 4} className="box-name">
                {territory.kuniName}
              </text>
              <text x={cx} y={cy + 13} className="box-owner">
                {territory.owner.name}
              </text>
            </g>
          );
        })}
      </svg>

      {/* 選択中の国の詳細（タッチ操作でも読めるよう、ツールチップではなく図の下に出す） */}
      <p className="schematic-detail" role="status">
        {active ? (
          <>
            <strong>{active.kuniName}</strong>：{active.owner.name}家の領地
            {active.mine ? "（自領）" : ""}・{active.owner.name}家は
            {ownerCounts.get(active.owner.id) ?? 0}国／隣接：
            {neighborNames(active.kuniId, territories)}
          </>
        ) : (
          <span className="muted">国をタップすると、隣接する国を強調します。</span>
        )}
      </p>
    </div>
  );
}
