import json
import re
from pathlib import Path
from docx import Document
from docx.oxml.table import CT_Tbl
from docx.oxml.text.paragraph import CT_P
from docx.table import Table
from docx.text.paragraph import Paragraph

INPUT_DOCX = Path("Правила NG.docx")
OUTPUT_DIR = Path("sorted_rules")

# Полный каталог из меню сайта NG (86 документов)
CATALOG = [
    # --- 01. ПРАВИЛА ПРОЄКТУ ---
    ("01_Правила_проєкту", "Офіційні правила проєкту NG GTA 5 RP"),
    ("01_Правила_проєкту", "Загальні правила державних організацій"),
    ("01_Правила_проєкту", "Правила допитів"),
    ("01_Правила_проєкту", "Офіційні правила лідерів проєкту NG"),
    ("01_Правила_проєкту", "Загальні правила крайму"),
    ("01_Правила_проєкту", "Лор кримінальних організацій"),
    ("01_Правила_проєкту", "Заходи кримінальних організацій"),
    ("01_Правила_проєкту", "Захоплення державних організацій"),
    ("01_Правила_проєкту", "Правила введення військового стану"),
    ("01_Правила_проєкту", "Правила поставок"),
    ("01_Правила_проєкту", "Правила рейду Crime орг."),
    ("01_Правила_проєкту", "Правила нападу на ФЗ (НГУ)"),
    ("01_Правила_проєкту", "Війна за AirDrop"),
    ("01_Правила_проєкту", "Війна за Хамери (ВЗХ)"),
    ("01_Правила_проєкту", "Ігрові Зони"),
    ("01_Правила_проєкту", "Правила подачі, написання та оформлення особистих і сімейних RP-біографій"),
    ("01_Правила_проєкту", "Правила сімейних організацій"),

    # --- 02. ЗАКОНОДАВСТВО ---
    ("02_Законодавство/Конституція", "Конституція штату NG"),
    ("02_Законодавство/Кодекси", "Кримінальний кодекс штату NG"),
    ("02_Законодавство/Кодекси", "Адміністративний кодекс штату NG"),
    ("02_Законодавство/Кодекси", "Процесуальний кодекс штату NG"),
    ("02_Законодавство/Кодекси", "Трудовий кодекс штату NG"),
    ("02_Законодавство/Кодекси", "Етичний кодекс штату NG"),
    ("02_Законодавство/Кодекси", "Повітряний кодекс штату NG"),
    ("02_Законодавство/Кодекси", "Дорожній кодекс штату NG"),
    ("02_Законодавство/Закони", "Закон про Адвокатуру та адвокатську діяльність штату NG"),
    ("02_Законодавство/Закони", "Закон про Державне Бюро Розслідувань (ДБР) штату NG"),
    ("02_Законодавство/Закони", "Закон про Державні документи штату NG"),
    ("02_Законодавство/Закони", "Закон про Державну Виконавчу Владу (ДВВ) та Кабінет Міністрів штату NG"),
    ("02_Законодавство/Закони", "Закон про Державну Виправну Колонію (ДВК)"),
    ("02_Законодавство/Закони", "Закон про Державну та Службову таємницю штату NG"),
    ("02_Законодавство/Закони", "Закон про Закриті та охоронювані території штату NG"),
    ("02_Законодавство/Закони", "Закон про Надзвичайний та воєнний стан на території штату NG"),
    ("02_Законодавство/Закони", "Закон про Національну гвардію України (НГУ) штату NG"),
    ("02_Законодавство/Закони", "Закон про Національну Поліцію України (НПУ) штату NG"),
    ("02_Законодавство/Закони", "Закон про Недоторканність штату NG"),
    ("02_Законодавство/Закони", "Закон про Органи прокуратури штату NG"),
    ("02_Законодавство/Закони", "Закон про Охорону здоров'я та діяльність Екстреної медичної допомоги (ЕМД) штату NG"),
    ("02_Законодавство/Закони", "Закон про Полювання та рибальство штату NG"),
    ("02_Законодавство/Закони", "Закон про Регулювання обігу зброї, боєприпасів та спецзасобів штату NG"),
    ("02_Законодавство/Закони", "Закон про Службу Безпеки України (СБУ) штату NG"),
    ("02_Законодавство/Закони", "Закон про Судоустрій та здійснення правосуддя штату NG"),
    ("02_Законодавство/Закони", "Закон про медіа штату NG"),

    # --- 03. ФРАКЦІЇ: НПУ ---
    ("03_Фракції/НПУ", "Внутрішній статут НПУ"),
    ("03_Фракції/НПУ", "Дисциплінарний статут НПУ"),
    ("03_Фракції/НПУ", "(( ООС Дисциплінарні стягнення НПУ ))"),
    ("03_Фракції/НПУ", "Система підвищень НПУ"),
    ("03_Фракції/НПУ", "Тен-Коди НПУ"),

    # --- 03. ФРАКЦІЇ: НГУ ---
    ("03_Фракції/НГУ", "Статут НГУ"),

    # --- 03. ФРАКЦІЇ: СБУ ---
    ("03_Фракції/СБУ", "Внутрішній статут СБУ"),
    ("03_Фракції/СБУ", "Загальні положення СБУ"),
    ("03_Фракції/СБУ", "Положення про захист службової інформації та режим конфіденційності в СБУ"),
    ("03_Фракції/СБУ", "Положення про організацію та проходження стажування в СБУ"),
    ("03_Фракції/СБУ", "Положення про службову форму, жетон, бейджик та боді-камеру в СБУ"),
    ("03_Фракції/СБУ", "Присяга СБУ"),
    ("03_Фракції/СБУ", "Дисциплінарні стягнення СБУ"),
    ("03_Фракції/СБУ", "Заборони при виконанні службових обов'язків СБУ"),
    ("03_Фракції/СБУ", "Інструкція з порядку затримання та застосування сили співробітниками СБУ"),
    ("03_Фракції/СБУ", "Регламент проведення набору, прийому на службу та звільнення в СБУ"),

    # --- 03. ФРАКЦІЇ: ДБР ---
    ("03_Фракції/ДБР", "Дисциплінарний статут ДБР"),
    ("03_Фракції/ДБР", "Статут ДБР"),
    ("03_Фракції/ДБР", "(( ООС правила ДБР ))"),
    ("03_Фракції/ДБР", "Система підвищення ДБР"),
    ("03_Фракції/ДБР", "Інструкція про службову відповідальність ДБР"),

    # --- 03. ФРАКЦІЇ: ДВК ---
    ("03_Фракції/ДВК", "Внутрішній статут ДВК"),
    ("03_Фракції/ДВК", "Дисциплінарний статут ДВК"),
    ("03_Фракції/ДВК", "Основні положення ДВК"),
    ("03_Фракції/ДВК", "Положення про дисциплінарну відповідальність співробітників ДВК"),
    ("03_Фракції/ДВК", "(( ООС правила ДВК ))"),
    ("03_Фракції/ДВК", "Етапи підвищення ДВК"),
    ("03_Фракції/ДВК", "Обов'язки та посадові інструкції ДВК"),
    ("03_Фракції/ДВК", "Тен-коди ДВК"),
    ("03_Фракції/ДВК", "Екіпіровка та форма ДВК"),

    # --- 03. ФРАКЦІЇ: ЕМД ---
    ("03_Фракції/ЕМД", "Внутрішній статут ЕМД"),
    ("03_Фракції/ЕМД", "Дисциплінарний статут ЕМД"),
    ("03_Фракції/ЕМД", "Загальні положення ЕМД"),
    ("03_Фракції/ЕМД", "Система підвищень ЕМД"),
    ("03_Фракції/ЕМД", "Заява на відновлення в ЕМД"),
    ("03_Фракції/ЕМД", "Фармакологія ЕМД"),

    # --- 03. ФРАКЦІЇ: ТСН ---
    ("03_Фракції/ТСН", "Внутрішній статут ТСН"),
    ("03_Фракції/ТСН", "Порядок оскарження ТСН"),
    ("03_Фракції/ТСН", "Забезпечення ТСН"),
    ("03_Фракції/ТСН", "Правила проведення ефірів ТСН (П.П.Е.)"),
    ("03_Фракції/ТСН", "Правила редагування оголошень ТСН (П.Р.О.)"),

    # --- 03. ФРАКЦІЇ: ДВВ ---
    ("03_Фракції/ДВВ", "Внутрішній статут Державної Виконавчої Влади (ДВВ) штату NG"),
    ("03_Фракції/ДВВ", "Загальна система підвищення Державної Виконавчої Влади (ДВВ)"),
]

