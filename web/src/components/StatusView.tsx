import { useMemo, useState } from "react";
import type { KuniStatus, MyStatus } from "../api/client";
import { bordersOf } from "../adjacency";
import { buildTerritories, countByOwner } from "../territory";
import { MapPanel } from "./MapPanel";

const numberFormat = new Intl.NumberFormat("ja-JP");

/** 自領の資源を合計する（表示用の集計のみ） */
function sumBy(kunis: KuniStatus[], key: "hei" | "kin" | "kome"): number {
  return kunis.reduce((sum, k) => sum + k[key], 0);
}

interface Props {
  status: MyStatus;
  updatedAt: Date | null;
}

/** 自国の状況・ターン数・勢力図を表示する */
export function StatusView({ status, updatedAt }: Props) {
  const [highlightedOwnerId, setHighlightedOwnerId] = useState<number | null>(null);
  const territories = useMemo(() => buildTerritories(status), [status]);
  const ownerCounts = useMemo(() => countByOwner(territories.values()), [territories]);
  // 自領に隣接する他国（攻め込める・攻め込まれうる国）
  const borders = useMemo(
    () => bordersOf(status.my_kunis.map((k) => k.id)).flatMap((id) => territories.get(id) ?? []),
    [status, territories],
  );

  const kpis = [
    { label: "領地", value: `${status.my_kunis.length}`, unit: "国" },
    { label: "兵", value: numberFormat.format(sumBy(status.my_kunis, "hei")), unit: "" },
    { label: "金", value: numberFormat.format(sumBy(status.my_kunis, "kin")), unit: "" },
    { label: "米", value: numberFormat.format(sumBy(status.my_kunis, "kome")), unit: "" },
  ];

  return (
    <main className="status">
      <header className="status-header">
        <div>
          <p className="eyebrow">{status.daimyo.name}家</p>
          <h1 className="hero">
            第<span className="hero-number">{status.turn}</span>ターン
          </h1>
        </div>
        {updatedAt && (
          <p className="muted updated">
            {updatedAt.toLocaleTimeString("ja-JP")} 更新（5秒ごとに自動更新）
          </p>
        )}
      </header>

      <section className="kpis" aria-label="自国の合計">
        {kpis.map((kpi) => (
          <div className="kpi" key={kpi.label}>
            <span className="kpi-label">{kpi.label}</span>
            <span className="kpi-value">
              {kpi.value}
              {kpi.unit && <small>{kpi.unit}</small>}
            </span>
          </div>
        ))}
      </section>

      <div className="layout">
        <section className="card map-card" aria-label="勢力図">
          <div className="card-title">
            <h2>勢力図</h2>
            <ul className="legend">
              <li>
                <span className="swatch mine" aria-hidden="true" />
                自領（{status.daimyo.name}）
              </li>
              <li>
                <span className="swatch other" aria-hidden="true" />
                他家の領地（国名の下に大名名）
              </li>
              <li>
                <span className="swatch-line" aria-hidden="true" />
                隣接（行き来できる国）
              </li>
            </ul>
          </div>
          <MapPanel
            territories={territories}
            ownerCounts={ownerCounts}
            highlightedOwnerId={highlightedOwnerId}
            onHighlightOwner={setHighlightedOwnerId}
          />
        </section>

        <div className="side">
          <section className="card" aria-labelledby="my-kunis-title">
            <h2 id="my-kunis-title">自領の状況</h2>
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
            <h3 className="subheading" id="borders-title">
              隣接する他国
            </h3>
            {borders.length === 0 ? (
              <p className="muted">隣接する他国はありません。</p>
            ) : (
              <ul className="borders" aria-labelledby="borders-title">
                {borders.map((t) => (
                  <li
                    key={t.kuniId}
                    onPointerEnter={() => setHighlightedOwnerId(t.owner.id)}
                    onPointerLeave={() => setHighlightedOwnerId(null)}
                  >
                    {t.kuniName}
                    <span className="muted">（{t.owner.name}）</span>
                  </li>
                ))}
              </ul>
            )}
          </section>

          <section className="card" aria-labelledby="others-title">
            <h2 id="others-title">他国の支配大名</h2>
            {status.other_kunis.length === 0 ? (
              <p className="muted">天下一統を果たしました。</p>
            ) : (
              <table className="others">
                <thead>
                  <tr>
                    <th scope="col">国</th>
                    <th scope="col">大名</th>
                  </tr>
                </thead>
                <tbody>
                  {status.other_kunis.map((k) => (
                    <tr
                      key={k.id}
                      className={k.daimyo.id === highlightedOwnerId ? "highlighted" : undefined}
                      onPointerEnter={() => setHighlightedOwnerId(k.daimyo.id)}
                      onPointerLeave={() => setHighlightedOwnerId(null)}
                    >
                      <th scope="row">{k.name}</th>
                      <td>
                        {k.daimyo.name}
                        {(ownerCounts.get(k.daimyo.id) ?? 0) > 1 && (
                          <span className="muted">（{ownerCounts.get(k.daimyo.id)}国）</span>
                        )}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </section>
        </div>
      </div>
    </main>
  );
}
