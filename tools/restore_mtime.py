#!/usr/bin/env python3
"""Set every tracked file's mtime to the time of the last commit that touched it.

A checkout gives files the time of the checkout, so CI's comparison cache (keyed on
"output newer than input") would see every fixture as changed on every run. This is what
`git restore-mtime` does, but that tool is built on `git whatchanged`, which Git 2.5x refuses
to run, and it then silently leaves every file untouched ("0 commits evaluated").

Usage: python3 tools/restore_mtime.py [repo_dir]     (needs the full history: fetch-depth 0)
"""
import os
import subprocess
import sys


def main() -> None:
    repo = sys.argv[1] if len(sys.argv) > 1 else "."
    git = lambda *a: subprocess.run(["git", "-C", repo, *a], check=True, capture_output=True, text=True).stdout  # noqa: E731
    pending = set(git("ls-files", "-z").split("\0")) - {""}
    total = len(pending)
    # Newest first, one @@<author time> line per commit then the files it touched; a merge commit
    # lists none, so a file is dated by the branch commit that changed it (as git-restore-mtime).
    log = subprocess.Popen(["git", "-C", repo, "log", "--pretty=format:@@%at", "--name-only", "--no-renames"],
                           stdout=subprocess.PIPE, text=True)
    stamp = 0
    for line in log.stdout:
        line = line.rstrip("\n")
        if line.startswith("@@"):
            stamp = int(line[2:])
        elif line in pending:
            pending.remove(line)
            os.utime(os.path.join(repo, line), (stamp, stamp), follow_symlinks=False)
            if not pending:
                break
    log.stdout.close()
    log.terminate()
    print(f"{repo}: {total - len(pending)} of {total} files dated" + (f", {len(pending)} not in the log" if pending else ""))


if __name__ == "__main__":
    main()
