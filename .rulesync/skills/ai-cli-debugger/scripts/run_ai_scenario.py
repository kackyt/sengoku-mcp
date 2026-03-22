import sys
import subprocess
import argparse

def main():
    parser = argparse.ArgumentParser(description="Run TUI AI Debug scenario")
    parser.add_argument("bin_cmd", help="Command to run the binary, e.g. 'cargo run -p openwars_cli --features ai-debug'")
    parser.add_argument("--keys", required=True, help="Space-separated list of keys, e.g. '5*right 3*down enter dump q'")
    args = parser.parse_args()

    # Expand keys
    expanded_keys = []
    for token in args.keys.split():
        if '*' in token:
            parts = token.split('*')
            if len(parts) == 2 and parts[0].isdigit():
                expanded_keys.extend([parts[1]] * int(parts[0]))
            else:
                expanded_keys.append(token)
        else:
            expanded_keys.append(token)
    
    input_str = "\n".join(expanded_keys) + "\n"
    
    # Run process
    process = subprocess.Popen(
        args.bin_cmd,
        shell=True,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE, # Keep stderr separate to avoid noise in stdout dump
        text=True,
        encoding='utf-8'
    )
    
    stdout, stderr = process.communicate(input=input_str)
    
    print(stdout)
    if process.returncode != 0:
        print(f"--- Process exited with code {process.returncode} ---")
        if stderr:
             print("STDERR:")
             print(stderr)

if __name__ == "__main__":
    main()
