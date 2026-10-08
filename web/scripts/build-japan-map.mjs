// 日本地図（ゲーム内の12国）の SVG パスデータを生成するスクリプト
//
// 入力: Natural Earth の 1:10m Admin-1（都道府県）GeoJSON（パブリックドメイン）
//   https://github.com/nvkelso/natural-earth-vector/blob/master/geojson/ne_10m_admin_1_states_provinces.geojson
// 出力: src/map/japanMap.json（投影済みの SVG パスとラベル位置）
//
// 使い方:
//   pnpm gen:map -- <ne_10m_admin_1_states_provinces.geojson のパス>
//
// 都道府県をゲームの国（static/master_data/kuni.csv）へ割り当てて結合し、簡略化してから投影します。
// 生成物はリポジトリにコミットするため、通常の開発でこのスクリプトを実行する必要はありません。

import mapshaper from "mapshaper";
import { geoMercator, geoPath } from "d3-geo";
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const OUTPUT = resolve(here, "../src/map/japanMap.json");

/** 地図の幅（SVG座標系）。高さは日本列島の縦横比から決める */
const WIDTH = 800;
/** これより南（緯度）の島は描かない（沖縄・奄美・小笠原など。ゲームの12国に含まれないため） */
const MIN_LATITUDE = 30.0;

/**
 * 都道府県コード（JIS X 0401）→ ゲームの国ID の対応表
 * ゲームの国は戦国大名の勢力圏を表す大まかな地域のため、都道府県単位で近似する。
 */
const PREFECTURE_TO_KUNI = {
  1: [1], // 蝦夷: 北海道
  2: [2, 3, 4, 5, 6, 7], // 奥州: 東北6県
  3: [15, 16, 17, 18], // 越州: 新潟・富山・石川・福井
  4: [19, 20], // 甲信: 山梨・長野
  5: [8, 9, 10, 11, 12, 13, 14], // 武蔵: 関東
  6: [22], // 三河: 静岡（遠江・駿河を含む徳川領として近似）
  7: [21, 23], // 尾張: 岐阜・愛知
  8: [24, 25, 26, 27, 28, 29, 30], // 山城: 近畿
  9: [31, 32, 33, 34, 35], // 安芸: 中国地方
  10: [36, 37, 38, 39], // 四国
  11: [40, 41, 42, 43, 44], // 豊後: 九州北部
  12: [45, 46], // 薩摩: 宮崎・鹿児島
};

/** ラベル位置の微調整（SVG座標系での [dx, dy]）。重心が海上や端に寄る国を補正する */
const LABEL_OFFSETS = {
  2: [0, 10],
  5: [8, -6],
  8: [-6, 4],
};

/**
 * 引き出し線つきで海上にラベルを置く国（SVG座標系での絶対位置）
 * 本州中央部は国が小さく密集してラベルが重なるため、周辺の海へ逃がす。
 */
const LEADER_LABELS = {
  3: [330, 482], // 越州: 日本海側へ
  6: [500, 735], // 三河: 太平洋側へ
};

/** 都道府県コード → 国ID の逆引き */
const kuniByPrefecture = new Map(
  Object.entries(PREFECTURE_TO_KUNI).flatMap(([kuniId, prefs]) =>
    prefs.map((pref) => [pref, Number(kuniId)]),
  ),
);

/** ジオメトリを Polygon の配列に分解する */
function toPolygons(geometry) {
  if (geometry.type === "Polygon") return [geometry.coordinates];
  if (geometry.type === "MultiPolygon") return geometry.coordinates;
  return [];
}

/** Natural Earth の都道府県を国IDつきの Polygon 群に変換する（南方離島は除外） */
function toKuniFeatures(source) {
  const features = [];
  for (const feature of source.features) {
    const props = feature.properties ?? {};
    if (props.adm0_a3 !== "JPN") continue;
    const pref = Number(String(props.iso_3166_2 ?? "").replace("JP-", ""));
    const kuniId = kuniByPrefecture.get(pref);
    if (kuniId === undefined) continue;

    for (const polygon of toPolygons(feature.geometry)) {
      // 向きに依存しないよう、外周の頂点の平均緯度で南方離島を判定する
      const outer = polygon[0];
      const meanLat = outer.reduce((sum, [, lat]) => sum + lat, 0) / outer.length;
      if (meanLat < MIN_LATITUDE) continue;
      const part = { type: "Polygon", coordinates: polygon };
      features.push({ type: "Feature", properties: { kuni_id: kuniId }, geometry: part });
    }
  }
  return { type: "FeatureCollection", features };
}

