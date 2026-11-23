import concurrent.futures
import hashlib
import json
import os
import re
import sys
import time
import urllib.request
from datetime import datetime, timezone
from html.parser import HTMLParser
from pathlib import Path

API_BASE = "https://api.ng-gta5.com/api/kb"
ROOT = Path(__file__).resolve().parent.parent
ASSETS_DIR = ROOT / "overlay" / "assets"
OUT_INDEX = ASSETS_DIR / "rules_index.json"
OUT_META = ASSETS_DIR / "rules_meta.json"

RULE_START_RE = re.compile(
    r"^(?:\d+\.\d+(?:\.\d+)?\.?\s+|Стаття\s+\d+|Розділ\s+[IVX\d]+|Глава\s+[IVX\d]+)",
    re.IGNORECASE,
)

FACTIONS = {"npu", "sbu", "ngu", "dbr", "dvk", "emd", "tsn", "dvv"}


def fetch_json(url: str, retries: int = 3) -> dict:
    req = urllib.request.Request(
        url,
        headers={"User-Agent": "NG-Helper-RulesSync/1.1", "Accept": "application/json"},
    )
    last_err = None
    for attempt in range(retries):
        try:
            with urllib.request.urlopen(req, timeout=20) as resp:
                return json.loads(resp.read().decode("utf-8"))
        except Exception as e:
            last_err = e
            if attempt + 1 < retries:
                time.sleep(1.5 * (attempt + 1))
    raise RuntimeError(f"Failed to fetch {url}: {last_err}")


def detect_cat(doc: dict) -> str:
    faction = (doc.get("faction") or "").lower()
    title = (doc.get("title") or "").lower()
    section = (doc.get("section") or "").lower()
    group = (doc.get("group") or "").lower()

    if faction in FACTIONS:
        return faction
    for f_code, kw in (
        ("sbu", "сбу"),
        ("npu", "нпу"),
        ("ngu", "нгу"),
        ("dbr", "дбр"),
        ("dvk", "двк"),
        ("emd", "емд"),
        ("tsn", "тсн"),
        ("dvv", "двв"),
    ):
        if kw in title and group == "factions":
            return f_code
    if group == "codes":
        return "codes"
    if group in ("constitution", "laws"):
        return "laws"
    if any(k in title for k in ("крайм", "кримінальн", "рейд", "взх", "airdrop", "поставок", "нападу на фз")):
        return "crime"
    if any(k in title for k in ("державних організацій", "допитів", "військового стану", "воєнного стану", "захоплення державних")):
        return "state"
    if section == "rules" or group == "rules":
        return "rules"
    return "other"


def clean_text(text: str) -> str:
    return re.sub(r"\s+", " ", text.replace("\xa0", " ")).strip()


class DocHTMLParser(HTMLParser):
    BLOCK_TAGS = {"p", "div", "h3", "h4", "h5", "blockquote"}

    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.sections: list[tuple[str, list[str]]] = []
        self.cur_sec = "Загальні положення"
        self.cur_lines: list[str] = []
        self.line_buf: list[str] = []
        self.h2_buf: list[str] = []
        self.cell_buf: list[str] = []
        self.row_cells: list[str] = []
        self.skip_depth = 0
        self.in_h1 = False
        self.in_h2 = False
        self.in_li = False
        self.in_cell = False

    def flush_line(self, is_li: bool = False):
        text = clean_text("".join(self.line_buf))
        self.line_buf.clear()
        if not text or text in ("|", "-", "—"):
            return
        if is_li and not text.startswith(("•", "-", "—", "*")):
            text = f"• {text}"
        self.cur_lines.append(text)

    def handle_starttag(self, tag: str, attrs):
        t = tag.lower()
        if t in ("script", "style"):
            self.skip_depth += 1
            return
        if self.skip_depth:
            return
        if t == "h1":
            self.flush_line()
            self.in_h1 = True
        elif t == "h2":
            self.flush_line()
            self.in_h2 = True
            self.h2_buf.clear()
        elif t in ("td", "th"):
            self.in_cell = True
            self.cell_buf.clear()
        elif t == "tr":
            self.flush_line()
            self.row_cells.clear()
        elif t == "li":
            self.flush_line()
            self.in_li = True
        elif t == "br":
            if self.in_cell:
                self.cell_buf.append(" ")
            elif self.in_h2:
                self.h2_buf.append(" ")
            else:
                self.flush_line(self.in_li)
        elif t in self.BLOCK_TAGS:
            if self.in_cell:
                self.cell_buf.append(" ")
            else:
                self.flush_line(self.in_li)

    def handle_endtag(self, tag: str):
        t = tag.lower()
        if t in ("script", "style"):
            self.skip_depth = max(0, self.skip_depth - 1)
            return
        if self.skip_depth:
            return
        if t == "h1":
            self.in_h1 = False
        elif t == "h2":
            self.in_h2 = False
            heading = clean_text("".join(self.h2_buf))
            self.h2_buf.clear()
            if heading:
                if self.cur_lines:
                    self.sections.append((self.cur_sec, self.cur_lines))
                    self.cur_lines = []
                self.cur_sec = heading
        elif t in ("td", "th"):
            self.in_cell = False
            cell = clean_text("".join(self.cell_buf))
            self.cell_buf.clear()
            if cell:
                self.row_cells.append(cell)
        elif t == "tr":
            if self.row_cells:
                self.cur_lines.append(" | ".join(self.row_cells))
                self.row_cells.clear()
        elif t == "li":
            self.flush_line(is_li=True)
            self.in_li = False
        elif t in self.BLOCK_TAGS and not self.in_cell:
            self.flush_line(self.in_li)

    def handle_data(self, data: str):
        if self.skip_depth or self.in_h1:
            return
        if self.in_h2:
            self.h2_buf.append(data)
        elif self.in_cell:
            self.cell_buf.append(data)
        else:
            self.line_buf.append(data)

    def finish(self) -> list[tuple[str, list[str]]]:
        self.flush_line(self.in_li)
        if self.cur_lines:
            self.sections.append((self.cur_sec, self.cur_lines))
            self.cur_lines = []
        return self.sections


