import { useMemo, useState } from "react";
import type { KuniStatus, MyStatus } from "../api/client";
import { daimyoColor } from "../daimyoColors";
import { buildTerritories, countByOwner } from "../territory";
import { MapPanel } from "./MapPanel";

const numberFormat = new Intl.NumberFormat("ja-JP");

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

interface Props {
  status: MyStatus;
}

/** 自国の状況・ターン数・勢力図を表示する（説明文は置かず、色と数字で示す） */
export function StatusView({ status }: Props) {
  const [highlightedOwnerId, setHighlightedOwnerId] = useState<number | null>(null);
  const territories = useMemo(() => buildTerritories(status), [status]);

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

  const kpis = [
    { label: "領地", value: status.my_kunis.length },
    { label: "兵", value: sumBy(status.my_kunis, "hei") },
    { label: "金", value: sumBy(status.my_kunis, "kin") },
    { label: "米", value: sumBy(status.my_kunis, "kome") },
  ];

  return (
    <main className="status">
      <header className="status-header">
        <p className="eyebrow">
          <span
            className="swatch"
            style={{ background: daimyoColor(status.daimyo.id) }}
            aria-hidden="true"
          />
          {status.daimyo.name}
        </p>
        <h1 className="hero">
          第<span className="hero-number">{status.turn}</span>ターン
        </h1>
      </header>

      <section className="kpis" aria-label="自国の合計">
        {kpis.map((kpi) => (
          <div className="kpi" key={kpi.label}>
            <span className="kpi-label">{kpi.label}</span>
            <span className="kpi-value">{numberFormat.format(kpi.value)}</span>
          </div>
        ))}
      </section>

      <div className="layout">
        <section className="card map-card" aria-label="勢力図">
          <MapPanel
            territories={territories}
            highlightedOwnerId={highlightedOwnerId}
            onHighlightOwner={setHighlightedOwnerId}
          />
          <ul className="legend" aria-label="大名">
            {legend.map((entry) => (
              <li
                key={entry.id}
                className={[
                  entry.mine ? "mine" : "",
                  entry.id === highlightedOwnerId ? "highlighted" : "",
                ]
                  .join(" ")
                  .trim() || undefined}
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
        </section>

        <section className="card" aria-label="自領">
          <div className="table-scroll">
            <table>
              <thead>
                <tr>
                  <th scope="col">国</th>
                  <th scope="col">兵</th>
                  <th scope="col">金</th>
                  <th scope="col">米</th>
                  <th scope="col">人口</th>
                  <th scope="col">石高</th>
                  <th scope="col">町</th>
                  <th scope="col">忠誠</th>
                </tr>
              </thead>
              <tbody>
                {status.my_kunis.map((k) => (
                  <tr key={k.id}>
                    <th scope="row">{k.name}</th>
                    <td>{numberFormat.format(k.hei)}</td>
                    <td>{numberFormat.format(k.kin)}</td>
                    <td>{numberFormat.format(k.kome)}</td>
                    <td>{numberFormat.format(k.jinko)}</td>
                    <td>{numberFormat.format(k.kokudaka)}</td>
                    <td>{numberFormat.format(k.machi)}</td>
                    <td>{k.tyu}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </section>
      </div>
    </main>
  );
}
