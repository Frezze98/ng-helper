<div align="center">

<img src="docs/logo.png" width="160" alt="NG Helper Logo" />

<h1>NG Helper</h1>

<p>
  <b>Ігровий оверлей для GTA 5 RP (проєкт NG)</b><br />
  Офлайн-пошук по базі правил, законів і статутів + розбір RP-ситуацій через ШІ (Gemini) поверх гри.
</p>

<p>
  <a href="https://github.com/Frezze98/ng-helper/releases/latest"><img src="https://img.shields.io/github/v/release/Frezze98/ng-helper?label=Download%20Setup&labelColor=101b17&color=26ffae&style=for-the-badge" alt="Download Setup" /></a>
  <a href="https://github.com/Frezze98/ng-helper/releases"><img src="https://img.shields.io/github/downloads/Frezze98/ng-helper/total?label=Downloads&labelColor=101b17&color=10b981&style=for-the-badge" alt="Total Downloads" /></a>
  <a href="https://www.virustotal.com/gui/file/c49c1157725623b1368c9a36c5b0f549aafc50cdea2d9ee59accd0a2919cdb01"><img src="https://img.shields.io/badge/VirusTotal-Report-394eff?style=for-the-badge&labelColor=101b17&logo=virustotal&logoColor=white" alt="VirusTotal" /></a>
</p>

<p>
  <a href="https://github.com/Frezze98/ng-helper/actions/workflows/release.yml"><img src="https://img.shields.io/github/actions/workflow/status/Frezze98/ng-helper/release.yml?label=Build&labelColor=101b17&style=flat-square" alt="Build Status" /></a>
  <img src="https://img.shields.io/badge/Windows-10%20%2F%2011-1f2937?style=flat-square&labelColor=101b17&logo=windows&logoColor=26ffae" alt="Windows 10/11" />
  <img src="https://img.shields.io/badge/Stack-Rust%20%2B%20WebView2-1f2937?style=flat-square&labelColor=101b17&logo=rust&logoColor=26ffae" alt="Rust + WebView2" />
</p>

</div>

---

## Скріншоти

<table>
  <tr>
    <td align="center" width="50%">
      <img src="docs/1.png" width="380" alt="Головне вікно та часті питання" /><br />
      <sub><b>Офлайн-пошук, фільтри та «Топ частих питань NG»</b></sub>
    </td>
    <td align="center" width="50%">
      <img src="docs/4.png" width="380" alt="Розбір ситуації через ШІ" /><br />
      <sub><b>Розбір RP-ситуацій через ШІ (IC + OOC + поради)</b></sub>
    </td>
  </tr>
  <tr>
    <td align="center" width="50%">
      <img src="docs/2.png" width="380" alt="Налаштування вигляду" /><br />
      <sub><b>Теми, акцентні кольори, мова та розмір вікна</b></sub>
    </td>
    <td align="center" width="50%">
      <img src="docs/3.png" width="380" alt="API-ключ та оновлення" /><br />
      <sub><b>API-ключ, гарячі клавіші, автооновлення та видалення</b></sub>
    </td>
  </tr>
</table>

---

## Основні можливості

- **Миттєвий виклик у грі:** відкриття та приховування оверлею по `Alt + Q`, фіксація поверх вікна гри (`📌`).
- **Офлайн-пошук без інтернету та ключів:** швидка навігація по вбудованій базі (понад **85 документів** та **2 300 розділів і статей**) з фільтрами `Усі`, `Правила`, `Закони`, `СБУ`, `НПУ`, `Крайм`.
- **Топ частих питань NG:** готові картки з найпопулярнішими правилами та термінами покарань із копіюванням в 1 клік.
- **ШІ-помічник (Gemini):** аналізує живу ситуацію за актуальною базою проєкту та розкладає відповідь на **IC (кодекси й закони)**, **OOC (правила сервера)** і конкретні дії.
- **Гнучка персоналізація:** 6 готових акцентних кольорів + власний колір, 3 теми фону (`Темний NG`, `OLED Чорний`, `Графіт`), вибір мови інтерфейсу та відповідей ШІ (`Українська` / `Русский`), стилю відповідей (`Детальний` / `Короткий`), розміру вікна й шрифту.
- **Зручне обслуговування:** вбудована перевірка й автооновлення з GitHub Releases, робота в треї та повне видалення однією кнопкою з налаштувань.

<details>
<summary><b>Що входить у вбудовану офлайн-базу правил (натисни, щоб розгорнути)</b></summary>

