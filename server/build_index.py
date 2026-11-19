import json
import re
from pathlib import Path

SORTED_DIR = Path("sorted_rules")
OUT_JSON = Path("../overlay/assets/rules_index.json")

RULE_START_RE = re.compile(
    r"^(?:\d+\.\d+(?:\.\d+)?\.?\s+|Стаття\s+\d+|Розділ\s+[IVX\d]+|Глава\s+[IVX\d]+)",
    re.IGNORECASE,
)


def detect_cat(category: str, title: str) -> str:
    c = category.lower()
    t = title.lower()
    if "сбу" in c or "сбу" in t:
        return "sbu"
    if "нпу" in c or "нпу" in t:
        return "npu"
    if "крайм" in t or "кримінальн" in t or "рейд" in t or "взх" in t or "airdrop" in t:
        return "crime"
    if "законодавство" in c:
        return "laws"
    if "правила_проєкту" in c:
        return "rules"
    return "other"


def parse_md(path: Path) -> list[dict]:
    raw = path.read_text(encoding="utf-8")
    category, title = path.parent.name, path.stem
    body = raw

    if raw.startswith("---"):
        parts = raw.split("---", 2)
        if len(parts) == 3:
            header, body = parts[1], parts[2]
            for line in header.splitlines():
                if line.startswith("category:"):
                    category = line.split(":", 1)[1].strip()
                elif line.startswith("title:"):
                    title = line.split(":", 1)[1].strip()

    cat = detect_cat(category, title)
    sections: list[tuple[str, list[str]]] = []
    cur_sec = "Загальні положення"
    cur_lines: list[str] = []

    for line in body.splitlines():
        s = line.strip()
        if not s or (s.startswith("# ") and not s.startswith("## ")):
            continue
        if s.startswith("## "):
            if cur_lines:
                sections.append((cur_sec, cur_lines))
                cur_lines = []
            cur_sec = s[3:].strip()
        else:
            cur_lines.append(s)
    if cur_lines:
        sections.append((cur_sec, cur_lines))

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


def main():
    all_items = []
    files_count = 0
    for md in sorted(SORTED_DIR.rglob("*.md")):
        if "00_Вступ" in str(md) or md.name.startswith("_"):
            continue
        all_items.extend(parse_md(md))
        files_count += 1

    OUT_JSON.parent.mkdir(parents=True, exist_ok=True)
    OUT_JSON.write_text(
        json.dumps(all_items, ensure_ascii=False, separators=(",", ":")),
        encoding="utf-8",
    )
    size_mb = OUT_JSON.stat().st_size / (1024 * 1024)
    print(f"Done: {files_count} files | {len(all_items)} items | {size_mb:.2f} MB")


if __name__ == "__main__":
    main()