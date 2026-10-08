import { useMemo, useState } from "react";
import type { KuniStatus, MyStatus } from "../api/client";
import { daimyoColor } from "../daimyoColors";
import { turnToDate } from "../gameDate";
import { buildTerritories, countByOwner } from "../territory";
import { MapPanel } from "./MapPanel";

const numberFormat = new Intl.NumberFormat("ja-JP");
const fmt = (n: number) => numberFormat.format(n);

/** 自領の資源を合計する（表示用の集計のみ） */
function sumBy(kunis: KuniStatus[], key: "hei" | "kin" | "kome"): number {
  return kunis.reduce((sum, k) => sum + k[key], 0);
}

/** 凡例に並べる大名（自国を先頭に、領地数の多い順） */
interface LegendEntry {
  id: number;
  name: string;
  count: number;
  mine: boolean;
}

/** 自領カードに並べる項目（1文字の見出しと値） */
const KUNI_STATS: ReadonlyArray<{ key: keyof KuniStatus; label: string; title: string }> = [
  { key: "hei", label: "兵", title: "兵" },
  { key: "kin", label: "金", title: "金" },
  { key: "kome", label: "米", title: "米" },
  { key: "jinko", label: "人", title: "人口" },
  { key: "kokudaka", label: "石", title: "石高" },
  { key: "machi", label: "町", title: "町" },
];

interface Props {
  status: MyStatus;
}

/**
 * 自国の状況・年と季節・勢力図を表示する
 *
 * ブラウザの横半分ほどのサイドバーで使う想定。上部の1行に大名・年月・合計を詰め、
 * 地図は画面の高さに合わせ、自領は小さなカードで地図の横（狭い場合は下）に並べる。
 */
export function StatusView({ status }: Props) {
  const [highlightedOwnerId, setHighlightedOwnerId] = useState<number | null>(null);
  const territories = useMemo(() => buildTerritories(status), [status]);
  const { year, season } = turnToDate(status.turn);

  const legend = useMemo<LegendEntry[]>(() => {
    const counts = countByOwner(territories.values());
    const names = new Map([...territories.values()].map((t) => [t.owner.id, t.owner.name]));
    return [...counts.entries()]
      .map(([id, count]) => ({
        id,
        name: names.get(id) ?? "",
        count,
        mine: id === status.daimyo.id,
      }))
      .sort((a, b) => Number(b.mine) - Number(a.mine) || b.count - a.count || a.id - b.id);
  }, [territories, status.daimyo.id]);

  const totals = [
    { label: "領地", value: status.my_kunis.length },
    { label: "兵", value: sumBy(status.my_kunis, "hei") },
    { label: "金", value: sumBy(status.my_kunis, "kin") },
    { label: "米", value: sumBy(status.my_kunis, "kome") },
  ];

  return (
    <main className="status">
      <header className="topbar">
        <span className="daimyo">
          <span
            className="swatch"
            style={{ background: daimyoColor(status.daimyo.id) }}
            aria-hidden="true"
          />
          {status.daimyo.name}
        </span>
        <h1 className="date" aria-label={`${year}年${season}`}>
          {year}
          <small>年</small>
          {season}
        </h1>
        <dl className="totals">
          {totals.map((t) => (
            <div key={t.label}>
              <dt>{t.label}</dt>
              <dd>{fmt(t.value)}</dd>
            </div>
          ))}
        </dl>
      </header>

      <div className="content">
        <section className="map-card" aria-label="勢力図">
          <MapPanel
            territories={territories}
            highlightedOwnerId={highlightedOwnerId}
            onHighlightOwner={setHighlightedOwnerId}
            overlay={
              <ul className="legend" aria-label="大名">
                {legend.map((entry) => (
                  <li
                    key={entry.id}
                    className={
                      [entry.mine ? "mine" : "", entry.id === highlightedOwnerId ? "highlighted" : ""]
                        .join(" ")
                        .trim() || undefined
                    }
                    onPointerEnter={() => setHighlightedOwnerId(entry.id)}
                    onPointerLeave={() => setHighlightedOwnerId(null)}
                  >
                    <span
                      className="swatch"
                      style={{ background: daimyoColor(entry.id) }}
                      aria-hidden="true"
                    />
                    {entry.name}
                    <span className="legend-count">{entry.count}</span>
                  </li>
                ))}
              </ul>
            }
          />
        </section>

        <ul className="kuni-cards" aria-label="自領">
          {status.my_kunis.map((k) => (
            <li key={k.id} className="kuni-card">
              <div className="kuni-card-head">
                <strong>{k.name}</strong>
                <span title="忠誠">
                  <small>忠</small>
                  {k.tyu}
                </span>
              </div>
              <dl>
                {KUNI_STATS.map((stat) => (
                  <div key={stat.key} title={stat.title}>
                    <dt>{stat.label}</dt>
                    <dd>{fmt(k[stat.key] as number)}</dd>
                  </div>
                ))}
              </dl>
            </li>
          ))}
        </ul>
      </div>
    </main>
  );
}
