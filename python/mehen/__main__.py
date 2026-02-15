"""
Command-line interface for mehen.

This module provides access to the mehen CLI when called as `python -m mehen`.
The main interface is the binary `mehen` command.
"""

import subprocess
import sys


def main() -> None:
    """Main entry point that delegates to the mehen binary."""
    try:
        result = subprocess.run(["mehen"] + sys.argv[1:], check=False)
        sys.exit(result.returncode)
    except FileNotFoundError:
        print(
            "mehen binary not found. Please ensure mehen is properly installed.",
            file=sys.stderr,
        )
        sys.exit(1)


if __name__ == "__main__":
    main()
