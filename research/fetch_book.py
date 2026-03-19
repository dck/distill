"""Download an EPUB book and extract chapters as plain text files.

Usage:
    uv run python fetch_book.py              # download from config URL
    uv run python fetch_book.py book.epub    # use local EPUB file
"""

from __future__ import annotations

import sys
import tempfile
from pathlib import Path

import ebooklib
import httpx
import tomli
from bs4 import BeautifulSoup
from ebooklib import epub

ROOT = Path(__file__).parent
OUTPUT_DIR = ROOT / "data" / "originals"


def load_config() -> dict:
    with open(ROOT / "config.toml", "rb") as f:
        return tomli.load(f)


def chapters_exist(n: int) -> bool:
    """Check if all expected chapter files exist and are non-empty."""
    for i in range(1, n + 1):
        path = OUTPUT_DIR / f"ch{i:02d}.txt"
        if not path.exists() or path.stat().st_size == 0:
            return False
    return True


def download_epub(url: str) -> bytes:
    """Download EPUB file, return raw bytes."""
    print(f"Downloading {url}...")
    r = httpx.get(url, follow_redirects=True, timeout=120)
    if r.status_code != 200:
        print(f"Download failed: HTTP {r.status_code}", file=sys.stderr)
        sys.exit(1)
    print(f"Downloaded {len(r.content):,} bytes")
    return r.content


def html_to_text(html: str) -> str:
    """Convert HTML content to clean plain text."""
    soup = BeautifulSoup(html, "html.parser")
    return soup.get_text(separator="\n", strip=True)


def extract_chapters(epub_bytes: bytes, num_chapters: int, skip_front_matter: bool) -> list[tuple[str, str]]:
    """Extract chapters from EPUB. Returns [(title, text), ...]."""
    # Write to temp file because ebooklib needs a file path
    with tempfile.NamedTemporaryFile(suffix=".epub", delete=False) as f:
        f.write(epub_bytes)
        tmp_path = f.name

    book = epub.read_epub(tmp_path)
    Path(tmp_path).unlink()

    # Get spine items (reading order)
    spine_ids = [item_id for item_id, _ in book.spine]
    items_by_id = {item.get_id(): item for item in book.get_items()}

    chapters = []
    for item_id in spine_ids:
        item = items_by_id.get(item_id)
        if item is None or item.get_type() != ebooklib.ITEM_DOCUMENT:
            continue

        html = item.get_content().decode("utf-8", errors="replace")
        text = html_to_text(html)

        # Skip very short items (cover pages, copyright, TOC)
        if len(text.split()) < 200:
            continue

        # Try to extract title from HTML
        soup = BeautifulSoup(html, "html.parser")
        title_tag = soup.find(["h1", "h2", "h3"])
        title = title_tag.get_text(strip=True) if title_tag else f"Chapter {len(chapters) + 1}"

        chapters.append((title, text))

    if skip_front_matter and chapters:
        # Heuristic: front matter items (preface, introduction) often come before
        # the first item with "chapter" or a numeral in the title.
        # Skip items until we find one that looks like a chapter heading,
        # or just skip the first item if nothing matches.
        skip = 0
        for i, (title, _) in enumerate(chapters):
            lower = title.lower()
            if any(kw in lower for kw in ["chapter", "step", "part 1", "i.", "1."]):
                skip = i
                break
        if skip > 0:
            print(f"Skipping {skip} front matter item(s)")
            chapters = chapters[skip:]

    if len(chapters) < num_chapters:
        print(
            f"WARNING: Found {len(chapters)} content chapters, expected {num_chapters}",
            file=sys.stderr,
        )

    return chapters[:num_chapters]


def save_chapters(chapters: list[tuple[str, str]]) -> None:
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    for i, (title, text) in enumerate(chapters, 1):
        path = OUTPUT_DIR / f"ch{i:02d}.txt"
        path.write_text(text, encoding="utf-8")


def print_info(chapters: list[tuple[str, str]]) -> None:
    total = 0
    print(f"\n{len(chapters)} chapters extracted:")
    print("-" * 55)
    for i, (title, text) in enumerate(chapters, 1):
        words = len(text.split())
        total += words
        print(f"  ch{i:02d}.txt  {title:<35s} {words:>6,} words")
    print("-" * 55)
    print(f"  {'Total':<42s} {total:>6,} words")


def main() -> None:
    config = load_config()
    book = config["book"]
    num_chapters = book["num_chapters"]

    if chapters_exist(num_chapters):
        print("All chapter files already exist -- skipping download.")
        chapters = []
        for i in range(1, num_chapters + 1):
            text = (OUTPUT_DIR / f"ch{i:02d}.txt").read_text(encoding="utf-8")
            chapters.append((f"Chapter {i}", text))
        print_info(chapters)
        return

    # Accept local EPUB path as argument, otherwise download from config URL
    if len(sys.argv) > 1:
        local_path = Path(sys.argv[1])
        if not local_path.exists():
            print(f"File not found: {local_path}", file=sys.stderr)
            sys.exit(1)
        print(f"Reading local EPUB: {local_path}")
        epub_bytes = local_path.read_bytes()
    else:
        epub_bytes = download_epub(book["url"])

    chapters = extract_chapters(
        epub_bytes,
        num_chapters,
        skip_front_matter=book.get("skip_front_matter", True),
    )

    save_chapters(chapters)
    print(f"\nSaved to {OUTPUT_DIR}/")
    print_info(chapters)


if __name__ == "__main__":
    main()
