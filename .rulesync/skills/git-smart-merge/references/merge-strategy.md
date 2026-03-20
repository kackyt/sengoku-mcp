# Git 統合戦略 判断ガイド

## 目次
1. [rebase vs merge の選択基準](#rebase-vs-merge)
2. [squash merge の使いどき](#squash-merge)
3. [cherry-pick の使いどき](#cherry-pick)
4. [コミットメッセージからの意図読み取り](#intent-from-commits)
5. [GitHub Flow / Git Flow との対応](#branch-models)

---

<a id="rebase-vs-merge"></a>
## rebase vs merge の選択基準

### rebase を選ぶ状況
- **個人作業ブランチ**（まだリモートに push していない）
- `feature/xxx` → `main` で**履歴を一直線に保ちたい**場合
- ブランチがベースから大幅に遅れていて、競合を先に解消したい場合
- コミット数が少なく（5件以内程度）、WIP コミットがない場合

### merge を選ぶ状況
- **複数人が作業**したブランチ（他の人の push が含まれる）
- **リモートに push 済み**のブランチ（rebase すると履歴が書き換わる）
- `release` や `hotfix` ブランチなど、**マージ記録**を明示的に残したい場合
- コミット数が多く、個々のコミットに意味がある場合

### 判断フロー
```text
push 済み? → YES → merge --no-ff
     ↓ NO
複数人が push? → YES → merge --no-ff
     ↓ NO
WIP コミットが多い? → YES → squash merge
     ↓ NO
rebase してから merge --no-ff
```

---

<a id="squash-merge"></a>
## squash merge の使いどき

多数の細かいコミット（"fix typo", "wip", "試し"など）を1つにまとめる場合に使用する。

**squash が適切なコミット例:**
```text
abc1234 fix typo
bcd2345 WIP: working on feature
cde3456 add test
def4567 actually add test
efg5678 fix test
```
→ 1コミット `feat: ○○機能を追加` にまとめる

**squash が不適切なケース:**
- 各コミットに意味があり、後で git blame で追跡したい場合
- 複数の独立した機能変更が混在している場合

---

<a id="cherry-pick"></a>
## cherry-pick の使いどき

特定のコミットだけを別ブランチに適用したい場合:

```bash
# 例: hotfix を main と develop 両方に適用
git checkout main
git cherry-pick <hotfix-commit-hash>

git checkout develop
git cherry-pick <hotfix-commit-hash>
```

**適切なケース:**
- バグ修正を複数のブランチへ適用
- 特定の機能追加だけを取り込みたい
- 間違えて別のブランチにコミットしてしまった

---

<a id="intent-from-commits"></a>
## コミットメッセージからの意図読み取り

### Conventional Commits 規約の場合

| プレフィックス | 意味 | 推奨戦略 |
|---------------|------|----------|
| `feat:` | 新機能 | rebase + merge --no-ff |
| `fix:` | バグ修正 | cherry-pick または rebase |
| `refactor:` | リファクタリング | rebase または squash |
| `chore:` | 雑務・設定変更 | squash merge |
| `wip:` / `WIP` | 作業中 | squash merge |
| `docs:` | ドキュメント | merge --no-ff |

### コミット数による判断
- **1〜3件、明確なメッセージ**: rebase + merge --no-ff
- **4〜10件、関連性が高い**: rebase してから merge
- **10件以上 or WIP 混在**: squash merge を検討
- **履歴が複雑に絡み合っている**: merge --no-ff で履歴保持

### ブランチ名からの推測

| ブランチ名パターン | 推奨戦略 |
|-------------------|----------|
| `feature/*` | rebase + merge --no-ff |
| `fix/*`, `hotfix/*` | cherry-pick または rebase |
| `release/*` | merge --no-ff（履歴保持） |
| `develop`, `staging` | merge --no-ff |
| `wip/*`, `draft/*` | squash merge |

---

<a id="branch-models"></a>
## GitHub Flow / Git Flow との対応

### GitHub Flow（シンプル: main + feature branches）
```text
feature/* → main: rebase してから merge --no-ff
hotfix/* → main: cherry-pick または rebase
```

### Git Flow（develop + release + feature）
```text
feature/* → develop: merge --no-ff
develop → release: merge --no-ff
release → main: merge --no-ff（タグ付き）
hotfix/* → main AND develop: cherry-pick
```

### このプロジェクト（GitHub Flow ベース）
```text
feature/* → main: rebase してから merge --no-ff が推奨
bugfix/* → main: rebase または cherry-pick
```

---

<a id="remote-pull"></a>
## リモートからの更新 (Pull/Update)

リモートリポジトリの変更をローカルに取り込む際の基準。

### git pull --rebase を選ぶ状況
- **ローカルに未 push のコミットがある**が、履歴を一直線に保ちたい
- 開発中の feature ブランチを最新の `main` に追従させる
- 小規模な修正でマージコミットを増やしたくない

### git pull (--merge) を選ぶ状況
- **共有ブランチ**（`main` など）で、誰がいつ更新を取り込んだか明確にしたい
- 複雑な競合が予想され、マージの記録を残しておきたい

### git pull --ff-only を選ぶ状況
- ローカルで一切変更を行っておらず、単にリモートの状態に合わせたい
- 予期せぬマージやリベースが発生するのを防ぎたい（安全策）

