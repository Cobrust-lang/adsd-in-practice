"""pyfmt-mini CLI entry."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from .rules import format_source


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        prog="pyfmt-mini",
        description="ADSD CS-04 minimal Python code formatter",
    )
    parser.add_argument("files", nargs="+", help="Python files (or '-' for stdin)")
    parser.add_argument("--check", action="store_true", help="Check only; exit 1 if would change")
    parser.add_argument("--diff", action="store_true", help="Print unified diff instead of writing")
    parser.add_argument(
        "-i", "--in-place", action="store_true", help="Modify files in place (default: stdout)"
    )

    args = parser.parse_args(argv)

    exit_code = 0
    for f in args.files:
        if f == "-":
            src = sys.stdin.read()
            out = format_source(src)
            if args.check:
                if src != out:
                    exit_code = 1
            else:
                sys.stdout.write(out)
        else:
            p = Path(f)
            src = p.read_text(encoding="utf-8")
            out = format_source(src)
            if args.check:
                if src != out:
                    print(f"would reformat: {f}", file=sys.stderr)
                    exit_code = 1
            elif args.diff:
                import difflib

                diff = difflib.unified_diff(
                    src.splitlines(keepends=True),
                    out.splitlines(keepends=True),
                    fromfile=f"{f} (before)",
                    tofile=f"{f} (after)",
                )
                sys.stdout.writelines(diff)
            elif args.in_place:
                if src != out:
                    p.write_text(out, encoding="utf-8")
                    print(f"reformatted: {f}")
            else:
                sys.stdout.write(out)

    return exit_code


if __name__ == "__main__":
    sys.exit(main())