# Строки-маркеры сайта, которые стоят прямо перед заголовком нового раздела
BREADCRUMB_MARKERS = {
    "проєкт ng gta 5 rp",
    "законодавство штату ng",
    "правила проєкту",
    "фракції",
    "конституція",
    "кодекси",
    "закони",
}

# Для нормализации кириллицы/латиницы при сравнении (например, OOC vs ООС)
HOMOGLYPHS = str.maketrans({
    "o": "о", "c": "с", "a": "а", "e": "е",
    "i": "і", "p": "р", "x": "х", "k": "к",
})


def normalize_for_match(text: str) -> str:
    text = text.replace("\xa0", " ").replace("\u200b", "")
    text = re.sub(r"[#✎]+", "", text)
    text = re.sub(r"[`’ʼ‘´]", "'", text)
    text = text.lower().strip()
    text = text.translate(HOMOGLYPHS)
    text = re.sub(r"\(\(\s*", "((", text)
    text = re.sub(r"\s*\)\)", "))", text)
    text = re.sub(r"\s+", " ", text)
    return text


def clean_line(text: str) -> str:
    text = text.replace("\xa0", " ").replace("\u200b", "")
    text = text.replace("#✎", "").replace("✎", "")
    text = re.sub(r"[`’ʼ]", "'", text)
    text = re.sub(r"[ \t]+", " ", text).strip()
    return text


