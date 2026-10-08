import { useEffect, useRef, useState } from "react";
import japanMap from "../map/japanMap.json";
import type { Territory } from "../territory";
import { JapanMap } from "./JapanMap";
import { SchematicMap } from "./SchematicMap";

/** 日本地図の表示幅がこれより狭くなる場合は、既定で接続図を表示する（CSSピクセル） */
export const SCHEMATIC_BREAKPOINT = 460;

/** 日本地図の高さの上限（画面の高さに対する割合。styles.css の .map svg の max-height と揃える） */
const MAP_MAX_HEIGHT_RATIO = 0.78;

/**
 * 日本地図を表示した場合の実際の表示幅
 *
 * 地図は縦長のため、画面の高さが低いと幅いっぱいには表示されない（高さで縮む）。
 */
export function effectiveMapWidth(containerWidth: number, viewportHeight: number): number {
  const widthByHeight = viewportHeight * MAP_MAX_HEIGHT_RATIO * (japanMap.width / japanMap.height);
  return Math.min(containerWidth, widthByHeight);
}

type ViewMode = "map" | "schematic";

interface Props {
  territories: Map<number, Territory>;
  ownerCounts: Map<number, number>;
  highlightedOwnerId: number | null;
  onHighlightOwner: (ownerId: number | null) => void;
}

/** 日本地図を表示した場合の実際の表示幅を監視する（ResizeObserver がない環境では null） */
function useEffectiveMapWidth(ref: React.RefObject<HTMLDivElement | null>): number | null {
  const [width, setWidth] = useState<number | null>(null);
  useEffect(() => {
    const element = ref.current;
    if (!element || typeof ResizeObserver === "undefined") return;
    // コンテナの幅の変化に加え、画面の高さの変化（ウィンドウのリサイズ）にも追従する
    const update = () =>
      setWidth(effectiveMapWidth(element.getBoundingClientRect().width, window.innerHeight));
    const observer = new ResizeObserver(update);
    observer.observe(element);
    window.addEventListener("resize", update);
    return () => {
      observer.disconnect();
      window.removeEventListener("resize", update);
    };
  }, [ref]);
  return width;
}

/**
 * 勢力図の表示切り替え（日本地図 / 接続図）
 *
 * 既定では表示幅に応じて自動で選び（狭い画面では接続図）、利用者が明示的に切り替えたらそれに従う。
 */
export function MapPanel(props: Props) {
  const ref = useRef<HTMLDivElement>(null);
  const width = useEffectiveMapWidth(ref);
  const [chosen, setChosen] = useState<ViewMode | null>(null);
  const auto: ViewMode = width !== null && width < SCHEMATIC_BREAKPOINT ? "schematic" : "map";
  const mode = chosen ?? auto;

  const select = (next: ViewMode) => {
    // 表示を切り替えたら強調表示は解除する
    props.onHighlightOwner(null);
    setChosen(next);
  };

  return (
    <div ref={ref}>
      <div className="view-switch" role="group" aria-label="表示の切り替え">
        <button
          type="button"
          className="secondary"
          aria-pressed={mode === "map"}
          onClick={() => select("map")}
        >
          日本地図
        </button>
        <button
          type="button"
          className="secondary"
          aria-pressed={mode === "schematic"}
          onClick={() => select("schematic")}
        >
          接続図
        </button>
      </div>
      {mode === "map" ? <JapanMap {...props} /> : <SchematicMap {...props} />}
    </div>
  );
}
