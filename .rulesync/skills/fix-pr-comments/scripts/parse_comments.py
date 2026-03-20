import json
import sys
import os

def parse_pr_comments(json_paths):
    threads = {}
    general_comments = []

    for json_path in json_paths:
        if not os.path.exists(json_path):
            print(f"Warning: {json_path} not found. Skipping.")
            continue

        with open(json_path, 'r', encoding='utf-8') as f:
            try:
                data = json.load(f)
            except json.JSONDecodeError as e:
                print(f"Error decoding JSON in {json_path}: {e}")
                continue

        # Handle different JSON structures
        if isinstance(data, list):
            # Format: gh api repos/:owner/:repo/pulls/:number/comments
            for rc in data:
                path = rc.get('path', 'Unknown file')
                # API lists 'line' or 'original_line'
                line = rc.get('line') or rc.get('original_line') or 'N/A'
                diff_hunk = rc.get('diff_hunk', '')
                
                key = f"{path}:{line}"
                if key not in threads:
                    threads[key] = {
                        'path': path,
                        'line': line,
                        'diff': diff_hunk,
                        'comments': []
                    }
                
                threads[key]['comments'].append({
                    'author': rc.get('user', {}).get('login', 'unknown'),
                    'body': rc.get('body', '').strip(),
                    'createdAt': rc.get('created_at', ''),
                    'id': rc.get('id', '')
                })
        elif isinstance(data, dict):
            # Format: gh pr view --json comments,reviews
            # General comments
            general_comments.extend(data.get('comments', []))
            
            # Inline review comments
            reviews = data.get('reviews', [])
            for review in reviews:
                review_comments = review.get('comments', [])
                for rc in review_comments:
                    path = rc.get('path', 'Unknown file')
                    line = rc.get('line') or rc.get('originalLine', 'N/A')
                    diff_hunk = rc.get('diffHunk', '')
                    
                    key = f"{path}:{line}"
                    if key not in threads:
                        threads[key] = {
                            'path': path,
                            'line': line,
                            'diff': diff_hunk,
                            'comments': []
                        }
                    
                    threads[key]['comments'].append({
                        'author': rc.get('author', {}).get('login', 'unknown'),
                        'body': rc.get('body', '').strip(),
                        'createdAt': rc.get('createdAt', ''),
                        'id': rc.get('id', '')
                    })

    output_lines = ["# PR Review Comments Suggestions\n"]

    if general_comments:
        output_lines.append("## General PR Comments\n")
        # Deduplicate and sort could be added here if needed
        for c in general_comments:
            author = c.get('author', {}).get('login', 'unknown')
            body = c.get('body', '').strip()
            if body:
                output_lines.append(f"### From @{author}\n{body}\n\n---\n")

    if threads:
        output_lines.append("## Inline Suggestions by File\n")
        
        def sort_key(k):
            # Split from the right in case path contains colons (rare but possible)
            if ':' in k:
                path, line_str = k.rsplit(':', 1)
                try:
                    return (path, int(line_str))
                except ValueError:
                    return (path, 0)
            return (k, 0)
            
        sorted_keys = sorted(threads.keys(), key=sort_key)
        for key in sorted_keys:
            thread = threads[key]
            output_lines.append(f"### File: `{thread['path']}` (Line: {thread['line']})\n")
            if thread['diff']:
                output_lines.append("```diff\n" + thread['diff'] + "\n```\n")
            
            # Sort comments in thread by date, then ID as fallback
            thread_comments = sorted(
                thread['comments'], 
                key=lambda x: (x['createdAt'], str(x['id']))
            )
            # Deduplicate by ID
            seen_ids = set()
            for tc in thread_comments:
                if tc['id'] and tc['id'] in seen_ids:
                    continue
                seen_ids.add(tc['id'])
                output_lines.append(f"**@{tc['author']}**: {tc['body']}\n\n")
            output_lines.append("---\n")

    output_path = "suggestions.md"
    with open(output_path, 'w', encoding='utf-8') as f:
        f.writelines(output_lines)
    
    print(f"Successfully generated {output_path}")

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python parse_comments.py <file1.json> [file2.json] ...")
        sys.exit(1)
    else:
        parse_pr_comments(sys.argv[1:])

