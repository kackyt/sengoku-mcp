import { useRef, useState } from "react";
import { NEIGHBOR_PAIRS, neighborsOf } from "../adjacency";
import { daimyoColor } from "../daimyoColors";
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
const BOX = { width: 72, height: 36 };

interface Props {
  territories: Map<number, Territory>;
  highlightedOwnerId: number | null;
  onHighlightOwner: (ownerId: number | null) => void;
}

/**
 * 勢力図を接続図（国を箱、隣接を線で表した模式図）で表示する
 *
 * サイドバーの限られた幅でも読めるよう、地理的な正確さより読みやすさを優先した配置で
 * 「どの国がどの大名の領地か（色）」「どの国とどの国が隣接しているか（線）」を示す。
 * マウスではホバー、タッチ操作ではタップで国を選ぶと、隣接国と同じ大名の領地を強調する。
 */
export function SchematicMap({ territories, highlightedOwnerId, onHighlightOwner }: Props) {
  const [activeKuniId, setActiveKuniId] = useState<number | null>(null);
  const active = activeKuniId === null ? undefined : territories.get(activeKuniId);
  const activeNeighbors = new Set(activeKuniId === null ? [] : neighborsOf(activeKuniId));

  // 国を選択（null で解除）し、同じ大名の領地の強調を合わせる
  const select = (territory: Territory | null) => {
    setActiveKuniId(territory?.kuniId ?? null);
    onHighlightOwner(territory?.owner.id ?? null);
  };
  // タップ・キー操作では選択を切り替える（タッチ操作ではホバーがないため）
  const toggle = (territory: Territory) =>
    select(activeKuniId === territory.kuniId ? null : territory);
  // 直前のポインター種別（マウスのクリックはホバーで選択済みのため、切り替えに使わない）
  const lastPointerType = useRef<string | null>(null);

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
        <title id="schematic-title">勢力図（接続図）</title>

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
            territory.mine ? "mine" : "",
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
              aria-label={`${territory.kuniName}：${territory.owner.name}`}
              onPointerDown={(e) => {
                lastPointerType.current = e.pointerType;
              }}
              onClick={() => {
                if (lastPointerType.current !== "mouse") toggle(territory);
              }}
              onPointerEnter={(e) => e.pointerType === "mouse" && select(territory)}
              onPointerLeave={(e) => e.pointerType === "mouse" && select(null)}
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
                style={{ fill: daimyoColor(territory.owner.id) }}
              />
              <text x={cx} y={cy + 6} className="box-name">
                {territory.kuniName}
              </text>
            </g>
          );
        })}
      </svg>

      {/* 選択中の国の支配大名（タッチ操作でも読めるよう、図の下に出す） */}
      {active && (
        <p className="schematic-detail" role="status">
          <span
            className="swatch"
            style={{ background: daimyoColor(active.owner.id) }}
            aria-hidden="true"
          />
          <strong>{active.kuniName}</strong>
          <span>{active.owner.name}</span>
        </p>
      )}
    </div>
  );
}
