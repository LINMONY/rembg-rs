# ✂️ rembg-rs — Fast AI Background Remover in Pure Rust

[**English**](#-english) | [**Русский**](#-русский)

---

## 🌐 English

**rembg-rs** is a lightweight, fast, standalone desktop application for removing backgrounds from images locally on your computer. Rewritten from scratch in **100% pure Rust**, it works as a single `.exe` file with **zero Python dependencies** and **no external DLLs**.

### ✨ Key Features

- **Single Portable `.exe`**: Powered by the pure-Rust [`tract-onnx`](https://github.com/sonos/tract) neural inference engine and native Win32 GUI — no Python, no CUDA setup, and no external `.dll` files required.
- **Convenient Native GUI**:
  - **Drag & Drop** any `.png`, `.jpg`, `.jpeg`, or `.webp` image directly into the window.
  - **Paste from Clipboard (`Ctrl+V`)** — supports screenshots (`Win+Shift+S`), images copied from browsers/messengers, and files copied in File Explorer.
  - **Manual Start & Instant Stop**: Preview your image first, press **`✂ Cut Out` (`Enter`)** to start, or **`⏹ Stop` (`Esc`)** at any moment.
  - **One-Click Export**: Copy the transparent PNG directly back to the clipboard (**`Ctrl+C`**, preserving full alpha transparency for Telegram, Discord, Photoshop, Figma, browsers) or save it to disk (**`Ctrl+S`**).
  - **Non-Blocking UI**: Model downloading, graph optimization, and neural inference run on a background worker thread so the window never freezes.
  - **Bilingual Interface**: English by default with a one-click **`🌐 EN ▸ RU`** switch in the top-right corner.
- **9 Curated AI Models with Built-in Guides**: Automatically downloads models on demand into `~/.rembg/models/` (compatible with original `rembg` cache) and explains when to use each model right inside the app.

### 🧠 Supported AI Models

| Model ID | Name | Resolution | Size | Best Used For |
| :--- | :--- | :---: | :---: | :--- |
| `u2net` | **U2-Net Standard** | `320×320` | 168 MB | Fast everyday cutouts of cars, products, animals, and objects with clear outlines. |
| `rmbg-1.4` | **⭐ BRIA RMBG 1.4** | `1024×1024` | 168 MB | High-precision model by BRIA AI. Best when `u2net` struggles with complex backgrounds, fur, hair, or e-commerce photos. |
| `isnet-general-use` | **IS-Net General** | `1024×1024` | 170 MB | High-resolution edge detection for thin wires, bicycle spokes, tree branches, and intricate geometry. |
| `isnet-anime` | **IS-Net Anime** | `1024×1024` | 168 MB | Purpose-built for anime characters, manga, stickers, VTubers, and 2D digital art. |
| `modnet` | **⭐ MODNet Portrait** | `512×512` | 25 MB | Lightweight portrait matting model for selfies, headshots, and soft semi-transparent hair edges. |
| `u2net_human_seg` | **U2-Net Human Seg** | `320×320` | 168 MB | Isolates people from background clutter (prevents chairs/objects next to a person from being kept). |
| `u2netp` | **U2-Netp Lite** | `320×320` | 4.4 MB | Ultra-fast pocket version of U2-Net. Downloads in 1 second and runs instantaneously. |
| `silueta` | **Silueta** | `320×320` | 42 MB | Compact 4× smaller version of U2-Net with nearly identical accuracy. |
| `u2net_cloth_seg` | **U2-Net Cloth Seg** | `768×768` | 168 MB | Fashion/apparel segmentation: extracts only the clothing while removing both background and skin/head. |

### 🚀 Building from Source

Make sure you have [Rust](https://rustup.rs/) installed (`x86_64-pc-windows-msvc`), then run:

```bash
cargo build --release
```

The compiled standalone executable will be located at:
```text
target/release/rembg-rs.exe
```

#### Command-Line Usage (Optional)
In addition to the graphical window (launched by double-clicking `rembg-rs.exe`), you can also run it from the terminal:
```bash
rembg-rs.exe input.jpg output.png [model_id]
```

---

## 🇷🇺 Русский

**rembg-rs** — лёгкая, быстрая и полностью автономная программа для удаления фона с картинок прямо на вашем компьютере. Проект полностью написан с нуля на **Rust** и компилируется в один независимый `.exe` файл — **без установки Python**, тяжёлых библиотек и внешних `.dll`.

### ✨ В чём кайф программы

- **Один лёгкий `.exe` файл**: внутри работает чисто-растовый нейросетевой движок [`tract-onnx`](https://github.com/sonos/tract) и нативное окно Win32 — программа запускается мгновенно на любом ПК с Windows.
- **Удобное графическое окно на каждый день**:
  - **Drag & Drop**: просто перетащите картинку (`.png`, `.jpg`, `.jpeg`, `.webp`) мышкой в окно.
  - **Вставка по `Ctrl+V`**: вставляйте скриншоты (`Win+Shift+S`), скопированные картинки из браузера/мессенджеров или файлы прямо из Проводника.
  - **Кнопки `✂ Вырезать` и `⏹ Остановить`**: обработка начинается по нажатию кнопки **`✂ Вырезать` (`Enter`)**, а в любой момент скачивания или вычисления процесс можно прервать кнопкой **`⏹ Остановить` (`Esc`)**.
  - **Быстрый экспорт**: скопируйте готовую картинку с прозрачностью обратно в буфер обмена (**`Ctrl+C`** — прозрачный PNG сразу вставляется в Telegram, Discord, Photoshop, Figma) или сохраните в файл (**`Ctrl+S`**).
  - **Без зависаний**: вся работа нейросети и докачка моделей идут в отдельном фоновом потоке — окно всегда остаётся плавным и отзывчивым.
  - **Два языка (EN / RU)**: переключение языка интерфейса и описаний моделей одной кнопкой **`🌐 EN ▸ RU`** в правом верхнем углу.
- **9 отборных нейросетей с подсказками**: прямо в окне можно выбрать модель под конкретную задачу (предметы, сложные детали, портреты, волосы, аниме, одежда) и прочитать подсказку, когда какую модель лучше использовать. Модели скачиваются автоматически при первом выборе в папку `~/.rembg/models/`.

### 🛠 Сборка из исходников

Установите [Rust](https://rustup.rs/), откройте терминал в папке проекта и выполните:

```bash
cargo build --release
```

Готовый к работе файл появится по пути:
```text
target\release\rembg-rs.exe
```

---

## 🙏 Acknowledgments / Благодарности

- Huge thanks to [**Daniel Gatis (`danielgatis/rembg`)**](https://github.com/danielgatis/rembg) for the original Python `rembg` project, inspiration, and hosted ONNX model releases.
- Model authors: **Xuebin Qin et al.** (*U2-Net*, *IS-Net / DIS*), **BRIA AI** (*RMBG-1.4*), **ZHKKKe** (*MODNet*), and the [**Sonos `tract`**](https://github.com/sonos/tract) team for the pure-Rust ONNX runtime.

## 📄 License

Licensed under the [MIT License](LICENSE) © 2026 **Женя Швец**.
