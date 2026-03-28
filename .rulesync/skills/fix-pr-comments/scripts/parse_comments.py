import subprocess
import json
import sys
import os
from datetime import datetime


def run_cmd(cmd):
    result = subprocess.run(cmd, capture_output=True, text=True, encoding='utf-8')
    if result.returncode != 0:
        print(f"Command failed: {' '.join(cmd)}\n{result.stderr}")
        sys.exit(1)
    return result.stdout


def get_pr_info(pr_arg=None):
    if pr_arg:
        cmd = ['gh', 'pr', 'view', pr_arg, '--json', 'url,number']
    else:
        cmd = ['gh', 'pr', 'view', '--json', 'url,number']

    out = run_cmd(cmd)
    data = json.loads(out)

    url = data['url']
    number = data['number']

    parts = url.split('/')
    owner = parts[-4]
    repo = parts[-3]

    return owner, repo, number


def fetch_threads(owner, repo, number):
    """
    GitHub GraphQL で PR のレビュースレッドを取得する。
    isResolved: ユーザーが明示的に「解決済み」にマークしたスレッド
    isOutdated: コードが変更されてスレッドの指摘箇所が古くなったスレッド
    isCollapsed: コードが更新によってスレッドが折りたたまれた状態
    これらのいずれかが true のスレッドは「解決済み/不要」とみなしてスキップする。
    """
    query = """
    query($name: String!, $owner: String!, $number: Int!, $cursor: String) {
      repository(owner: $owner, name: $name) {
        pullRequest(number: $number) {
          reviewThreads(first: 100, after: $cursor) {
            pageInfo {
              hasNextPage
              endCursor
            }
            nodes {
              isResolved
              isOutdated
              isCollapsed
              comments(first: 50) {
                nodes {
                  id
                  body
                  path
                  line
                  originalLine
                  author { login }
                  createdAt
                  diffHunk
                }
              }
            }
          }
        }
      }
    }
    """

    all_threads = []
    cursor = None

    while True:
        cmd = [
            'gh', 'api', 'graphql',
            '-F', f'owner={owner}',
            '-F', f'name={repo}',
            '-F', f'number={number}',
            '-f', f'query={query}'
        ]
        if cursor:
            cmd.extend(['-F', f'cursor={cursor}'])

        out = run_cmd(cmd)
        data = json.loads(out)

        pr_data = data.get('data', {}).get('repository', {}).get('pullRequest', {})
        threads_page = pr_data.get('reviewThreads', {})

        all_threads.extend(threads_page.get('nodes', []))

        page_info = threads_page.get('pageInfo', {})
        if page_info.get('hasNextPage'):
            cursor = page_info.get('endCursor')
        else:
            break

    return all_threads


def is_unresolved(thread):
    """
    スレッドが未解決かどうかを判定する。
    以下のいずれかに該当する場合は解決済み/不要とみなす:
    - isResolved: True  (ユーザーが明示的に解決済みにした)
    - isOutdated: True  (コード変更によって指摘箇所が古くなった)
    - isCollapsed: True (コード更新によって折りたたまれた)
    """
    if thread.get('isResolved'):
        return False
    if thread.get('isOutdated'):
        return False
    if thread.get('isCollapsed'):
        return False
    return True


def generate_markdown(threads):
    output_lines = ["# PR Review Comments - Unresolved Threads\n"]
    output_lines.append(f"Generated at: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n\n")

    active_threads = []

    for thread in threads:
        if not is_unresolved(thread):
            continue

        comments = thread.get('comments', {}).get('nodes', [])
        if not comments:
            continue

        first_comment = comments[0]
        path = first_comment.get('path', 'Unknown file')
        line = first_comment.get('line') or first_comment.get('originalLine') or 'N/A'
        diff_hunk = first_comment.get('diffHunk', '')

        active_threads.append({
            'path': path,
            'line': line,
            'comments': comments,
        })

    if not active_threads:
        output_lines.append("✅ 未解決のレビュースレッドはありません。\n")
    else:
        output_lines.append(f"**{len(active_threads)} 件の未解決スレッド**\n\n")

        def sort_key(t):
            try:
                line_num = int(t['line']) if t['line'] != 'N/A' else 0
                return (t['path'], line_num)
            except (ValueError, TypeError):
                return (t['path'], 0)

        active_threads.sort(key=sort_key)

        for thread in active_threads:
            output_lines.append(f"### `{thread['path']}` (Line: {thread['line']})\n")

            for comment in thread['comments']:
                author = comment.get('author', {}).get('login', 'unknown') if comment.get('author') else 'unknown'
                body = comment.get('body', '').strip()
                created = comment.get('createdAt', '')
                output_lines.append(f"**@{author}** ({created[:10]}):\n\n{body}\n\n")
            output_lines.append("---\n\n")

    output_path = "suggestions.md"
    with open(output_path, 'w', encoding='utf-8') as f:
        f.writelines(output_lines)

    print(f"Successfully generated {output_path} ({len(active_threads)} unresolved threads)")


if __name__ == "__main__":
    pr_arg = sys.argv[1] if len(sys.argv) > 1 else None

    print("Fetching PR info...")
    owner, repo, number = get_pr_info(pr_arg)

    print(f"Fetching review threads for {owner}/{repo}#{number}...")
    threads = fetch_threads(owner, repo, number)

    total = len(threads)
    resolved = sum(1 for t in threads if not is_unresolved(t))
    print(f"Total threads: {total}, Resolved/Outdated/Collapsed: {resolved}, Active: {total - resolved}")

    print("Generating suggestions.md...")
    generate_markdown(threads)
