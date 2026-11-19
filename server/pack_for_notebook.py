from pathlib import Path

SORTED_DIR = Path("sorted_rules")
OUT_DIR = Path("notebook_pack")

# Распределение 17 файлов из "01_Правила_проєкту" по 4 логическим группам
PROJECT_RULES_GROUPS = {
    "01_Офіційні_правила_та_лідери.md": [
        "Офіційні правила проєкту NG GTA 5 RP",
        "Офіційні правила лідерів проєкту NG",
        "Ігрові Зони",
        "RP-біографій",
        "Правила сімейних організацій",
    ],
    "02_Держструктури_та_допити.md": [
        "Загальні правила державних організацій",
        "Правила допитів",
        "Захоплення державних організацій",
        "Правила введення військового стану",
    ],
    "03_Крайм_та_заходи.md": [
        "Загальні правила крайму",
        "Лор кримінальних організацій",
        "Заходи кримінальних організацій",
    ],
    "04_Поставки_рейди_ФЗ_ВЗХ_AirDrop.md": [
        "Правила поставок",
        "Правила рейду",
        "Правила нападу на ФЗ",
        "AirDrop",
        "ВЗХ",
    ],
}

# Папки законодательства и фракций -> целевой файл
FOLDER_GROUPS = {
    "02_Законодавство/Конституція": "05_Конституція_та_Кодекси.md",
    "02_Законодавство/Кодекси": "05_Конституція_та_Кодекси.md",
    "02_Законодавство/Закони": "06_Закони_штату_NG.md",
    "03_Фракції/НПУ": "07_Фракція_НПУ.md",
    "03_Фракції/НГУ": "08_Фракція_НГУ.md",
    "03_Фракції/СБУ": "09_Фракція_СБУ.md",
    "03_Фракції/ДБР": "10_Фракція_ДБР.md",
    "03_Фракції/ДВК": "11_Фракція_ДВК.md",
    "03_Фракції/ЕМД": "12_Фракція_ЕМД.md",
    "03_Фракції/ТСН": "13_Фракція_ТСН.md",
    "03_Фракції/ДВВ": "14_Фракція_ДВВ.md",
}


def strip_frontmatter(text: str) -> str:
    if text.startswith("---"):
        parts = text.split("---", 2)
        if len(parts) == 3:
            return parts[2].strip()
    return text.strip()


def pick_target(rel_path: Path) -> str | None:
    folder_str = rel_path.parent.as_posix()
    if folder_str == "01_Правила_проєкту":
        name = rel_path.name.lower()
        for out_file, keywords in PROJECT_RULES_GROUPS.items():
            if any(kw.lower() in name for kw in keywords):
                return out_file
        return "01_Офіційні_правила_та_лідери.md"

    return FOLDER_GROUPS.get(folder_str)


def main():
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    buckets: dict[str, list[str]] = {}
    packed = 0
    skipped = []

    for md in sorted(SORTED_DIR.rglob("*.md")):
        if "00_Вступ" in str(md) or md.name.startswith("_"):
            continue
        rel = md.relative_to(SORTED_DIR)
        target = pick_target(rel)
        if not target:
            skipped.append(str(rel))
            continue

        body = strip_frontmatter(md.read_text(encoding="utf-8"))
        buckets.setdefault(target, []).append(body)
        packed += 1

    for out_name, parts in sorted(buckets.items()):
        out_path = OUT_DIR / out_name
        merged = "\n\n---\n\n".join(parts) + "\n"
        out_path.write_text(merged, encoding="utf-8")
        words = len(merged.split())
        print(f"  {out_name:42} | разделов: {len(parts):2} | слов: {words:,}")

    print(f"\nГотово! Упаковано разделов: {packed} -> файлов: {len(buckets)}")
    print(f"Папка для загрузки в блокнот: {OUT_DIR.resolve()}")
    if skipped:
        print(f"[!] Пропущено: {skipped}")


if __name__ == "__main__":
    main()