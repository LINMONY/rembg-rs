use std::env;
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Ru,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PostProcessMode {
    /// Standard min-max normalization `(pred - min) / (max - min)` on channel 0.
    MinMax,
    /// Apply `sigmoid(x)` first, then min-max normalization (used by BiRefNet family).
    SigmoidMinMax,
    /// Direct alpha matte in `0.0..=1.0` (used by MODNet portrait matting).
    DirectMatte,
    /// 4-class argmax segmentation `[1, 4, H, W]` for clothing (`u2net_cloth_seg`).
    ClothArgmax,
}

#[derive(Clone, Copy)]
pub struct ModelSpec {
    pub id: &'static str,
    pub filename: &'static str,
    pub title_en: &'static str,
    pub title_ru: &'static str,
    pub badge_en: &'static str,
    pub badge_ru: &'static str,
    pub desc_en: &'static str,
    pub desc_ru: &'static str,
    pub url: &'static str,
    pub size_mb: u32,
    pub input_w: u32,
    pub input_h: u32,
    pub mean: [f32; 3],
    pub std: [f32; 3],
    pub postprocess: PostProcessMode,
}

impl ModelSpec {
    pub fn title(&self, lang: Lang) -> &'static str {
        match lang {
            Lang::En => self.title_en,
            Lang::Ru => self.title_ru,
        }
    }

    pub fn badge(&self, lang: Lang) -> &'static str {
        match lang {
            Lang::En => self.badge_en,
            Lang::Ru => self.badge_ru,
        }
    }

    pub fn desc(&self, lang: Lang) -> &'static str {
        match lang {
            Lang::En => self.desc_en,
            Lang::Ru => self.desc_ru,
        }
    }

    pub fn local_path(&self) -> PathBuf {
        let primary = rembg_home().join("models").join(self.id).join(self.filename);
        let legacy = legacy_home().join(self.filename);
        for candidate in [&primary, &legacy] {
            if let Ok(meta) = candidate.metadata() {
                if meta.len() > 1_000_000 {
                    return candidate.clone();
                }
            }
        }
        primary
    }

    pub fn is_downloaded(&self) -> bool {
        let p = self.local_path();
        p.exists() && p.metadata().map(|m| m.len() > 1_000_000).unwrap_or(false)
    }
}

