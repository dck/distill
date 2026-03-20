"""Extract chapters from an EPUB book as plain text files.

Usage:
    uv run python fetch_book.py book.epub              # extract configured chapters
    uv run python fetch_book.py book.epub --list        # list all chapters (for picking)
    uv run python fetch_book.py book.epub --chapters 1,4,7,11,14,20
"""

from __future__ import annotations

import argparse
import sys
import tempfile
from pathlib import Path

import ebooklib
import tomli
from bs4 import BeautifulSoup
from ebooklib import epub

ROOT = Path(__file__).parent
OUTPUT_DIR = ROOT / "data" / "originals"


def load_config() -> dict:
    with open(ROOT / "config.toml", "rb") as f:
        return tomli.load(f)


def extract_all_chapters(epub_path: Path, skip_front_matter: bool) -> list[tuple[int, str, str]]:
    """Extract all content chapters from EPUB. Returns [(1-based index, title, text), ...]."""
    if epub_path.suffix == ".epub":
        book = epub.read_epub(str(epub_path))
    else:
        # Handle temp file case
        book = epub.read_epub(str(epub_path))

    spine_ids = [item_id for item_id, _ in book.spine]
    items_by_id = {item.get_id(): item for item in book.get_items()}

    chapters = []
    for item_id in spine_ids:
        item = items_by_id.get(item_id)
        if item is None or item.get_type() != ebooklib.ITEM_DOCUMENT:
            continue

        html = item.get_content().decode("utf-8", errors="replace")
        soup = BeautifulSoup(html, "html.parser")
        text = soup.get_text(separator="\n", strip=True)

        # Skip very short items (cover pages, copyright, TOC)
        if len(text.split()) < 200:
            continue

        title_tag = soup.find(["h1", "h2", "h3"])
        title = title_tag.get_text(strip=True) if title_tag else f"Chapter {len(chapters) + 1}"

        chapters.append((len(chapters) + 1, title, text))

    if skip_front_matter and chapters:
        skip = 0
        for i, (_, title, _) in enumerate(chapters):
            lower = title.lower()
            if any(kw in lower for kw in ["chapter", "step", "part 1", "part one", "i.", "1.", "1:"]):
                skip = i
                break
        if skip > 0:
            print(f"Skipping {skip} front matter item(s)")
            # Re-index after skipping
            chapters = [
                (j + 1, title, text)
                for j, (_, title, text) in enumerate(chapters[skip:])
            ]

    return chapters


def select_chapters(
    all_chapters: list[tuple[int, str, str]],
    indices: list[int],
) -> list[tuple[int, str, str]]:
    """Select specific chapters by 1-based index."""
    by_index = {idx: (idx, title, text) for idx, title, text in all_chapters}
    selected = []
    for i in indices:
        if i not in by_index:
            print(
                f"WARNING: Chapter {i} not found (book has {len(all_chapters)} chapters)",
                file=sys.stderr,
            )
            continue
        selected.append(by_index[i])
    return selected


def save_chapters(chapters: list[tuple[int, str, str]]) -> None:
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    for i, (_, _, text) in enumerate(chapters, 1):
        path = OUTPUT_DIR / f"ch{i:02d}.txt"
        path.write_text(text, encoding="utf-8")


def print_chapter_list(chapters: list[tuple[int, str, str]]) -> None:
    """Print all chapters with indices for selection."""
    print(f"\n{len(chapters)} content chapters found:")
    print("-" * 65)
    for idx, title, text in chapters:
        words = len(text.split())
        print(f"  {idx:>3d}. {title:<45s} {words:>6,} words")
    print("-" * 65)
    print("\nUse --chapters 1,4,7 to select specific chapters.")


def print_info(chapters: list[tuple[int, str, str]]) -> None:
    total = 0
    print(f"\n{len(chapters)} chapters extracted:")
    print("-" * 65)
    for i, (orig_idx, title, text) in enumerate(chapters, 1):
        words = len(text.split())
        total += words
        print(f"  ch{i:02d}.txt  (book ch {orig_idx:>2d}) {title:<30s} {words:>6,} words")
    print("-" * 65)
    print(f"  {'Total':<52s} {total:>6,} words")


def chapters_on_disk() -> list[tuple[int, str, str]] | None:
    """Load existing chapters from disk if they exist."""
    if not OUTPUT_DIR.exists():
        return None
    files = sorted(OUTPUT_DIR.glob("ch*.txt"))
    if not files:
        return None
    chapters = []
    for i, path in enumerate(files, 1):
        text = path.read_text(encoding="utf-8")
        if not text:
            return None
        chapters.append((i, f"Chapter {i}", text))
    return chapters


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Extract chapters from an EPUB book"
    )
    parser.add_argument("epub", type=Path, help="Path to EPUB file")
    parser.add_argument(
        "--list", action="store_true",
        help="List all chapters with indices, then exit",
    )
    parser.add_argument(
        "--chapters", type=str, default=None,
        help="Comma-separated 1-based chapter indices (overrides config.toml)",
    )
    args = parser.parse_args()

    if not args.epub.exists():
        print(f"File not found: {args.epub}", file=sys.stderr)
        sys.exit(1)

    config = load_config()
    book_config = config["book"]
    skip_front_matter = book_config.get("skip_front_matter", True)

    print(f"Reading EPUB: {args.epub}")
    all_chapters = extract_all_chapters(args.epub, skip_front_matter)

    if not all_chapters:
        print("No content chapters found in EPUB.", file=sys.stderr)
        sys.exit(1)

    if args.list:
        print_chapter_list(all_chapters)
        return

    # Determine which chapters to extract
    if args.chapters:
        indices = [int(x.strip()) for x in args.chapters.split(",")]
    elif "chapters" in book_config and book_config["chapters"]:
        indices = book_config["chapters"]
    else:
        # All chapters
        indices = [idx for idx, _, _ in all_chapters]

    selected = select_chapters(all_chapters, indices)
    if not selected:
        print("No chapters matched the selection.", file=sys.stderr)
        sys.exit(1)

    save_chapters(selected)
    print(f"\nSaved to {OUTPUT_DIR}/")
    print_info(selected)


if __name__ == "__main__":
    main()