- **Правила проєкту NG (17 документів):** Офіційні правила проєкту, правила держорганізацій, допитів, лідерів, крайму, лор та заходи крайму, захоплення держорганізацій, воєнний стан, поставки, рейди, напад на ФЗ (НГУ), AirDrop, ВЗХ, ігрові зони, RP-біографії, сімейні організації.
- **Законодавство штату NG:**
  - Конституція штату NG
  - **7 Кодексів:** Кримінальний, Адміністративний, Процесуальний, Трудовий, Етичний, Повітряний, Дорожній
  - **18 Законів штату NG:** про адвокатуру, ДБР, державні документи, ДВВ, ДВК, держтаємницю, закриті території, воєнний стан, НГУ, НПУ, недоторканність, прокуратуру, охорону здоров'я (ЕМД), полювання та рибальство, зброю, СБУ, судоустрій, медіа
- **Статути та регламенти 8 фракцій:** НПУ, НГУ, СБУ, ДБР, ДВК, ЕМД, ТСН, ДВВ.

</details>

---

## Встановлення та запуск

1. Перейди у розділ [Releases](https://github.com/Frezze98/ng-helper/releases/latest) і завантаж **`Setup-NG-Helper.exe`**.
2. Запусти інсталятор, обери мову та за бажанням увімкни створення ярлика на робочому столі й автозапуск разом із Windows.
3. Запусти **NG Helper** — натискай `Alt + Q`, щоб миттєво відкрити або сховати оверлей.

> [!TIP]
> Щоб оверлей відображався поверх GTA 5, у налаштуваннях графіки гри має бути обрано режим **«Вікно без рамок» (Borderless Window)** або **«У вікні»**.

### Гарячі клавіші

| Дія | Керування |
| :--- | :--- |
| Відкрити / сховати оверлей | `Alt + Q` або клік ЛКМ по іконці в треї |
| Повністю закрити програму | `Alt + Shift + Q` або меню трею |
| Перемістити вікно | Перетягування за верхню шапку |

---

## Як отримати безкоштовний API-ключ Gemini

Офлайн-пошук і «Топ частих питань» працюють **одразу без жодного ключа**. Персональний ключ потрібен лише для кнопки **«ШІ»** (у кожного користувача свій безкоштовний ліміт, пароль від Google у програму вводити не потрібно):

1. Відкрий **Налаштування** в оверлеї та натисни **«Відкрити Google AI Studio у браузері»** (або перейди на [aistudio.google.com/apikey](https://aistudio.google.com/apikey)).
2. Увійди у свій Google-акаунт.
3. Натисни **Create API key**, скопіюй ключ, встав його у поле в налаштуваннях і натисни **Готово**.

---

## Безпека, VirusTotal та Windows SmartScreen

- **Відкритий код:** усі вихідні файли відкриті у цьому репозиторії.
- **Автоматична збірка:** інсталятор `Setup-NG-Helper.exe` компілюється не на локальному ПК, а автоматично на серверах **GitHub Actions** (див. `.github/workflows/release.yml`).
- **Перевірка VirusTotal (`v1.0.5`):** [відкрити звіт для `Setup-NG-Helper.exe`](https://www.virustotal.com/gui/file/c49c1157725623b1368c9a36c5b0f549aafc50cdea2d9ee59accd0a2919cdb01)
  - `SHA-256`: `c49c1157725623b1368c9a36c5b0f549aafc50cdea2d9ee59accd0a2919cdb01`

> [!NOTE]
> **Чому Windows може показати синє вікно «ОС Windows захистила цей ПК»?**
> Це стандартна реакція **Microsoft Defender SmartScreen** на нові безкоштовні open-source програми без платного комерційного сертифіката підпису коду (`Невідомий видавець`). Файл уже надіслано на перевірку в **Microsoft Security Intelligence**. Щоб запустити інсталятор, натисни **«Детальніше»** → **«Виконати все одно»**.

<details>
<summary><b>Де зберігаються налаштування та як повністю видалити програму?</b></summary>

- **Налаштування, кеш WebView2 та логи помилок (`error.log`):** `%LOCALAPPDATA%\NGHelper`
- **Файли програми:** `%LOCALAPPDATA%\Programs\NG Helper`
- **Видалення:** відкрий **Налаштування** → натисни **«Видалити повністю»** (або видали через стандартне меню *«Програми та засоби»* Windows).

</details>

---

## Контакти

- **Discord:** `@Frezze98`
- **GitHub:** [github.com/Frezze98/ng-helper](https://github.com/Frezze98/ng-helper)