pub fn home_dir() -> PathBuf {
    env::var_os("USERPROFILE")
        .or_else(|| env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn legacy_home() -> PathBuf {
    if let Some(u2net_home) = env::var_os("U2NET_HOME") {
        return PathBuf::from(u2net_home);
    }
    if let Some(xdg) = env::var_os("XDG_DATA_HOME") {
        return PathBuf::from(xdg).join(".u2net");
    }
    home_dir().join(".u2net")
}

pub fn rembg_home() -> PathBuf {
    if env::var_os("U2NET_HOME").is_some() {
        return legacy_home();
    }
    if let Some(rembg) = env::var_os("REMBG_HOME") {
        return PathBuf::from(rembg);
    }
    if let Some(xdg) = env::var_os("XDG_DATA_HOME") {
        return PathBuf::from(xdg).join("rembg");
    }
    home_dir().join(".rembg")
}

pub const MODELS: &[ModelSpec] = &[
    // 1. Classic U2-Net (Default)
    ModelSpec {
        id: "u2net",
        filename: "u2net.onnx",
        title_en: "U2-Net (Standard Universal)",
        title_ru: "U2-Net (Базовая универсальная)",
        badge_en: "Fast • 320×320 • 168 MB",
        badge_ru: "Быстрая • 320×320 • 168 МБ",
        desc_en: "Best everyday starting point. Fast (~1 sec) and reliable for cars, products, animals, and objects with clear outlines.",
        desc_ru: "Универсальная модель на каждый день. Быстро (~1 сек) вырезает предметы, авто, товары и животных с чётким контуром.",
        url: "https://github.com/danielgatis/rembg/releases/download/v0.0.0/u2net.onnx",
        size_mb: 168,
        input_w: 320,
        input_h: 320,
        mean: [0.485, 0.456, 0.406],
        std: [0.229, 0.224, 0.225],
        postprocess: PostProcessMode::MinMax,
    },
    // 2. BRIA RMBG 1.4 (External Top Choice from HuggingFace)
    ModelSpec {
        id: "rmbg-1.4",
        filename: "rmbg-1.4.onnx",
        title_en: "⭐ BRIA RMBG 1.4 (High-Detail Universal)",
        title_ru: "⭐ BRIA RMBG 1.4 (Высокая точность деталей)",
        badge_en: "Recommended HD • 1024×1024 • 168 MB",
        badge_ru: "Топ для деталей • 1024×1024 • 168 МБ",
        desc_en: "Top-rated community model by BRIA AI (HuggingFace). Choose when U2-Net makes mistakes on complex backgrounds, fur, hair, or e-commerce items.",
        desc_ru: "Знаменитая внешняя модель от BRIA AI (HuggingFace). Выбирайте, когда базовый U2-Net ошибается на сложном фоне, шерсти, волосах или товарах.",
        url: "https://huggingface.co/briaai/RMBG-1.4/resolve/main/onnx/model.onnx",
        size_mb: 168,
        input_w: 1024,
        input_h: 1024,
        mean: [0.5, 0.5, 0.5],
        std: [1.0, 1.0, 1.0],
        postprocess: PostProcessMode::MinMax,
    },
    // 3. IS-Net General Use (rembg HD)
    ModelSpec {
        id: "isnet-general-use",
        filename: "isnet-general-use.onnx",
        title_en: "IS-Net General (Fine Edges & Thin Structures)",
        title_ru: "IS-Net General (Тонкие линии и мелкие края)",
        badge_en: "HD Edges • 1024×1024 • 170 MB",
        badge_ru: "Чёткие края • 1024×1024 • 170 МБ",
        desc_en: "High-resolution DIS model from rembg. Best for objects with thin wires, bicycle spokes, chairs, plants, and intricate geometry.",
        desc_ru: "Высокоточная модель из rembg (1024×1024). Отлично сохраняет тонкие провода, спицы колёс, ветки растений, стулья и сложную геометрию.",
        url: "https://github.com/danielgatis/rembg/releases/download/v0.0.0/isnet-general-use.onnx",
        size_mb: 170,
        input_w: 1024,
        input_h: 1024,
        mean: [0.5, 0.5, 0.5],
        std: [1.0, 1.0, 1.0],
        postprocess: PostProcessMode::MinMax,
    },
    // 4. IS-Net Anime (rembg Anime / 2D Art)
    ModelSpec {
        id: "isnet-anime",
        filename: "isnet-anime.onnx",
        title_en: "IS-Net Anime (Anime, Manga & 2D Illustrations)",
        title_ru: "IS-Net Anime (Аниме, арты, стикеры и 2D)",
        badge_en: "Anime & Art • 1024×1024 • 168 MB",
        badge_ru: "Аниме и 2D-арт • 1024×1024 • 168 МБ",
        desc_en: "Specifically trained on anime characters, game art, stickers, and 2D illustrations. Accurately keeps spiky hair strands and drawn outlines.",
        desc_ru: "Создана специально для аниме-персонажей, 2D-артов, стикеров и игровых рендеров. Идеально вырезает тонкие пряди волос и рисованные контуры.",
        url: "https://github.com/danielgatis/rembg/releases/download/v0.0.0/isnet-anime.onnx",
        size_mb: 168,
        input_w: 1024,
        input_h: 1024,
        mean: [0.485, 0.456, 0.406],
        std: [1.0, 1.0, 1.0],
        postprocess: PostProcessMode::MinMax,
    },
    // 5. MODNet Portrait Matting (External HuggingFace model)
    ModelSpec {
        id: "modnet",
        filename: "modnet.onnx",
        title_en: "⭐ MODNet Portrait (Soft Hair & People Matting)",
        title_ru: "⭐ MODNet Portrait (Портреты и мягкие волосы)",
        badge_en: "Portraits • 512×512 • 25 MB",
        badge_ru: "Портреты • 512×512 • 25 МБ",
        desc_en: "Lightweight external portrait matting model (HuggingFace). Ideal for selfies, headshots, and natural soft alpha edges on human hair.",
        desc_ru: "Популярная внешняя модель портретного матирования (25 МБ). Лучший выбор для селфи, аватарок и мягких полупрозрачных краёв женских/пышных волос.",
        url: "https://huggingface.co/Xenova/modnet/resolve/main/onnx/model.onnx",
        size_mb: 25,
        input_w: 512,
        input_h: 512,
        mean: [0.5, 0.5, 0.5],
        std: [0.5, 0.5, 0.5],
        postprocess: PostProcessMode::DirectMatte,
    },
    // 6. U2-Net Human Seg (rembg People)
    ModelSpec {
        id: "u2net_human_seg",
        filename: "u2net_human_seg.onnx",
        title_en: "U2-Net Human Seg (Full-Body People & Portraits)",
        title_ru: "U2-Net Human Seg (Люди в полный рост и портреты)",
        badge_en: "People • 320×320 • 168 MB",
        badge_ru: "Люди • 320×320 • 168 МБ",
        desc_en: "Trained exclusively to isolate humans from background clutter. Use when general models accidentally keep objects next to a person.",
        desc_ru: "Обучена выделять только людей. Используйте на фотографиях людей, если универсальные модели оставляют стул, стол или предметы рядом с человеком.",
        url: "https://github.com/danielgatis/rembg/releases/download/v0.0.0/u2net_human_seg.onnx",
        size_mb: 168,
        input_w: 320,
        input_h: 320,
        mean: [0.485, 0.456, 0.406],
        std: [0.229, 0.224, 0.225],
        postprocess: PostProcessMode::MinMax,
    },
    // 7. U2-Netp (rembg Pocket / Ultra-Fast)
    ModelSpec {
        id: "u2netp",
        filename: "u2netp.onnx",
        title_en: "U2-Netp Lite (Ultra-Fast Pocket Model)",
        title_ru: "U2-Netp Lite (Сверхбыстрая лёгкая модель)",
        badge_en: "Ultra-Fast • 320×320 • 4.4 MB",
        badge_ru: "Мгновенная • 320×320 • 4.4 МБ",
        desc_en: "Tiny 4.4 MB version of U2-Net. Downloads in 1 second and processes images almost instantaneously. Best for simple icons, logos, or quick previews.",
        desc_ru: "Карманная версия U2-Net весом всего 4.4 МБ. Работает почти мгновенно — подходит для простых объектов, логотипов, иконок и быстрых тестов.",
        url: "https://github.com/danielgatis/rembg/releases/download/v0.0.0/u2netp.onnx",
        size_mb: 4,
        input_w: 320,
        input_h: 320,
        mean: [0.485, 0.456, 0.406],
        std: [0.229, 0.224, 0.225],
        postprocess: PostProcessMode::MinMax,
    },
    // 8. Silueta (rembg Compact Universal)
    ModelSpec {
        id: "silueta",
        filename: "silueta.onnx",
        title_en: "Silueta (Compact General Model)",
        title_ru: "Silueta (Компактная универсальная)",
        badge_en: "Compact • 320×320 • 42 MB",
        badge_ru: "Компактная • 320×320 • 42 МБ",
        desc_en: "Pruned 42 MB version of U2-Net that delivers almost identical cutouts at 1/4th the file size. Great balance of speed and quality.",
        desc_ru: "Уменьшенная в 4 раза версия U2-Net (42 МБ), сохраняющая почти ту же точность. Отличный баланс между размером файла и качеством.",
        url: "https://github.com/danielgatis/rembg/releases/download/v0.0.0/silueta.onnx",
        size_mb: 42,
        input_w: 320,
        input_h: 320,
        mean: [0.485, 0.456, 0.406],
        std: [0.229, 0.224, 0.225],
        postprocess: PostProcessMode::MinMax,
    },
    // 9. U2-Net Cloth Seg (rembg Apparel / Fashion)
    ModelSpec {
        id: "u2net_cloth_seg",
        filename: "u2net_cloth_seg.onnx",
        title_en: "U2-Net Cloth Seg (Cut Out Clothing Only)",
        title_ru: "U2-Net Cloth Seg (Вырезать только одежду)",
        badge_en: "Clothing Only • 768×768 • 168 MB",
        badge_ru: "Только одежда • 768×768 • 168 МБ",
        desc_en: "Specialized fashion model: keeps only the garments (shirts, pants, dresses) and removes both the background AND the person's face/skin.",
        desc_ru: "Специальная модель для каталогов одежды: оставляет только саму одежду на человеке, удаляя и фон, и голову/руки/ноги модели.",
        url: "https://github.com/danielgatis/rembg/releases/download/v0.0.0/u2net_cloth_seg.onnx",
        size_mb: 168,
        input_w: 768,
        input_h: 768,
        mean: [0.485, 0.456, 0.406],
        std: [0.229, 0.224, 0.225],
        postprocess: PostProcessMode::ClothArgmax,
    },
];
