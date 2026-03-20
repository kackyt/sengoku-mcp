---
name: git-smart-merge
description: >-
  ブランチを指定して git rebase または git merge を実行するスキル。コミット履歴・コミットメッセージ・
  ブランチ名から変更の意図を読み取り、適切な統合戦略（rebase / merge / squash merge / pull）を選択して
  実行する。コンフリクトが発生した場合はコミット内容から適切な解決策を提示する。
  ユーザーが「〜ブランチに rebase して」「〜に merge して」「ブランチを統合して」「git rebase」
  「git merge」「git pull」「リモートから更新して」などと要求したときにトリガーする。
---

# Git Smart Merge/Rebase スキル

コミット履歴・ブランチ名・コミットメッセージを分析し、適切な Git 統合戦略を選択・実行する。

## ワークフロー

### Step 1: 現在の状態確認

```bash
# 現在のブランチと作業ツリーの状態
git status
git branch --show-current

# ローカルおよびリモートブランチ一覧
git branch -a

# リモートの最新情報を取得（更新があるか確認）
git fetch origin
git status -u
```

### Step 2: コミット履歴の分析

```bash
# 対象ブランチ（またはリモート追跡ブランチ）のコミット履歴
git log <source-branch> --not <target-branch> --oneline --stat
# 例: git log feature-branch --not main --oneline --stat

# ブランチが分岐した共通祖先
git merge-base <target-branch> <source-branch>

# 差分の概要
git diff <target-branch>...<source-branch> --stat
```

コミットメッセージと変更内容から以下を判断する:
- **機能追加**: rebase か通常 merge
- **バグ修正**: cherry-pick または rebase
- **複数作業者**: merge commit を保持（`--no-ff`）
- **WIP / 細かい整理コミット**: squash merge

### Step 3: 統合戦略の選択

判断基準は [./references/merge-strategy.md](./references/merge-strategy.md) を参照。

| 状況 | 推奨戦略 |
|------|----------|
| feature → main、履歴を綺麗に保ちたい | `rebase` してから `merge --no-ff` |
| 複数人が触った長期ブランチ | `merge --no-ff` (履歴保持) |
| WIP コミットをまとめたい | `merge --squash` |
| hotfix を複数ブランチへ適用 | `cherry-pick` |
| release ブランチへの取り込み | `merge --no-ff` |

### Step 4: 実行

**Rebase の場合:**
```bash
git checkout <source-branch>
git rebase <target-branch>
# コンフリクト発生時は Step 5 へ
git checkout <target-branch>
git merge --no-ff <source-branch>
```

**Merge の場合:**
```bash
git checkout <target-branch>
git merge --no-ff <source-branch> -m "merge: <source-branch> を <target-branch> へ統合"
```

**Squash Merge の場合:**
```bash
git checkout <target-branch>
git merge --squash <source-branch>
git commit -m "feat: <変更内容の要約>"
```

**Pull (リモートからの更新) の場合:**
リモートの `main` を現在の `main` に取り込む例：
```bash
# rebase を優先する場合（ローカルに未 push のコミットがある時）
git pull --rebase origin main

# fast-forward のみ許可する場合
git pull --ff-only origin main

# 通常のマージ（マージコミットを作成）
git pull origin main
```

### Step 5: コンフリクト解決

```bash
# コンフリクトファイルの確認
git diff --name-only --diff-filter=U

# rebase 中の場合
git rebase --continue   # 解決後
git rebase --abort      # 中止する場合
```

コンフリクト解決の方針:
1. コミットメッセージと diff から **双方の意図を把握**する
2. 単純な追加同士 → 両方を保持
3. 同じ行の変更 → コードの意図を読みとってマージする。意図が不明確な場合は独断で進めず**必ずユーザーに確認**を求める
4. 解決できない場合はユーザーに確認を求める

### Step 6: 結果確認

```bash
# 統合後の履歴確認
git log --oneline --graph -10

# 差分が意図通りか確認
git diff <target-branch>@{1} <target-branch> --stat
```

## 注意事項

- **push 済みブランチへの rebase は原則禁止**（リモートと履歴が乖離する）
- 作業前に `git status` でクリーンな状態を確認する
- 不明な場合は実行前にユーザーに戦略を確認する