def parse_doc_items(meta: dict, doc_data: dict) -> list[dict]:
    title = clean_text(doc_data.get("title") or meta.get("title") or meta["slug"])
    cat = detect_cat(meta)
    html_body = doc_data.get("html") or ""

    parser = DocHTMLParser()
    parser.feed(html_body)
    parser.close()
    sections = parser.finish()

    items = []
    for sec_name, lines in sections:
        blocks: list[str] = []
        buf: list[str] = []
        for line in lines:
            if RULE_START_RE.match(line) and buf:
                blocks.append("\n".join(buf))
                buf = [line]
            else:
                buf.append(line)
        if buf:
            blocks.append("\n".join(buf))

        chunk_buf: list[str] = []
        chunk_len = 0
        for blk in blocks:
            if len(blk) > 1400:
                if chunk_buf:
                    items.append({"c": cat, "d": title, "s": sec_name, "t": "\n".join(chunk_buf)})
                    chunk_buf, chunk_len = [], 0
                for i in range(0, len(blk), 1000):
                    items.append({"c": cat, "d": title, "s": sec_name, "t": blk[i : i + 1200]})
                continue

            if chunk_len + len(blk) > 750 and chunk_buf:
                items.append({"c": cat, "d": title, "s": sec_name, "t": "\n".join(chunk_buf)})
                chunk_buf = [blk]
                chunk_len = len(blk)
            else:
                chunk_buf.append(blk)
                chunk_len += len(blk) + 1

        if chunk_buf:
            items.append({"c": cat, "d": title, "s": sec_name, "t": "\n".join(chunk_buf)})

    return items


def fetch_doc(meta: dict) -> tuple[dict, dict]:
    return meta, fetch_json(f"{API_BASE}/docs/{meta['slug']}")


def main():
    sync_mode = os.environ.get("SYNC_MODE", "manual").strip().lower()
    if sync_mode not in ("auto", "manual"):
        sync_mode = "manual"

    index = fetch_json(f"{API_BASE}/index")
    docs = index.get("docs") or []
    if len(docs) < 50:
        sys.exit(f"Aborting: unexpected docs count ({len(docs)})")

    with concurrent.futures.ThreadPoolExecutor(max_workers=8) as pool:
        fetched = list(pool.map(fetch_doc, docs))

    all_items: list[dict] = []
    for meta, doc_data in fetched:
        all_items.extend(parse_doc_items(meta, doc_data))

    if len(all_items) < 500:
        sys.exit(f"Aborting: unexpected items count ({len(all_items)})")

    payload = json.dumps(all_items, ensure_ascii=False, separators=(",", ":"))
    digest = hashlib.sha256(payload.encode("utf-8")).hexdigest()

    ASSETS_DIR.mkdir(parents=True, exist_ok=True)
    old_hash = None
    if OUT_META.exists():
        try:
            old_hash = json.loads(OUT_META.read_text(encoding="utf-8")).get("sha256")
        except Exception:
            old_hash = None

    if old_hash == digest and OUT_INDEX.exists():
        print(f"Up to date ({sync_mode}): {len(docs)} docs | {len(all_items)} items | sha256={digest[:12]}")
        return

    OUT_INDEX.write_text(payload, encoding="utf-8")
    meta_out = {
        "sha256": digest,
        "docs": len(docs),
        "items": len(all_items),
        "updated_at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "updated_by": sync_mode,
    }
    OUT_META.write_text(json.dumps(meta_out, indent=2) + "\n", encoding="utf-8")
    size_mb = OUT_INDEX.stat().st_size / (1024 * 1024)
    print(f"Updated ({sync_mode}): {len(docs)} docs | {len(all_items)} items | {size_mb:.2f} MB | sha256={digest[:12]}")


if __name__ == "__main__":
    main()