/** mapshaper で国ごとに結合・小島除去・簡略化する */
async function dissolveAndSimplify(collection) {
  const output = await mapshaper.applyCommands(
    [
      "-i input.json",
      "-dissolve kuni_id",
      // 面積の小さい島は描画しない（地図の見やすさ優先）
      "-filter-islands min-area=150km2 remove-empty",
      "-simplify 12% keep-shapes",
      "-o output.json format=geojson precision=0.0001",
    ].join(" "),
    { "input.json": collection },
  );
  return JSON.parse(output["output.json"]);
}

/** 経緯度平面での符号付き面積（正なら反時計回り） */
function signedArea(ring) {
  let sum = 0;
  for (let i = 0, j = ring.length - 1; i < ring.length; j = i++) {
    sum += (ring[j][0] - ring[i][0]) * (ring[j][1] + ring[i][1]);
  }
  return sum / 2;
}

/**
 * d3-geo の球面ジオメトリの向き（外周は時計回り・穴は反時計回り）に揃える
 * GeoJSON（RFC 7946）は逆向きのため、そのままだと「地球全体から国を除いた領域」と解釈される。
 */
function rewindForD3(collection) {
  const rewindPolygon = (polygon) =>
    polygon.map((ring, index) => {
      const clockwise = signedArea(ring) < 0;
      const wantClockwise = index === 0;
      return clockwise === wantClockwise ? ring : [...ring].reverse();
    });
  for (const feature of collection.features) {
    const g = feature.geometry;
    if (g.type === "Polygon") g.coordinates = rewindPolygon(g.coordinates);
    if (g.type === "MultiPolygon") g.coordinates = g.coordinates.map(rewindPolygon);
  }
  return collection;
}

/** 投影して SVG パスとラベル位置を生成する */
function project(collection) {
  // メルカトル図法で全体を幅 WIDTH に収め、高さは縦横比から決める
  const projection = geoMercator().fitWidth(WIDTH, collection);
  const path = geoPath(projection);
  const [[, y0], [, y1]] = path.bounds(collection);
  const height = Math.ceil(y1 - y0);
  projection.translate([projection.translate()[0], projection.translate()[1] - y0]);

  const kunis = collection.features
    .map((feature) => {
      const kuniId = Number(feature.properties.kuni_id);
      // ラベルは最も大きな陸地の重心に置く（離島に引っ張られないように）
      const largest = toPolygons(feature.geometry)
        .map((coordinates) => ({ type: "Polygon", coordinates }))
        .sort((a, b) => path.area(b) - path.area(a))[0];
      const [cx, cy] = path.centroid(largest);
      const [dx, dy] = LABEL_OFFSETS[kuniId] ?? [0, 0];
      const anchor = [Math.round(cx + dx), Math.round(cy + dy)];
      const leader = LEADER_LABELS[kuniId];
      return {
        kuniId,
        d: path.digits(1)(feature),
        // 引き出し線を使う国は、ラベルを海上に置き、国内の anchor と線で結ぶ
        ...(leader ? { label: leader, anchor } : { label: anchor }),
      };
    })
    .sort((a, b) => a.kuniId - b.kuniId);

  return { width: WIDTH, height, kunis };
}

async function main() {
  const inputPath = process.argv.slice(2).find((arg) => arg !== "--");
  if (!inputPath) {
    console.error("使い方: pnpm gen:map -- <ne_10m_admin_1_states_provinces.geojson>");
    process.exit(1);
  }
  const source = JSON.parse(readFileSync(inputPath, "utf8"));
  const simplified = await dissolveAndSimplify(toKuniFeatures(source));
  const map = project(rewindForD3(simplified));

  const ids = map.kunis.map((k) => k.kuniId);
  if (ids.length !== 12) {
    throw new Error(`12国分のパスが生成されませんでした: ${ids.join(",")}`);
  }
  writeFileSync(
    OUTPUT,
    JSON.stringify(
      {
        source: "Natural Earth 1:10m Admin-1 (public domain), aggregated into game provinces",
        ...map,
      },
      null,
      2,
    ) + "\n",
  );
  console.log(`生成しました: ${OUTPUT} (${map.width}x${map.height})`);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