def sanitize_filename(name: str) -> str:
    name = re.sub(r'[<>:"/\\|?*]', "", name)
    name = re.sub(r"\s+", " ", name).strip(" .")
    return name[:120]


def iter_block_items(doc: Document):
    """Читает абзацы и таблицы строго в порядке их следования в документе."""
    for child in doc.element.body.iterchildren():
        if isinstance(child, CT_P):
            yield Paragraph(child, doc)
        elif isinstance(child, CT_Tbl):
            yield Table(child, doc)


def table_to_markdown(table: Table) -> str:
    rows = []
    for row in table.rows:
        cells = [clean_line(cell.text).replace("\n", " ") for cell in row.cells]
        if any(cells):
            rows.append(cells)
    if not rows:
        return ""
    max_cols = max(len(r) for r in rows)
    rows = [r + [""] * (max_cols - len(r)) for r in rows]
    lines = [
        "| " + " | ".join(rows[0]) + " |",
        "| " + " | ".join(["---"] * max_cols) + " |",
    ]
    for r in rows[1:]:
        lines.append("| " + " | ".join(r) + " |")
    return "\n".join(lines)


def format_heading_if_needed(line: str, is_site_heading: bool) -> str:
    # Заголовки подразделов с сайта (раньше оканчивались на #✎) делаем красивыми ##
    if is_site_heading and not re.match(r"^\d+\.\d+", line):
        return f"## {line}"
    return line


