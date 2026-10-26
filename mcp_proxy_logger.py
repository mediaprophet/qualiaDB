import sys
import subprocess
import os

LOG_PATH = r"C:\github\qualiaDB\.qualia\mcp_debug.log"

def log(msg):
    with open(LOG_PATH, "a", encoding="utf-8") as f:
        f.write(msg + "\n")
        f.flush()

def main():
    log("\n=== MCP PROXY STARTED ===")
    log(f"Args: {sys.argv}")
    cmd = [r"C:\Users\tim_h\.cargo\bin\qualia-cli.exe", "--enable-qpu", "mcp", "serve", "--transport", "stdio"]
    proc = subprocess.Popen(
        cmd,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        bufsize=0
    )

    import threading

    def read_stderr():
        while True:
            line = proc.stderr.readline()
            if not line:
                break
            log(f"[STDERR] {line.decode('utf-8', errors='replace').rstrip()}")

    def read_stdout():
        while True:
            line = proc.stdout.readline()
            if not line:
                break
            log(f"[FROM SERVER] {line.decode('utf-8', errors='replace').rstrip()}")
            sys.stdout.buffer.write(line)
            sys.stdout.buffer.flush()

    t_err = threading.Thread(target=read_stderr, daemon=True)
    t_out = threading.Thread(target=read_stdout, daemon=True)
    t_err.start()
    t_out.start()

    try:
        while True:
            line = sys.stdin.buffer.readline()
            if not line:
                log("[STDIN EOF]")
                break
            log(f"[TO SERVER] {line.decode('utf-8', errors='replace').rstrip()}")
            proc.stdin.write(line)
            proc.stdin.flush()
    except Exception as e:
        log(f"[EXCEPTION] {e}")
    finally:
        log("[MCP PROXY CLOSING]")
        proc.terminate()

if __name__ == "__main__":
    main()
