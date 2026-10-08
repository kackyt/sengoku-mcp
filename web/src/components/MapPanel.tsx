import { useEffect, useRef, useState, type ReactNode } from "react";
import japanMap from "../map/japanMap.json";
import type { Territory } from "../territory";
import { JapanMap } from "./JapanMap";
import { SchematicMap } from "./SchematicMap";

/** 日本地図の表示幅がこれより狭くなる場合は、既定で接続図を表示する（CSSピクセル） */
export const SCHEMATIC_BREAKPOINT = 460;

/**
 * 地図以外が縦に使う高さ（上部バーと余白、CSSピクセル）
 * styles.css の --chrome-height と揃える。地図の高さの上限は「画面の高さ - この値」。
 */
const CHROME_HEIGHT = 64;

/**
 * 日本地図を表示した場合の実際の表示幅
 *
 * 地図は縦長のため、画面の高さが低いと幅いっぱいには表示されない（高さで縮む）。
 */
export function effectiveMapWidth(
  containerWidth: number,
  viewportHeight: number,
  reserve = 0,
): number {
  const widthByHeight =
    Math.max(0, viewportHeight - CHROME_HEIGHT - reserve) * (japanMap.width / japanMap.height);
  return Math.min(containerWidth, widthByHeight);
}

type ViewMode = "map" | "schematic";

interface Props {
  territories: Map<number, Territory>;
  highlightedOwnerId: number | null;
  onHighlightOwner: (ownerId: number | null) => void;
  /** 地図の右下（海の上の空き領域）に重ねて表示する要素（凡例） */
  overlay?: ReactNode;
}

/** 日本地図を表示した場合の実際の表示幅を監視する（ResizeObserver がない環境では null） */
function useEffectiveMapWidth(ref: React.RefObject<HTMLDivElement | null>): number | null {
  const [width, setWidth] = useState<number | null>(null);
  useEffect(() => {
    const element = ref.current;
    if (!element || typeof ResizeObserver === "undefined") return;
    // コンテナの幅の変化に加え、画面の高さの変化（ウィンドウのリサイズ）にも追従する
    // 地図の下に自領カードを並べるレイアウトでは、その高さ（--map-reserve）も差し引く
    const update = () => {
      const reserve = parseFloat(getComputedStyle(element).getPropertyValue("--map-reserve")) || 0;
      setWidth(
        effectiveMapWidth(element.getBoundingClientRect().width, window.innerHeight, reserve),
      );
    };
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
export function MapPanel({ overlay, ...props }: Props) {
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

  // 切り替えボタンは左上、凡例は右下に重ねる（どちらも地図・接続図の海の上の空き領域）
  return (
    <div ref={ref} className="map-panel">
      {mode === "map" ? <JapanMap {...props} /> : <SchematicMap {...props} />}
      <div className="view-switch" role="group" aria-label="表示の切り替え">
        <button
          type="button"
          className="secondary"
          aria-pressed={mode === "map"}
          onClick={() => select("map")}
        >
          地図
        </button>
        <button
          type="button"
          className="secondary"
          aria-pressed={mode === "schematic"}
          onClick={() => select("schematic")}
        >
          接続
        </button>
      </div>
      {overlay && <div className="map-overlay">{overlay}</div>}
    </div>
  );
}