def main():
    if not INPUT_DOCX.exists():
        print(f"Файл не найден: {INPUT_DOCX.resolve()}")
        return

    print(f"Читаю {INPUT_DOCX.name}...")
    doc = Document(INPUT_DOCX)

    # Индекс каталога: нормализованное название -> (папка, каноничное название)
    title_map = {
        normalize_for_match(title): (folder, title)
        for folder, title in CATALOG
    }

    sections = []  # список словарей: {folder, title, lines}
    current = None
    seen_titles = set()
    unmatched_after_breadcrumb = []
    prev_was_breadcrumb = False

    for block in iter_block_items(doc):
        if isinstance(block, Table):
            md_table = table_to_markdown(block)
            if md_table and current is not None:
                current["lines"].append("")
                current["lines"].append(md_table)
                current["lines"].append("")
            prev_was_breadcrumb = False
            continue

        raw_text = block.text or ""
        is_site_heading = "#✎" in raw_text
        line = clean_line(raw_text)
        if not line or line == "|":
            continue

        norm = normalize_for_match(line)

        # Проверяем, является ли строка "хлебной крошкой" перед новым документом
        if norm in BREADCRUMB_MARKERS:
            prev_was_breadcrumb = True
            continue

        # Проверяем совпадение с названием документа из каталога
        if norm in title_map:
            folder, canonical_title = title_map[norm]
            # Защита от случайного дубля в тексте: если уже нашли такой документ и он не пустой — не рвём его
            already_has_content = (
                canonical_title in seen_titles
                and any(s["title"] == canonical_title and len(s["lines"]) > 5 for s in sections)
            )
            if not already_has_content:
                current = {
                    "folder": folder,
                    "title": canonical_title,
                    "lines": [],
                }
                sections.append(current)
                seen_titles.add(canonical_title)
                prev_was_breadcrumb = False
                continue

        # Если прямо перед строкой была крошка (например "Проєкт NG GTA 5 RP"), но заголовок не совпал с каталогом —
        # фиксируем, чтобы увидеть, не изменилось ли название в самом .docx
        if prev_was_breadcrumb:
            unmatched_after_breadcrumb.append(line)
            prev_was_breadcrumb = False

        if current is None:
            current = {
                "folder": "00_Вступ_або_Нерозпізнане",
                "title": "Початок_документа",
                "lines": [],
            }
            sections.append(current)

        formatted = format_heading_if_needed(line, is_site_heading)
        current["lines"].append(formatted)

    # Сохраняем всё в папку sorted_rules/
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    manifest = []

    for idx, sec in enumerate(sections, start=1):
        folder_path = OUTPUT_DIR / sec["folder"]
        folder_path.mkdir(parents=True, exist_ok=True)

        filename = f"{idx:02d}_{sanitize_filename(sec['title'])}.md"
        file_path = folder_path / filename

        content = (
            f"---\n"
            f"category: {sec['folder']}\n"
            f"title: {sec['title']}\n"
            f"---\n\n"
            f"# {sec['title']}\n\n"
            + "\n".join(sec["lines"])
            + "\n"
        )
        file_path.write_text(content, encoding="utf-8")

        manifest.append({
            "file": str(file_path.relative_to(OUTPUT_DIR)),
            "category": sec["folder"],
            "title": sec["title"],
            "lines": len(sec["lines"]),
            "chars": len(content),
        })

    (OUTPUT_DIR / "_manifest.json").write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2),
        encoding="utf-8",
    )

    # Отчёт
    found_titles = {s["title"] for s in sections}
    missing = [title for _, title in CATALOG if title not in found_titles]

    print(f"\nГотово! Создано файлов: {len(sections)} (в каталоге: {len(CATALOG)})")
    print(f"Результат сохранён в: {OUTPUT_DIR.resolve()}")

    if missing:
        print(f"\n[!] Не найдено точных совпадений ({len(missing)}):")
        for m in missing:
            print(f"   - {m}")

    if unmatched_after_breadcrumb:
        print(f"\n[?] Строки после разделителя сайта, не совпавшие с каталогом ({len(unmatched_after_breadcrumb)}):")
        for u in unmatched_after_breadcrumb:
            print(f"   - {u}")

    # Показываем самые маленькие файлы (вдруг какой-то раздел пустой или склеился)
    tiny = [m for m in manifest if m["lines"] < 5]
    if tiny:
        print(f"\n[!] Подозрительно короткие файлы (< 5 строк):")
        for t in tiny:
            print(f"   - {t['title']} ({t['lines']} строк)")


if __name__ == "__main__":
    main()