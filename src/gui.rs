#![allow(non_snake_case, non_camel_case_types, clippy::upper_case_acronyms)]

use std::ffi::c_void;
use std::mem::{size_of, zeroed};
use std::path::PathBuf;
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

use image::codecs::png::PngEncoder;
use image::{ColorType, DynamicImage, ImageEncoder, RgbaImage};

use crate::models::{Lang, ModelSpec, MODELS};
use crate::RembgEngine;

type HWND = usize;
type HDC = usize;
type HGDIOBJ = usize;
type HBITMAP = usize;
type HBRUSH = usize;
type HPEN = usize;
type HFONT = usize;
type HICON = usize;
type HCURSOR = usize;
type HMENU = usize;
type HINSTANCE = usize;
type HGLOBAL = usize;
type HDROP = usize;
type LRESULT = isize;
type WPARAM = usize;
type LPARAM = isize;
type BOOL = i32;
type UINT = u32;
type DWORD = u32;
type WORD = u16;
type COLORREF = u32;

const WS_OVERLAPPEDWINDOW: DWORD = 0x00CF_0000;
const WS_VISIBLE: DWORD = 0x1000_0000;
const WS_CLIPCHILDREN: DWORD = 0x0200_0000;
const CS_HREDRAW: UINT = 0x0002;
const CS_VREDRAW: UINT = 0x0001;

const WM_CREATE: UINT = 0x0001;
const WM_DESTROY: UINT = 0x0002;
const WM_SIZE: UINT = 0x0005;
const WM_PAINT: UINT = 0x000F;
const WM_CLOSE: UINT = 0x0010;
const WM_ERASEBKGND: UINT = 0x0014;
const WM_GETMINMAXINFO: UINT = 0x0024;
const WM_SETCURSOR: UINT = 0x0020;
const WM_KEYDOWN: UINT = 0x0100;
const WM_TIMER: UINT = 0x0113;
const WM_MOUSEMOVE: UINT = 0x0200;
const WM_LBUTTONUP: UINT = 0x0202;
const WM_DROPFILES: UINT = 0x0233;
const WM_APP_WORKER_MSG: UINT = 0x8001;

const VK_RETURN: usize = 0x0D;
const VK_ESCAPE: usize = 0x1B;
const VK_CONTROL: i32 = 0x11;
const VK_V: usize = 0x56;
const VK_C: usize = 0x43;
const VK_S: usize = 0x53;
const VK_O: usize = 0x4F;

const IDC_ARROW: *const u16 = 32512 as *const u16;
const IDC_HAND: *const u16 = 32649 as *const u16;
const IDI_APPLICATION: *const u16 = 32512 as *const u16;

const BI_RGB: DWORD = 0;
const BI_BITFIELDS: DWORD = 3;
const DIB_RGB_COLORS: UINT = 0;
const SRCCOPY: DWORD = 0x00CC_0020;
const TRANSPARENT: i32 = 1;

const DT_LEFT: UINT = 0x0000;
const DT_CENTER: UINT = 0x0001;
const DT_VCENTER: UINT = 0x0004;
const DT_WORDBREAK: UINT = 0x0010;
const DT_SINGLELINE: UINT = 0x0020;
const DT_CALCRECT: UINT = 0x0400;
const DT_NOPREFIX: UINT = 0x0800;
const DT_END_ELLIPSIS: UINT = 0x8000;

const CF_DIB: UINT = 8;
const CF_HDROP: UINT = 15;
const GMEM_MOVEABLE: UINT = 0x0002;

const OFN_OVERWRITEPROMPT: DWORD = 0x0000_0002;
const OFN_PATHMUSTEXIST: DWORD = 0x0000_0800;
const OFN_FILEMUSTEXIST: DWORD = 0x0000_1000;
const OFN_NOCHANGEDIR: DWORD = 0x0000_0008;

const MF_STRING: UINT = 0x0000;
const MF_CHECKED: UINT = 0x0008;
const TPM_LEFTALIGN: UINT = 0x0000;
const TPM_TOPALIGN: UINT = 0x0000;
const TPM_RETURNCMD: UINT = 0x0100;
const TPM_NONOTIFY: UINT = 0x0080;

const TIMER_SPINNER_ID: usize = 1;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct POINT {
    x: i32,
    y: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct RECT {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

impl RECT {
    fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.left && x < self.right && y >= self.top && y < self.bottom
    }
}

#[repr(C)]
struct MINMAXINFO {
    ptReserved: POINT,
    ptMaxSize: POINT,
    ptMaxPosition: POINT,
    ptMinTrackSize: POINT,
    ptMaxTrackSize: POINT,
}

#[repr(C)]
struct MSG {
    hwnd: HWND,
    message: UINT,
    wParam: WPARAM,
    lParam: LPARAM,
    time: DWORD,
    pt: POINT,
}

#[repr(C)]
struct WNDCLASSEXW {
    cbSize: UINT,
    style: UINT,
    lpfnWndProc: unsafe extern "system" fn(HWND, UINT, WPARAM, LPARAM) -> LRESULT,
    cbClsExtra: i32,
    cbWndExtra: i32,
    hInstance: HINSTANCE,
    hIcon: HICON,
    hCursor: HCURSOR,
    hbrBackground: HBRUSH,
    lpszMenuName: *const u16,
    lpszClassName: *const u16,
    hIconSm: HICON,
}

#[repr(C)]
struct PAINTSTRUCT {
    hdc: HDC,
    fErase: BOOL,
    rcPaint: RECT,
    fRestore: BOOL,
    fIncUpdate: BOOL,
    rgbReserved: [u8; 32],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct BITMAPINFOHEADER {
    biSize: DWORD,
    biWidth: i32,
    biHeight: i32,
    biPlanes: WORD,
    biBitCount: WORD,
    biCompression: DWORD,
    biSizeImage: DWORD,
    biXPelsPerMeter: i32,
    biYPelsPerMeter: i32,
    biClrUsed: DWORD,
    biClrImportant: DWORD,
}

#[repr(C)]
struct BITMAPINFO {
    bmiHeader: BITMAPINFOHEADER,
    bmiColors: [u32; 1],
}

#[repr(C)]
struct OPENFILENAMEW {
    lStructSize: DWORD,
    hwndOwner: HWND,
    hInstance: HINSTANCE,
    lpstrFilter: *const u16,
    lpstrCustomFilter: *mut u16,
    nMaxCustFilter: DWORD,
    nFilterIndex: DWORD,
    lpstrFile: *mut u16,
    nMaxFile: DWORD,
    lpstrFileTitle: *mut u16,
    nMaxFileTitle: DWORD,
    lpstrInitialDir: *const u16,
    lpstrTitle: *const u16,
    Flags: DWORD,
    nFileOffset: WORD,
    nFileExtension: WORD,
    lpstrDefExt: *const u16,
    lCustData: isize,
    lpfnHook: usize,
    lpTemplateName: *const u16,
    pvReserved: *mut c_void,
    dwReserved: DWORD,
    FlagsEx: DWORD,
}

#[link(name = "kernel32")]
extern "system" {
    fn GetModuleHandleW(lpModuleName: *const u16) -> HINSTANCE;
    fn GlobalAlloc(uFlags: UINT, dwBytes: usize) -> HGLOBAL;
    fn GlobalLock(hMem: HGLOBAL) -> *mut u8;
    fn GlobalUnlock(hMem: HGLOBAL) -> BOOL;
    fn GlobalSize(hMem: HGLOBAL) -> usize;
    fn GlobalFree(hMem: HGLOBAL) -> HGLOBAL;
}

#[link(name = "user32")]
extern "system" {
    fn SetProcessDPIAware() -> BOOL;
    fn RegisterClassExW(lpwcx: *const WNDCLASSEXW) -> WORD;
    fn CreateWindowExW(
        dwExStyle: DWORD,
        lpClassName: *const u16,
        lpWindowName: *const u16,
        dwStyle: DWORD,
        X: i32,
        Y: i32,
        nWidth: i32,
        nHeight: i32,
        hWndParent: HWND,
        hMenu: usize,
        hInstance: HINSTANCE,
        lpParam: *mut c_void,
    ) -> HWND;
    fn SetWindowTextW(hWnd: HWND, lpString: *const u16) -> BOOL;
    fn DefWindowProcW(hWnd: HWND, Msg: UINT, wParam: WPARAM, lParam: LPARAM) -> LRESULT;
    fn GetMessageW(lpMsg: *mut MSG, hWnd: HWND, wMsgFilterMin: UINT, wMsgFilterMax: UINT) -> BOOL;
    fn TranslateMessage(lpMsg: *const MSG) -> BOOL;
    fn DispatchMessageW(lpMsg: *const MSG) -> LRESULT;
    fn PostQuitMessage(nExitCode: i32);
    fn PostMessageW(hWnd: HWND, Msg: UINT, wParam: WPARAM, lParam: LPARAM) -> BOOL;
    fn InvalidateRect(hWnd: HWND, lpRect: *const RECT, bErase: BOOL) -> BOOL;
    fn BeginPaint(hWnd: HWND, lpPaint: *mut PAINTSTRUCT) -> HDC;
    fn EndPaint(hWnd: HWND, lpPaint: *const PAINTSTRUCT) -> BOOL;
    fn GetClientRect(hWnd: HWND, lpRect: *mut RECT) -> BOOL;
    fn ClientToScreen(hWnd: HWND, lpPoint: *mut POINT) -> BOOL;
    fn FillRect(hDC: HDC, lprc: *const RECT, hbr: HBRUSH) -> i32;
    fn DrawTextW(
        hdc: HDC,
        lpchText: *const u16,
        cchText: i32,
        lprc: *mut RECT,
        format: UINT,
    ) -> i32;
    fn LoadCursorW(hInstance: HINSTANCE, lpCursorName: *const u16) -> HCURSOR;
    fn LoadIconW(hInstance: HINSTANCE, lpIconName: *const u16) -> HICON;
    fn SetCursor(hCursor: HCURSOR) -> HCURSOR;
    fn GetKeyState(nVirtKey: i32) -> i16;
    fn SetTimer(hWnd: HWND, nIDEvent: usize, uElapse: UINT, lpTimerFunc: usize) -> usize;
    fn KillTimer(hWnd: HWND, uIDEvent: usize) -> BOOL;
    fn OpenClipboard(hWndNewOwner: HWND) -> BOOL;
    fn CloseClipboard() -> BOOL;
    fn EmptyClipboard() -> BOOL;
    fn IsClipboardFormatAvailable(format: UINT) -> BOOL;
    fn GetClipboardData(uFormat: UINT) -> HGLOBAL;
    fn SetClipboardData(uFormat: UINT, hMem: HGLOBAL) -> HGLOBAL;
    fn RegisterClipboardFormatW(lpszFormat: *const u16) -> UINT;
    fn GetSystemMetrics(nIndex: i32) -> i32;
    fn CreatePopupMenu() -> HMENU;
    fn AppendMenuW(hMenu: HMENU, uFlags: UINT, uIDNewItem: usize, lpNewItem: *const u16) -> BOOL;
    fn TrackPopupMenu(
        hMenu: HMENU,
        uFlags: UINT,
        x: i32,
        y: i32,
        nReserved: i32,
        hWnd: HWND,
        prcRect: *const RECT,
    ) -> i32;
    fn DestroyMenu(hMenu: HMENU) -> BOOL;
}

#[link(name = "gdi32")]
extern "system" {
    fn CreateCompatibleDC(hdc: HDC) -> HDC;
    fn DeleteDC(hdc: HDC) -> BOOL;
    fn CreateCompatibleBitmap(hdc: HDC, cx: i32, cy: i32) -> HBITMAP;
    fn SelectObject(hdc: HDC, h: HGDIOBJ) -> HGDIOBJ;
    fn DeleteObject(ho: HGDIOBJ) -> BOOL;
    fn BitBlt(
        hdc: HDC,
        x: i32,
        y: i32,
        cx: i32,
        cy: i32,
        hdcSrc: HDC,
        x1: i32,
        y1: i32,
        rop: DWORD,
    ) -> BOOL;
    fn StretchDIBits(
        hdc: HDC,
        xDest: i32,
        yDest: i32,
        DestWidth: i32,
        DestHeight: i32,
        xSrc: i32,
        ySrc: i32,
        SrcWidth: i32,
        SrcHeight: i32,
        lpBits: *const c_void,
        lpbmi: *const BITMAPINFO,
        iUsage: UINT,
        rop: DWORD,
    ) -> i32;
    fn CreateSolidBrush(color: COLORREF) -> HBRUSH;
    fn CreatePen(iStyle: i32, cWidth: i32, color: COLORREF) -> HPEN;
    fn RoundRect(
        hdc: HDC,
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
        width: i32,
        height: i32,
    ) -> BOOL;
    fn SetBkMode(hdc: HDC, mode: i32) -> i32;
    fn SetTextColor(hdc: HDC, color: COLORREF) -> COLORREF;
    fn CreateFontW(
        cHeight: i32,
        cWidth: i32,
        cEscapement: i32,
        cOrientation: i32,
        cWeight: i32,
        bItalic: DWORD,
        bUnderline: DWORD,
        bStrikeOut: DWORD,
        iCharSet: DWORD,
        iOutPrecision: DWORD,
        iClipPrecision: DWORD,
        iQuality: DWORD,
        iPitchAndFamily: DWORD,
        pszFaceName: *const u16,
    ) -> HFONT;
    fn GetStockObject(i: i32) -> HGDIOBJ;
}

#[link(name = "shell32")]
extern "system" {
    fn DragAcceptFiles(hWnd: HWND, fAccept: BOOL);
    fn DragQueryFileW(hDrop: HDROP, iFile: UINT, lpszFile: *mut u16, cch: UINT) -> UINT;
    fn DragFinish(hDrop: HDROP);
}

#[link(name = "comdlg32")]
extern "system" {
    fn GetSaveFileNameW(lpofn: *mut OPENFILENAMEW) -> BOOL;
    fn GetOpenFileNameW(lpofn: *mut OPENFILENAMEW) -> BOOL;
}

#[link(name = "dwmapi")]
extern "system" {
    fn DwmSetWindowAttribute(
        hwnd: HWND,
        dwAttribute: DWORD,
        pvAttribute: *const c_void,
        cbAttribute: DWORD,
    ) -> i32;
}

const fn rgb(r: u8, g: u8, b: u8) -> COLORREF {
    (r as u32) | ((g as u32) << 8) | ((b as u32) << 16)
}

fn wide_null(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

struct WorkerJob {
    job_id: u64,
    model_idx: usize,
    img: DynamicImage,
    source_name: String,
    cancel_flag: Arc<AtomicBool>,
}

enum WorkerEvent {
    DownloadProgress {
        job_id: u64,
        model_idx: usize,
        downloaded_mb: f32,
        total_mb: f32,
    },
    ModelCompiling {
        job_id: u64,
        model_idx: usize,
    },
    RunningInference {
        job_id: u64,
        model_idx: usize,
    },
    JobFinished {
        job_id: u64,
        model_idx: usize,
        result: Result<(RgbaImage, f32), String>,
        source_name: String,
    },
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum HoverTarget {
    None,
    StartStopBtn,
    CopyBtn,
    SaveBtn,
    CanvasEmpty,
    ModelDropdown,
    ModelPrev,
    ModelNext,
    LangBtn,
}

struct AppState {
    hwnd: HWND,
    lang: Lang,
    selected_model_idx: usize,
    worker_tx: Sender<WorkerJob>,
    worker_rx: Receiver<WorkerEvent>,
    processing: bool,
    active_cancel: Option<Arc<AtomicBool>>,
    busy_banner_en: String,
    busy_banner_ru: String,
    current_job_id: u64,
    spinner_step: usize,
    status_en: String,
    status_ru: String,
    source_name: String,
    original_dyn: Option<DynamicImage>,
    original_img: Option<RgbaImage>,
    result_img: Option<RgbaImage>,
    preview_cache: Option<(u32, u32, Vec<u8>)>,
    hover: HoverTarget,
    btn_start_rect: RECT,
    btn_copy_rect: RECT,
    btn_save_rect: RECT,
    btn_model_prev_rect: RECT,
    btn_model_drop_rect: RECT,
    btn_model_next_rect: RECT,
    btn_lang_rect: RECT,
    canvas_rect: RECT,
}

static mut APP_STATE: *mut Mutex<AppState> = null_mut();

impl AppState {
    fn current_spec(&self) -> ModelSpec {
        MODELS[self.selected_model_idx]
    }

    fn status_text(&self) -> &str {
        match self.lang {
            Lang::En => &self.status_en,
            Lang::Ru => &self.status_ru,
        }
    }

    fn banner_text(&self) -> &str {
        match self.lang {
            Lang::En => &self.busy_banner_en,
            Lang::Ru => &self.busy_banner_ru,
        }
    }

    fn set_status(&mut self, en: impl Into<String>, ru: impl Into<String>) {
        self.status_en = en.into();
        self.status_ru = ru.into();
    }

    fn set_banner(&mut self, en: impl Into<String>, ru: impl Into<String>) {
        self.busy_banner_en = en.into();
        self.busy_banner_ru = ru.into();
    }

    fn invalidate_preview(&mut self) {
        self.preview_cache = None;
    }

    fn load_image_for_staging(&mut self, img: DynamicImage, source_name: String) {
        // Cancel any ongoing processing if user drops a new image
        self.stop_processing(false);

        let (w, h) = (img.width(), img.height());
        self.original_dyn = Some(img.clone());
        self.original_img = Some(img.to_rgba8());
        self.result_img = None;
        self.source_name = source_name.clone();
        self.invalidate_preview();

        self.set_status(
            format!(
                "🖼 Loaded '{}' ({}×{}) • Click '✂ Cut Out' to start",
                source_name, w, h
            ),
            format!(
                "🖼 Загружено «{}» ({}×{}) • Нажмите «✂ Вырезать» для запуска",
                source_name, w, h
            ),
        );

        unsafe {
            InvalidateRect(self.hwnd, null(), 0);
        }
    }

    fn switch_model(&mut self, new_idx: usize) {
        if new_idx >= MODELS.len() || new_idx == self.selected_model_idx {
            return;
        }
        self.stop_processing(false);
        self.selected_model_idx = new_idx;
        let spec = self.current_spec();

        if self.original_img.is_some() {
            self.set_status(
                format!(
                    "Selected {} • Click '✂ Cut Out' to process image",
                    spec.title_en
                ),
                format!(
                    "Выбрана {} • Нажмите «✂ Вырезать» для обработки",
                    spec.title_ru
                ),
            );
        } else {
            self.set_status(
                format!(
                    "Selected {} • Drag & Drop image or press Ctrl+V",
                    spec.title_en
                ),
                format!(
                    "Выбрана {} • Перетащите картинку или нажмите Ctrl+V",
                    spec.title_ru
                ),
            );
        }

        unsafe {
            InvalidateRect(self.hwnd, null(), 0);
        }
    }

    fn start_processing(&mut self) {
        let Some(img) = self.original_dyn.clone() else {
            return;
        };
        if self.processing {
            return;
        }

        if let Some(prev_cancel) = self.active_cancel.take() {
            prev_cancel.store(true, Ordering::Relaxed);
        }

        self.current_job_id += 1;
        let job_id = self.current_job_id;
        let spec = self.current_spec();
        let source_name = self.source_name.clone();
        let cancel_flag = Arc::new(AtomicBool::new(false));
        self.active_cancel = Some(cancel_flag.clone());

        self.result_img = None;
        self.processing = true;
        self.invalidate_preview();

        if spec.is_downloaded() {
            self.set_banner(
                format!("Removing background ({})...", spec.id),
                format!("Вырезаем фон ({})...", spec.id),
            );
            self.set_status(
                format!("⏳ Cutting out '{}' with {}...", source_name, spec.title_en),
                format!("⏳ Вырезаем «{}» моделью {}...", source_name, spec.title_ru),
            );
        } else {
            self.set_banner(
                format!("Downloading {} (~{} MB)...", spec.id, spec.size_mb),
                format!("Скачивание {} (~{} МБ)...", spec.id, spec.size_mb),
            );
            self.set_status(
                format!("⬇ Downloading {} (~{} MB)...", spec.title_en, spec.size_mb),
                format!("⬇ Скачивание {} (~{} МБ)...", spec.title_ru, spec.size_mb),
            );
        }

        unsafe {
            SetTimer(self.hwnd, TIMER_SPINNER_ID, 80, 0);
            InvalidateRect(self.hwnd, null(), 0);
        }

        let _ = self.worker_tx.send(WorkerJob {
            job_id,
            model_idx: self.selected_model_idx,
            img,
            source_name,
            cancel_flag,
        });
    }

    fn stop_processing(&mut self, update_status: bool) {
        if let Some(cancel) = self.active_cancel.take() {
            cancel.store(true, Ordering::Relaxed);
        }
        if self.processing {
            self.processing = false;
            self.current_job_id += 1;
            self.invalidate_preview();
            unsafe {
                KillTimer(self.hwnd, TIMER_SPINNER_ID);
            }
            if update_status {
                self.set_status(
                    "⏹ Stopped by user • Click '✂ Cut Out' to try again",
                    "⏹ Остановлено • Нажмите «✂ Вырезать», чтобы запустить снова",
                );
            }
            unsafe {
                InvalidateRect(self.hwnd, null(), 0);
            }
        }
    }
}

fn build_checkerboard_preview(
    img: &RgbaImage,
    dst_w: u32,
    dst_h: u32,
    dim_overlay: bool,
) -> Vec<u8> {
    let (src_w, src_h) = img.dimensions();
    let mut buf = vec![0u8; (dst_w * dst_h * 4) as usize];
    let cell = 12u32;

    for y in 0..dst_h {
        let sy = ((y as u64 * src_h as u64) / dst_h.max(1) as u64).min((src_h - 1) as u64) as u32;
        for x in 0..dst_w {
            let sx =
                ((x as u64 * src_w as u64) / dst_w.max(1) as u64).min((src_w - 1) as u64) as u32;
            let checker_dark = ((x / cell) + (y / cell)) % 2 == 1;
            let bg: u32 = if checker_dark { 36 } else { 52 };

            let p = img.get_pixel(sx, sy);
            let a = p[3] as u32;
            let inv_a = 255 - a;

            let mut r = (p[0] as u32 * a + bg * inv_a + 127) / 255;
            let mut g = (p[1] as u32 * a + bg * inv_a + 127) / 255;
            let mut b = (p[2] as u32 * a + bg * inv_a + 127) / 255;

            if dim_overlay {
                r = (r * 5) / 10;
                g = (g * 5) / 10;
                b = (b * 5) / 10;
            }

            let idx = ((y * dst_w + x) * 4) as usize;
            buf[idx] = b as u8;
            buf[idx + 1] = g as u8;
            buf[idx + 2] = r as u8;
            buf[idx + 3] = 255;
        }
    }
    buf
}

fn copy_rgba_to_clipboard(hwnd: HWND, img: &RgbaImage) -> Result<(), (String, String)> {
    let mut png_bytes = Vec::new();
    {
        let encoder = PngEncoder::new(&mut png_bytes);
        encoder
            .write_image(
                img.as_raw(),
                img.width(),
                img.height(),
                ColorType::Rgba8.into(),
            )
            .map_err(|e| {
                (
                    format!("PNG encoding error: {e}"),
                    format!("Ошибка кодирования PNG: {e}"),
                )
            })?;
    }

    unsafe {
        if OpenClipboard(hwnd) == 0 {
            return Err((
                "Failed to open Windows clipboard".to_string(),
                "Не удалось открыть буфер обмена Windows".to_string(),
            ));
        }
        EmptyClipboard();

        let png_fmt_name = wide_null("PNG");
        let cf_png = RegisterClipboardFormatW(png_fmt_name.as_ptr());
        if cf_png != 0 {
            let hmem = GlobalAlloc(GMEM_MOVEABLE, png_bytes.len());
            if hmem != 0 {
                let dst = GlobalLock(hmem);
                if !dst.is_null() {
                    std::ptr::copy_nonoverlapping(png_bytes.as_ptr(), dst, png_bytes.len());
                    GlobalUnlock(hmem);
                    if SetClipboardData(cf_png, hmem) == 0 {
                        GlobalFree(hmem);
                    }
                } else {
                    GlobalFree(hmem);
                }
            }
        }

        let (w, h) = img.dimensions();
        let pixel_bytes_len = (w as usize) * (h as usize) * 4;
        let dib_total = size_of::<BITMAPINFOHEADER>() + pixel_bytes_len;
        let hdib = GlobalAlloc(GMEM_MOVEABLE, dib_total);
        if hdib != 0 {
            let dst = GlobalLock(hdib);
            if !dst.is_null() {
                let header = BITMAPINFOHEADER {
                    biSize: size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: w as i32,
                    biHeight: h as i32,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB,
                    biSizeImage: pixel_bytes_len as u32,
                    biXPelsPerMeter: 0,
                    biYPelsPerMeter: 0,
                    biClrUsed: 0,
                    biClrImportant: 0,
                };
                std::ptr::copy_nonoverlapping(
                    &header as *const _ as *const u8,
                    dst,
                    size_of::<BITMAPINFOHEADER>(),
                );
                let pixels_dst = dst.add(size_of::<BITMAPINFOHEADER>());
                for y in 0..h {
                    let src_y = h - 1 - y;
                    for x in 0..w {
                        let p = img.get_pixel(x, src_y);
                        let off = ((y * w + x) * 4) as usize;
                        *pixels_dst.add(off) = p[2];
                        *pixels_dst.add(off + 1) = p[1];
                        *pixels_dst.add(off + 2) = p[0];
                        *pixels_dst.add(off + 3) = p[3];
                    }
                }
                GlobalUnlock(hdib);
                if SetClipboardData(CF_DIB, hdib) == 0 {
                    GlobalFree(hdib);
                }
            } else {
                GlobalFree(hdib);
            }
        }

        CloseClipboard();
    }

    Ok(())
}

fn read_image_from_clipboard(hwnd: HWND) -> Result<(DynamicImage, String), (String, String)> {
    unsafe {
        if OpenClipboard(hwnd) == 0 {
            return Err((
                "Could not open clipboard".to_string(),
                "Не удалось открыть буфер обмена".to_string(),
            ));
        }

        let result = (|| {
            if IsClipboardFormatAvailable(CF_HDROP) != 0 {
                let hdrop = GetClipboardData(CF_HDROP);
                if hdrop != 0 {
                    if let Some(path) = query_dropped_file(hdrop) {
                        let img = image::open(&path).map_err(|e| {
                            (
                                format!("Failed to open {}: {e}", path.display()),
                                format!("Не удалось открыть {}: {e}", path.display()),
                            )
                        })?;
                        let name = path
                            .file_name()
                            .and_then(|s| s.to_str())
                            .unwrap_or("clipboard")
                            .to_string();
                        return Ok((img, name));
                    }
                }
            }

            let png_fmt_name = wide_null("PNG");
            let cf_png = RegisterClipboardFormatW(png_fmt_name.as_ptr());
            if cf_png != 0 && IsClipboardFormatAvailable(cf_png) != 0 {
                let hmem = GetClipboardData(cf_png);
                if hmem != 0 {
                    let ptr = GlobalLock(hmem);
                    let len = GlobalSize(hmem);
                    if !ptr.is_null() && len > 0 {
                        let slice = std::slice::from_raw_parts(ptr, len);
                        let res = image::load_from_memory(slice);
                        GlobalUnlock(hmem);
                        if let Ok(img) = res {
                            return Ok((img, "clipboard.png".to_string()));
                        }
                    }
                }
            }

            if IsClipboardFormatAvailable(CF_DIB) != 0 {
                let hmem = GetClipboardData(CF_DIB);
                if hmem != 0 {
                    let ptr = GlobalLock(hmem);
                    let len = GlobalSize(hmem);
                    if !ptr.is_null() && len >= size_of::<BITMAPINFOHEADER>() {
                        let slice = std::slice::from_raw_parts(ptr, len);
                        let res = decode_dib_slice(slice);
                        GlobalUnlock(hmem);
                        if let Some(img) = res {
                            return Ok((
                                DynamicImage::ImageRgba8(img),
                                "clipboard.png".to_string(),
                            ));
                        }
                    }
                }
            }

            Err((
                "Clipboard does not contain an image".to_string(),
                "В буфере обмена нет картинки".to_string(),
            ))
        })();

        CloseClipboard();
        result
    }
}

fn decode_dib_slice(data: &[u8]) -> Option<RgbaImage> {
    if data.len() < size_of::<BITMAPINFOHEADER>() {
        return None;
    }
    let header: BITMAPINFOHEADER =
        unsafe { std::ptr::read_unaligned(data.as_ptr() as *const BITMAPINFOHEADER) };
    let width = header.biWidth;
    let height_raw = header.biHeight;
    if width <= 0 || height_raw == 0 {
        return None;
    }
    let top_down = height_raw < 0;
    let height = height_raw.unsigned_abs();
    let bpp = header.biBitCount;
    if bpp != 24 && bpp != 32 {
        return None;
    }

    let mut offset = header.biSize as usize;
    if header.biCompression == BI_BITFIELDS && header.biSize == 40 {
        offset += 12;
    }
    if offset >= data.len() {
        return None;
    }
    let pixels = &data[offset..];
    let stride = (((width as usize * bpp as usize) + 31) / 32) * 4;
    if pixels.len() < stride * (height as usize) {
        return None;
    }

    let mut img = RgbaImage::new(width as u32, height);
    let mut all_alpha_zero = true;

    for y in 0..height {
        let src_row = if top_down { y } else { height - 1 - y } as usize;
        let row_slice = &pixels[src_row * stride..];
        for x in 0..(width as u32) {
            let px = (x as usize) * (bpp as usize / 8);
            let b = row_slice[px];
            let g = row_slice[px + 1];
            let r = row_slice[px + 2];
            let a = if bpp == 32 { row_slice[px + 3] } else { 255 };
            if a != 0 {
                all_alpha_zero = false;
            }
            img.put_pixel(x, y, image::Rgba([r, g, b, a]));
        }
    }

    if bpp == 32 && all_alpha_zero {
        for p in img.pixels_mut() {
            p[3] = 255;
        }
    }

    Some(img)
}

unsafe fn query_dropped_file(hdrop: HDROP) -> Option<PathBuf> {
    let len = DragQueryFileW(hdrop, 0, null_mut(), 0);
    if len == 0 {
        return None;
    }
    let mut buf = vec![0u16; (len + 1) as usize];
    DragQueryFileW(hdrop, 0, buf.as_mut_ptr(), len + 1);
    if let Some(pos) = buf.iter().position(|&c| c == 0) {
        buf.truncate(pos);
    }
    Some(PathBuf::from(String::from_utf16_lossy(&buf)))
}

fn open_file_dialog(hwnd: HWND, lang: Lang) -> Option<PathBuf> {
    unsafe {
        let filter_str = match lang {
            Lang::En => "Images (*.png;*.jpg;*.jpeg;*.webp)\0*.png;*.jpg;*.jpeg;*.webp\0All Files (*.*)\0*.*\0\0",
            Lang::Ru => "Изображения (*.png;*.jpg;*.jpeg;*.webp)\0*.png;*.jpg;*.jpeg;*.webp\0Все файлы (*.*)\0*.*\0\0",
        };
        let filter: Vec<u16> = filter_str.encode_utf16().collect();
        let title = match lang {
            Lang::En => wide_null("Select an image to remove background"),
            Lang::Ru => wide_null("Выберите картинку для удаления фона"),
        };
        let mut file_buf = vec![0u16; 1024];

        let mut ofn: OPENFILENAMEW = zeroed();
        ofn.lStructSize = size_of::<OPENFILENAMEW>() as u32;
        ofn.hwndOwner = hwnd;
        ofn.lpstrFilter = filter.as_ptr();
        ofn.nFilterIndex = 1;
        ofn.lpstrFile = file_buf.as_mut_ptr();
        ofn.nMaxFile = file_buf.len() as u32;
        ofn.lpstrTitle = title.as_ptr();
        ofn.Flags = OFN_PATHMUSTEXIST | OFN_FILEMUSTEXIST | OFN_NOCHANGEDIR;

        if GetOpenFileNameW(&mut ofn) != 0 {
            if let Some(pos) = file_buf.iter().position(|&c| c == 0) {
                file_buf.truncate(pos);
            }
            Some(PathBuf::from(String::from_utf16_lossy(&file_buf)))
        } else {
            None
        }
    }
}

fn save_file_dialog(hwnd: HWND, default_stem: &str, model_id: &str, lang: Lang) -> Option<PathBuf> {
    unsafe {
        let filter_str = match lang {
            Lang::En => "Transparent PNG (*.png)\0*.png\0All Files (*.*)\0*.*\0\0",
            Lang::Ru => "PNG с прозрачным фоном (*.png)\0*.png\0Все файлы (*.*)\0*.*\0\0",
        };
        let filter: Vec<u16> = filter_str.encode_utf16().collect();
        let title = match lang {
            Lang::En => wide_null("Save transparent PNG"),
            Lang::Ru => wide_null("Сохранить картинку без фона"),
        };
        let def_ext = wide_null("png");

        let stem = PathBuf::from(default_stem)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("image")
            .to_string();
        let default_name = format!("{}_{}_nobg.png", stem, model_id);

        let mut file_buf = vec![0u16; 1024];
        for (i, ch) in default_name.encode_utf16().enumerate() {
            if i + 1 < file_buf.len() {
                file_buf[i] = ch;
            }
        }

        let mut ofn: OPENFILENAMEW = zeroed();
        ofn.lStructSize = size_of::<OPENFILENAMEW>() as u32;
        ofn.hwndOwner = hwnd;
        ofn.lpstrFilter = filter.as_ptr();
        ofn.nFilterIndex = 1;
        ofn.lpstrFile = file_buf.as_mut_ptr();
        ofn.nMaxFile = file_buf.len() as u32;
        ofn.lpstrTitle = title.as_ptr();
        ofn.lpstrDefExt = def_ext.as_ptr();
        ofn.Flags = OFN_OVERWRITEPROMPT | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR;

        if GetSaveFileNameW(&mut ofn) != 0 {
            if let Some(pos) = file_buf.iter().position(|&c| c == 0) {
                file_buf.truncate(pos);
            }
            Some(PathBuf::from(String::from_utf16_lossy(&file_buf)))
        } else {
            None
        }
    }
}

unsafe fn measure_text_width(hdc: HDC, font: HFONT, text: &str) -> i32 {
    let old_font = SelectObject(hdc, font);
    let mut rc = RECT {
        left: 0,
        top: 0,
        right: 2000,
        bottom: 200,
    };
    let wtext = wide_null(text);
    DrawTextW(
        hdc,
        wtext.as_ptr(),
        (wtext.len() - 1) as i32,
        &mut rc,
        DT_CALCRECT | DT_SINGLELINE | DT_NOPREFIX,
    );
    SelectObject(hdc, old_font);
    rc.right - rc.left
}

unsafe fn draw_rounded_box(
    hdc: HDC,
    rc: &RECT,
    radius: i32,
    bg_color: COLORREF,
    border_color: COLORREF,
    border_width: i32,
) {
    let brush = CreateSolidBrush(bg_color);
    let pen = CreatePen(0, border_width, border_color);
    let old_brush = SelectObject(hdc, brush);
    let old_pen = SelectObject(hdc, pen);

    RoundRect(hdc, rc.left, rc.top, rc.right, rc.bottom, radius, radius);

    SelectObject(hdc, old_brush);
    SelectObject(hdc, old_pen);
    DeleteObject(brush);
    DeleteObject(pen);
}

unsafe fn draw_rounded_button(
    hdc: HDC,
    rc: &RECT,
    text: &str,
    bg_color: COLORREF,
    border_color: COLORREF,
    text_color: COLORREF,
    font: HFONT,
) {
    draw_rounded_box(hdc, rc, 10, bg_color, border_color, 1);

    let old_font = SelectObject(hdc, font);
    SetTextColor(hdc, text_color);
    let mut text_rc = RECT {
        left: rc.left + 8,
        top: rc.top,
        right: rc.right - 8,
        bottom: rc.bottom,
    };
    let wtext = wide_null(text);
    DrawTextW(
        hdc,
        wtext.as_ptr(),
        (wtext.len() - 1) as i32,
        &mut text_rc,
        DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS,
    );
    SelectObject(hdc, old_font);
}

unsafe fn paint_window(hwnd: HWND, state: &mut AppState) {
    let mut ps: PAINTSTRUCT = zeroed();
    let hdc_win = BeginPaint(hwnd, &mut ps);

    let mut client: RECT = zeroed();
    GetClientRect(hwnd, &mut client);
    let cw = (client.right - client.left).max(1);
    let ch = (client.bottom - client.top).max(1);

    let hdc = CreateCompatibleDC(hdc_win);
    let hbm = CreateCompatibleBitmap(hdc_win, cw, ch);
    let old_bm = SelectObject(hdc, hbm);

    SetBkMode(hdc, TRANSPARENT);

    // Main window dark background (#141418)
    let bg_brush = CreateSolidBrush(rgb(20, 20, 24));
    FillRect(hdc, &client, bg_brush);
    DeleteObject(bg_brush);

    let font_face = wide_null("Segoe UI");
    let font_title = CreateFontW(
        -22, 0, 0, 0, 600, 0, 0, 0, 1, 0, 0, 5, 0, font_face.as_ptr(),
    );
    let font_sub = CreateFontW(
        -15, 0, 0, 0, 400, 0, 0, 0, 1, 0, 0, 5, 0, font_face.as_ptr(),
    );
    let font_btn = CreateFontW(
        -15, 0, 0, 0, 600, 0, 0, 0, 1, 0, 0, 5, 0, font_face.as_ptr(),
    );
    let font_desc = CreateFontW(
        -14, 0, 0, 0, 400, 0, 0, 0, 1, 0, 0, 5, 0, font_face.as_ptr(),
    );
    let font_badge = CreateFontW(
        -13, 0, 0, 0, 600, 0, 0, 0, 1, 0, 0, 5, 0, font_face.as_ptr(),
    );

    let pad = 18;
    let lang = state.lang;
    let spec = state.current_spec();

    // =========================================================================
    // 1. TOP HEADER: Model Selector + Language Switcher + Model Description Box
    // =========================================================================
    let top_row_y = 14;
    let top_row_h = 38;

    // Language button on top right
    let lang_w = 116;
    let lang_rc = RECT {
        left: cw - pad - lang_w,
        top: top_row_y,
        right: cw - pad,
        bottom: top_row_y + top_row_h,
    };
    state.btn_lang_rect = lang_rc;

    let lang_label = match lang {
        Lang::En => "🌐 EN ▸ RU",
        Lang::Ru => "🌐 RU ▸ EN",
    };
    let (lang_bg, lang_border) = if state.hover == HoverTarget::LangBtn {
        (rgb(52, 52, 64), rgb(96, 165, 250))
    } else {
        (rgb(34, 34, 42), rgb(64, 64, 76))
    };
    draw_rounded_button(
        hdc,
        &lang_rc,
        lang_label,
        lang_bg,
        lang_border,
        rgb(235, 235, 245),
        font_btn,
    );

    // "Model:" label on top left
    let model_lbl_w = 68;
    let mut model_lbl_rc = RECT {
        left: pad,
        top: top_row_y,
        right: pad + model_lbl_w,
        bottom: top_row_y + top_row_h,
    };
    let old_f = SelectObject(hdc, font_btn);
    SetTextColor(hdc, rgb(190, 190, 205));
    let model_lbl_str = match lang {
        Lang::En => wide_null("Model:"),
        Lang::Ru => wide_null("Модель:"),
    };
    DrawTextW(
        hdc,
        model_lbl_str.as_ptr(),
        (model_lbl_str.len() - 1) as i32,
        &mut model_lbl_rc,
        DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
    );
    SelectObject(hdc, old_f);

    let nav_btn_w = 38;
    let prev_rc = RECT {
        left: model_lbl_rc.right,
        top: top_row_y,
        right: model_lbl_rc.right + nav_btn_w,
        bottom: top_row_y + top_row_h,
    };
    let next_rc = RECT {
        left: lang_rc.left - 12 - nav_btn_w,
        top: top_row_y,
        right: lang_rc.left - 12,
        bottom: top_row_y + top_row_h,
    };
    let drop_rc = RECT {
        left: prev_rc.right + 6,
        top: top_row_y,
        right: next_rc.left - 6,
        bottom: top_row_y + top_row_h,
    };
    state.btn_model_prev_rect = prev_rc;
    state.btn_model_drop_rect = drop_rc;
    state.btn_model_next_rect = next_rc;

    let (prev_bg, prev_bdr) = if state.hover == HoverTarget::ModelPrev {
        (rgb(52, 52, 64), rgb(96, 165, 250))
    } else {
        (rgb(32, 32, 40), rgb(58, 58, 70))
    };
    draw_rounded_button(
        hdc,
        &prev_rc,
        "◀",
        prev_bg,
        prev_bdr,
        rgb(230, 230, 240),
        font_btn,
    );

    let (next_bg, next_bdr) = if state.hover == HoverTarget::ModelNext {
        (rgb(52, 52, 64), rgb(96, 165, 250))
    } else {
        (rgb(32, 32, 40), rgb(58, 58, 70))
    };
    draw_rounded_button(
        hdc,
        &next_rc,
        "▶",
        next_bg,
        next_bdr,
        rgb(230, 230, 240),
        font_btn,
    );

    let (drop_bg, drop_bdr) = if state.hover == HoverTarget::ModelDropdown {
        (rgb(44, 48, 64), rgb(96, 165, 250))
    } else {
        (rgb(30, 32, 42), rgb(68, 74, 94))
    };
    let installed_tag = if spec.is_downloaded() {
        match lang {
            Lang::En => "✓ Installed",
            Lang::Ru => "✓ Скачана",
        }
    } else {
        match lang {
            Lang::En => "⬇ Auto-Download",
            Lang::Ru => "⬇ Авто-загрузка",
        }
    };
    let drop_title = format!("{}   [{}]   ▼", spec.title(lang), installed_tag);
    draw_rounded_button(
        hdc,
        &drop_rc,
        &drop_title,
        drop_bg,
        drop_bdr,
        rgb(245, 245, 255),
        font_btn,
    );

    // Model Description & Recommendation Box (generous height for 2 lines + exact badge width)
    let info_top = top_row_y + top_row_h + 8;
    let info_h = 80;
    let info_rc = RECT {
        left: pad,
        top: info_top,
        right: cw - pad,
        bottom: info_top + info_h,
    };
    draw_rounded_box(hdc, &info_rc, 10, rgb(26, 27, 33), rgb(48, 50, 62), 1);

    // Measure exact pixel width of badge text with DT_CALCRECT so letters never get clipped
    let badge_text = spec.badge(lang);
    let measured_badge_w = measure_text_width(hdc, font_badge, badge_text);
    let badge_w = (measured_badge_w + 28).max(120);
    let badge_rc = RECT {
        left: info_rc.left + 12,
        top: info_rc.top + 9,
        right: (info_rc.left + 12 + badge_w).min(info_rc.right - 12),
        bottom: info_rc.top + 33,
    };
    let badge_bg = if spec.is_downloaded() {
        rgb(24, 62, 48)
    } else {
        rgb(36, 52, 86)
    };
    let badge_bdr = if spec.is_downloaded() {
        rgb(46, 130, 96)
    } else {
        rgb(68, 108, 180)
    };
    draw_rounded_button(
        hdc,
        &badge_rc,
        badge_text,
        badge_bg,
        badge_bdr,
        rgb(225, 242, 255),
        font_badge,
    );

    let counter_str = match lang {
        Lang::En => format!(
            "Model {} of {} • Click dropdown above to choose another model",
            state.selected_model_idx + 1,
            MODELS.len()
        ),
        Lang::Ru => format!(
            "Модель {} из {} • Нажмите на список выше для выбора другой модели",
            state.selected_model_idx + 1,
            MODELS.len()
        ),
    };
    let mut counter_rc = RECT {
        left: badge_rc.right + 14,
        top: info_rc.top + 9,
        right: info_rc.right - 12,
        bottom: info_rc.top + 33,
    };
    let old_f = SelectObject(hdc, font_badge);
    SetTextColor(hdc, rgb(140, 145, 165));
    let wcounter = wide_null(&counter_str);
    DrawTextW(
        hdc,
        wcounter.as_ptr(),
        (wcounter.len() - 1) as i32,
        &mut counter_rc,
        DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS,
    );

    let mut desc_rc = RECT {
        left: info_rc.left + 14,
        top: info_rc.top + 38,
        right: info_rc.right - 14,
        bottom: info_rc.bottom - 6,
    };
    SelectObject(hdc, font_desc);
    SetTextColor(hdc, rgb(212, 214, 228));
    let wdesc = wide_null(spec.desc(lang));
    DrawTextW(
        hdc,
        wdesc.as_ptr(),
        (wdesc.len() - 1) as i32,
        &mut desc_rc,
        DT_LEFT | DT_WORDBREAK | DT_NOPREFIX,
    );
    SelectObject(hdc, old_f);

    // =========================================================================
    // 2. MAIN IMAGE CANVAS AREA
    // =========================================================================
    let toolbar_h = 68;
    let canvas_rc = RECT {
        left: pad,
        top: info_rc.bottom + 10,
        right: cw - pad,
        bottom: (ch - toolbar_h - 6).max(info_rc.bottom + 100),
    };
    state.canvas_rect = canvas_rc;

    let canvas_bdr_color = if state.hover == HoverTarget::CanvasEmpty
        && state.result_img.is_none()
        && state.original_img.is_none()
    {
        rgb(79, 134, 247)
    } else {
        rgb(52, 54, 66)
    };
    draw_rounded_box(hdc, &canvas_rc, 14, rgb(28, 28, 34), canvas_bdr_color, 1);

    let active_img = state.result_img.as_ref().or(state.original_img.as_ref());
    if let Some(img) = active_img {
        let inner_pad = 12;
        let avail_w = (canvas_rc.right - canvas_rc.left - inner_pad * 2).max(16) as u32;
        let avail_h = (canvas_rc.bottom - canvas_rc.top - inner_pad * 2).max(16) as u32;
        let (iw, ih) = img.dimensions();

        let scale = (avail_w as f64 / iw.max(1) as f64).min(avail_h as f64 / ih.max(1) as f64);
        let dst_w = ((iw as f64 * scale).round() as u32).max(1);
        let dst_h = ((ih as f64 * scale).round() as u32).max(1);

        let need_rebuild = match &state.preview_cache {
            Some((cw_cache, ch_cache, _)) => *cw_cache != dst_w || *ch_cache != dst_h,
            None => true,
        };
        if need_rebuild {
            let buf = build_checkerboard_preview(img, dst_w, dst_h, state.processing);
            state.preview_cache = Some((dst_w, dst_h, buf));
        }

        if let Some((pw, ph, ref pixels)) = state.preview_cache {
            let draw_x = canvas_rc.left + (canvas_rc.right - canvas_rc.left - pw as i32) / 2;
            let draw_y = canvas_rc.top + (canvas_rc.bottom - canvas_rc.top - ph as i32) / 2;

            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: pw as i32,
                    biHeight: -(ph as i32),
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB,
                    biSizeImage: 0,
                    biXPelsPerMeter: 0,
                    biYPelsPerMeter: 0,
                    biClrUsed: 0,
                    biClrImportant: 0,
                },
                bmiColors: [0],
            };

            StretchDIBits(
                hdc,
                draw_x,
                draw_y,
                pw as i32,
                ph as i32,
                0,
                0,
                pw as i32,
                ph as i32,
                pixels.as_ptr() as *const c_void,
                &bmi,
                DIB_RGB_COLORS,
                SRCCOPY,
            );
        }

        if state.processing {
            let spinners = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let sp = spinners[state.spinner_step % spinners.len()];
            let banner_w = 450;
            let banner_h = 52;
            let bx = (canvas_rc.left + canvas_rc.right - banner_w) / 2;
            let by = (canvas_rc.top + canvas_rc.bottom - banner_h) / 2;
            let banner_rc = RECT {
                left: bx,
                top: by,
                right: bx + banner_w,
                bottom: by + banner_h,
            };
            let banner_str = format!("{}  {}", sp, state.banner_text());
            draw_rounded_button(
                hdc,
                &banner_rc,
                &banner_str,
                rgb(22, 24, 32),
                rgb(79, 134, 247),
                rgb(245, 245, 255),
                font_btn,
            );
        }
    } else {
        let cy = (canvas_rc.top + canvas_rc.bottom) / 2;
        let mut r1 = RECT {
            left: canvas_rc.left + 20,
            top: cy - 44,
            right: canvas_rc.right - 20,
            bottom: cy - 10,
        };
        let mut r2 = RECT {
            left: canvas_rc.left + 20,
            top: cy - 4,
            right: canvas_rc.right - 20,
            bottom: cy + 26,
        };
        let mut r3 = RECT {
            left: canvas_rc.left + 20,
            top: cy + 26,
            right: canvas_rc.right - 20,
            bottom: cy + 54,
        };

        let (t1_str, t2_str, t3_str) = match lang {
            Lang::En => (
                "Drag & Drop an image into this window",
                "or press Ctrl+V to paste from clipboard",
                "(click here to browse and open an image file)",
            ),
            Lang::Ru => (
                "Перетащите картинку в это окно мышкой",
                "или нажмите Ctrl+V, чтобы вставить из буфера обмена",
                "(кликните сюда, чтобы выбрать файл с диска)",
            ),
        };

        let old_f = SelectObject(hdc, font_title);
        SetTextColor(hdc, rgb(235, 235, 245));
        let t1 = wide_null(t1_str);
        DrawTextW(
            hdc,
            t1.as_ptr(),
            (t1.len() - 1) as i32,
            &mut r1,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
        );

        SelectObject(hdc, font_sub);
        SetTextColor(hdc, rgb(160, 162, 180));
        let t2 = wide_null(t2_str);
        DrawTextW(
            hdc,
            t2.as_ptr(),
            (t2.len() - 1) as i32,
            &mut r2,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
        );

        SetTextColor(hdc, rgb(115, 150, 235));
        let t3 = wide_null(t3_str);
        DrawTextW(
            hdc,
            t3.as_ptr(),
            (t3.len() - 1) as i32,
            &mut r3,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
        );
        SelectObject(hdc, old_f);
    }

    // =========================================================================
    // 3. BOTTOM TOOLBAR: Status + [Cut Out / Stop] + [Copy] + [Save]
    // =========================================================================
    let btn_h = 42;
    let btn_y = ch - 14 - btn_h;
    let gap = 10;

    let save_w = 186;
    let copy_w = 196;
    let start_w = 166;

    let save_rc = RECT {
        left: cw - pad - save_w,
        top: btn_y,
        right: cw - pad,
        bottom: btn_y + btn_h,
    };
    let copy_rc = RECT {
        left: save_rc.left - gap - copy_w,
        top: btn_y,
        right: save_rc.left - gap,
        bottom: btn_y + btn_h,
    };
    let start_rc = RECT {
        left: copy_rc.left - gap - start_w,
        top: btn_y,
        right: copy_rc.left - gap,
        bottom: btn_y + btn_h,
    };
    state.btn_start_rect = start_rc;
    state.btn_copy_rect = copy_rc;
    state.btn_save_rect = save_rc;

    let has_input = state.original_img.is_some();
    let has_result = state.result_img.is_some() && !state.processing;

    // Start ("✂ Cut Out" / "✂ Вырезать") OR Stop ("⏹ Stop" / "⏹ Остановить") button
    let (start_label, start_bg, start_border, start_fg) = if state.processing {
        let lbl = match lang {
            Lang::En => "⏹ Stop (Esc)",
            Lang::Ru => "⏹ Остановить",
        };
        if state.hover == HoverTarget::StartStopBtn {
            (lbl, rgb(220, 50, 50), rgb(248, 113, 113), rgb(255, 255, 255))
        } else {
            (lbl, rgb(185, 38, 38), rgb(225, 70, 70), rgb(255, 255, 255))
        }
    } else {
        let lbl = match lang {
            Lang::En => "✂ Cut Out",
            Lang::Ru => "✂ Вырезать",
        };
        if !has_input {
            (lbl, rgb(32, 32, 38), rgb(50, 50, 58), rgb(105, 105, 118))
        } else if state.hover == HoverTarget::StartStopBtn {
            (lbl, rgb(22, 163, 74), rgb(74, 222, 128), rgb(255, 255, 255))
        } else {
            (lbl, rgb(21, 128, 61), rgb(34, 197, 94), rgb(255, 255, 255))
        }
    };

    let (copy_bg, copy_border, copy_fg) = if !has_result {
        (rgb(32, 32, 38), rgb(50, 50, 58), rgb(105, 105, 118))
    } else if state.hover == HoverTarget::CopyBtn {
        (rgb(59, 130, 246), rgb(96, 165, 250), rgb(255, 255, 255))
    } else {
        (rgb(37, 99, 235), rgb(59, 130, 246), rgb(255, 255, 255))
    };

    let (save_bg, save_border, save_fg) = if !has_result {
        (rgb(32, 32, 38), rgb(50, 50, 58), rgb(105, 105, 118))
    } else if state.hover == HoverTarget::SaveBtn {
        (rgb(63, 63, 74), rgb(100, 100, 116), rgb(255, 255, 255))
    } else {
        (rgb(45, 45, 54), rgb(75, 75, 88), rgb(240, 240, 248))
    };

    let (copy_label, save_label) = match lang {
        Lang::En => ("Copy (Ctrl+C)", "Save PNG (Ctrl+S)"),
        Lang::Ru => ("Скопировать (Ctrl+C)", "Сохранить (Ctrl+S)"),
    };

    draw_rounded_button(
        hdc,
        &start_rc,
        start_label,
        start_bg,
        start_border,
        start_fg,
        font_btn,
    );
    draw_rounded_button(
        hdc,
        &copy_rc,
        copy_label,
        copy_bg,
        copy_border,
        copy_fg,
        font_btn,
    );
    draw_rounded_button(
        hdc,
        &save_rc,
        save_label,
        save_bg,
        save_border,
        save_fg,
        font_btn,
    );

    let mut status_rc = RECT {
        left: pad,
        top: btn_y,
        right: (start_rc.left - 12).max(pad + 60),
        bottom: btn_y + btn_h,
    };
    let old_font = SelectObject(hdc, font_sub);
    SetTextColor(hdc, rgb(188, 190, 206));
    let status_w = wide_null(state.status_text());
    DrawTextW(
        hdc,
        status_w.as_ptr(),
        (status_w.len() - 1) as i32,
        &mut status_rc,
        DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS,
    );
    SelectObject(hdc, old_font);

    BitBlt(hdc_win, 0, 0, cw, ch, hdc, 0, 0, SRCCOPY);

    DeleteObject(font_title);
    DeleteObject(font_sub);
    DeleteObject(font_btn);
    DeleteObject(font_desc);
    DeleteObject(font_badge);
    SelectObject(hdc, old_bm);
    DeleteObject(hbm);
    DeleteDC(hdc);

    EndPaint(hwnd, &ps);
}

unsafe fn show_model_popup_menu(hwnd: HWND) {
    let (current_idx, lang, drop_rc) = if let Ok(st) = (*APP_STATE).lock() {
        (st.selected_model_idx, st.lang, st.btn_model_drop_rect)
    } else {
        return;
    };

    let hmenu = CreatePopupMenu();
    if hmenu == 0 {
        return;
    }

    for (idx, m) in MODELS.iter().enumerate() {
        let status_tag = if m.is_downloaded() {
            match lang {
                Lang::En => "✓ Ready".to_string(),
                Lang::Ru => "✓ Скачана".to_string(),
            }
        } else {
            match lang {
                Lang::En => format!("⬇ {} MB", m.size_mb),
                Lang::Ru => format!("⬇ {} МБ", m.size_mb),
            }
        };
        let label = format!("{}   [{} — {}]", m.title(lang), status_tag, m.badge(lang));
        let wlabel = wide_null(&label);
        let flags = MF_STRING | if idx == current_idx { MF_CHECKED } else { 0 };
        AppendMenuW(hmenu, flags, idx + 1, wlabel.as_ptr());
    }

    let mut pt = POINT {
        x: drop_rc.left,
        y: drop_rc.bottom + 2,
    };
    ClientToScreen(hwnd, &mut pt);

    let cmd = TrackPopupMenu(
        hmenu,
        TPM_LEFTALIGN | TPM_TOPALIGN | TPM_RETURNCMD | TPM_NONOTIFY,
        pt.x,
        pt.y,
        0,
        hwnd,
        null(),
    );
    DestroyMenu(hmenu);

    if cmd > 0 {
        let chosen_idx = (cmd as usize) - 1;
        if let Ok(mut st) = (*APP_STATE).lock() {
            st.switch_model(chosen_idx);
        }
    }
}

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if APP_STATE.is_null() {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }

    match msg {
        WM_CREATE => {
            DragAcceptFiles(hwnd, 1);
            let dark_mode: i32 = 1;
            let _ = DwmSetWindowAttribute(hwnd, 20, &dark_mode as *const _ as *const c_void, 4);
            0
        }
        WM_ERASEBKGND => 1,
        WM_GETMINMAXINFO => {
            let mmi = lparam as *mut MINMAXINFO;
            if !mmi.is_null() {
                (*mmi).ptMinTrackSize.x = 820;
                (*mmi).ptMinTrackSize.y = 540;
            }
            0
        }
        WM_SIZE => {
            if let Ok(mut st) = (*APP_STATE).lock() {
                st.invalidate_preview();
            }
            InvalidateRect(hwnd, null(), 0);
            0
        }
        WM_TIMER => {
            if wparam == TIMER_SPINNER_ID {
                if let Ok(mut st) = (*APP_STATE).lock() {
                    if st.processing {
                        st.spinner_step = st.spinner_step.wrapping_add(1);
                        InvalidateRect(hwnd, null(), 0);
                    } else {
                        KillTimer(hwnd, TIMER_SPINNER_ID);
                    }
                }
            }
            0
        }
        WM_APP_WORKER_MSG => {
            if let Ok(mut st) = (*APP_STATE).lock() {
                while let Ok(ev) = st.worker_rx.try_recv() {
                    match ev {
                        WorkerEvent::DownloadProgress {
                            job_id,
                            model_idx,
                            downloaded_mb,
                            total_mb,
                        } => {
                            if st.processing
                                && job_id == st.current_job_id
                                && model_idx == st.selected_model_idx
                            {
                                let spec = MODELS[model_idx];
                                let pct = ((downloaded_mb / total_mb.max(0.1)) * 100.0).min(100.0);
                                st.set_banner(
                                    format!(
                                        "Downloading {}: {:.1} / {:.1} MB ({:.0}%)",
                                        spec.id, downloaded_mb, total_mb, pct
                                    ),
                                    format!(
                                        "Скачивание {}: {:.1} / {:.1} МБ ({:.0}%)",
                                        spec.id, downloaded_mb, total_mb, pct
                                    ),
                                );
                                st.set_status(
                                    format!(
                                        "⬇ Downloading {}: {:.1} / {:.1} MB ({:.0}%)",
                                        spec.title_en, downloaded_mb, total_mb, pct
                                    ),
                                    format!(
                                        "⬇ Скачивание {}: {:.1} / {:.1} МБ ({:.0}%)",
                                        spec.title_ru, downloaded_mb, total_mb, pct
                                    ),
                                );
                            }
                        }
                        WorkerEvent::ModelCompiling { job_id, model_idx } => {
                            if st.processing
                                && job_id == st.current_job_id
                                && model_idx == st.selected_model_idx
                            {
                                let spec = MODELS[model_idx];
                                st.set_banner(
                                    format!("Preparing {} neural network...", spec.id),
                                    format!("Подготовка нейросети {}...", spec.id),
                                );
                                st.set_status(
                                    format!("⚙ Preparing model {}...", spec.title_en),
                                    format!("⚙ Подготовка модели {}...", spec.title_ru),
                                );
                            }
                        }
                        WorkerEvent::RunningInference { job_id, model_idx } => {
                            if st.processing
                                && job_id == st.current_job_id
                                && model_idx == st.selected_model_idx
                            {
                                let spec = MODELS[model_idx];
                                st.set_banner(
                                    format!("Removing background ({})...", spec.id),
                                    format!("Вырезаем фон ({})...", spec.id),
                                );
                                st.set_status(
                                    format!("⏳ Cutting out background ({})...", spec.title_en),
                                    format!("⏳ Вырезаем фон ({})...", spec.title_ru),
                                );
                            }
                        }
                        WorkerEvent::JobFinished {
                            job_id,
                            model_idx,
                            result,
                            source_name,
                        } => {
                            if st.processing
                                && job_id == st.current_job_id
                                && model_idx == st.selected_model_idx
                            {
                                st.processing = false;
                                st.active_cancel = None;
                                KillTimer(hwnd, TIMER_SPINNER_ID);
                                let spec = MODELS[model_idx];
                                match result {
                                    Ok((cutout, secs)) => {
                                        let (w, h) = cutout.dimensions();
                                        st.result_img = Some(cutout);
                                        st.invalidate_preview();
                                        st.set_status(
                                            format!(
                                                "✅ Done ({}): {} ({}×{}, {:.2}s)",
                                                spec.id, source_name, w, h, secs
                                            ),
                                            format!(
                                                "✅ Готово ({}): {} ({}×{}, за {:.2} сек)",
                                                spec.id, source_name, w, h, secs
                                            ),
                                        );
                                    }
                                    Err(e) => {
                                        st.invalidate_preview();
                                        if e == "Cancelled" {
                                            st.set_status(
                                                "⏹ Stopped by user • Click '✂ Cut Out' to start",
                                                "⏹ Остановлено • Нажмите «✂ Вырезать» для запуска",
                                            );
                                        } else {
                                            st.set_status(
                                                format!("❌ Error: {e}"),
                                                format!("❌ Ошибка: {e}"),
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            InvalidateRect(hwnd, null(), 0);
            0
        }
        WM_DROPFILES => {
            let hdrop = wparam as HDROP;
            if let Some(path) = query_dropped_file(hdrop) {
                match image::open(&path) {
                    Ok(img) => {
                        let name = path
                            .file_name()
                            .and_then(|s| s.to_str())
                            .unwrap_or("image.png")
                            .to_string();
                        if let Ok(mut st) = (*APP_STATE).lock() {
                            st.load_image_for_staging(img, name);
                        }
                    }
                    Err(e) => {
                        if let Ok(mut st) = (*APP_STATE).lock() {
                            st.set_status(
                                format!("❌ Failed to open file: {e}"),
                                format!("❌ Не удалось открыть файл: {e}"),
                            );
                        }
                        InvalidateRect(hwnd, null(), 0);
                    }
                }
            }
            DragFinish(hdrop);
            0
        }
        WM_KEYDOWN => {
            if wparam == VK_ESCAPE {
                if let Ok(mut st) = (*APP_STATE).lock() {
                    st.stop_processing(true);
                }
                return 0;
            }
            if wparam == VK_RETURN {
                if let Ok(mut st) = (*APP_STATE).lock() {
                    if !st.processing && st.original_img.is_some() {
                        st.start_processing();
                    }
                }
                return 0;
            }

            let ctrl_down = (GetKeyState(VK_CONTROL) as u16 & 0x8000) != 0;
            if ctrl_down {
                match wparam {
                    VK_V => {
                        match read_image_from_clipboard(hwnd) {
                            Ok((img, name)) => {
                                if let Ok(mut st) = (*APP_STATE).lock() {
                                    st.load_image_for_staging(img, name);
                                }
                            }
                            Err((en, ru)) => {
                                if let Ok(mut st) = (*APP_STATE).lock() {
                                    st.set_status(format!("⚠ {en}"), format!("⚠ {ru}"));
                                }
                                InvalidateRect(hwnd, null(), 0);
                            }
                        }
                        return 0;
                    }
                    VK_C => {
                        trigger_copy_action(hwnd);
                        return 0;
                    }
                    VK_S => {
                        trigger_save_action(hwnd);
                        return 0;
                    }
                    VK_O => {
                        trigger_open_action(hwnd);
                        return 0;
                    }
                    _ => {}
                }
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_MOUSEMOVE => {
            let x = (lparam & 0xFFFF) as i16 as i32;
            let y = ((lparam >> 16) & 0xFFFF) as i16 as i32;
            if let Ok(mut st) = (*APP_STATE).lock() {
                let has_input = st.original_img.is_some();
                let has_res = st.result_img.is_some() && !st.processing;
                let new_hover = if st.btn_lang_rect.contains(x, y) {
                    HoverTarget::LangBtn
                } else if st.btn_model_drop_rect.contains(x, y) {
                    HoverTarget::ModelDropdown
                } else if st.btn_model_prev_rect.contains(x, y) {
                    HoverTarget::ModelPrev
                } else if st.btn_model_next_rect.contains(x, y) {
                    HoverTarget::ModelNext
                } else if (st.processing || has_input) && st.btn_start_rect.contains(x, y) {
                    HoverTarget::StartStopBtn
                } else if has_res && st.btn_copy_rect.contains(x, y) {
                    HoverTarget::CopyBtn
                } else if has_res && st.btn_save_rect.contains(x, y) {
                    HoverTarget::SaveBtn
                } else if st.canvas_rect.contains(x, y)
                    && st.result_img.is_none()
                    && st.original_img.is_none()
                {
                    HoverTarget::CanvasEmpty
                } else {
                    HoverTarget::None
                };
                if new_hover != st.hover {
                    st.hover = new_hover;
                    InvalidateRect(hwnd, null(), 0);
                }
            }
            0
        }
        WM_SETCURSOR => {
            let hover = (*APP_STATE)
                .lock()
                .map(|s| s.hover)
                .unwrap_or(HoverTarget::None);
            if hover != HoverTarget::None {
                SetCursor(LoadCursorW(0, IDC_HAND));
                return 1;
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_LBUTTONUP => {
            let x = (lparam & 0xFFFF) as i16 as i32;
            let y = ((lparam >> 16) & 0xFFFF) as i16 as i32;

            let clicked = if let Ok(st) = (*APP_STATE).lock() {
                let has_input = st.original_img.is_some();
                let has_res = st.result_img.is_some() && !st.processing;
                if st.btn_lang_rect.contains(x, y) {
                    HoverTarget::LangBtn
                } else if st.btn_model_drop_rect.contains(x, y) {
                    HoverTarget::ModelDropdown
                } else if st.btn_model_prev_rect.contains(x, y) {
                    HoverTarget::ModelPrev
                } else if st.btn_model_next_rect.contains(x, y) {
                    HoverTarget::ModelNext
                } else if (st.processing || has_input) && st.btn_start_rect.contains(x, y) {
                    HoverTarget::StartStopBtn
                } else if has_res && st.btn_copy_rect.contains(x, y) {
                    HoverTarget::CopyBtn
                } else if has_res && st.btn_save_rect.contains(x, y) {
                    HoverTarget::SaveBtn
                } else if st.canvas_rect.contains(x, y)
                    && !st.processing
                    && st.original_img.is_none()
                {
                    HoverTarget::CanvasEmpty
                } else {
                    HoverTarget::None
                }
            } else {
                HoverTarget::None
            };

            match clicked {
                HoverTarget::LangBtn => {
                    if let Ok(mut st) = (*APP_STATE).lock() {
                        st.lang = match st.lang {
                            Lang::En => Lang::Ru,
                            Lang::Ru => Lang::En,
                        };
                        let title = match st.lang {
                            Lang::En => wide_null("Rembg Rust — AI Background Remover"),
                            Lang::Ru => wide_null("Rembg Rust — Удаление фона с картинок"),
                        };
                        SetWindowTextW(hwnd, title.as_ptr());
                    }
                    InvalidateRect(hwnd, null(), 0);
                }
                HoverTarget::ModelDropdown => {
                    show_model_popup_menu(hwnd);
                }
                HoverTarget::ModelPrev => {
                    if let Ok(mut st) = (*APP_STATE).lock() {
                        let next_idx = if st.selected_model_idx == 0 {
                            MODELS.len() - 1
                        } else {
                            st.selected_model_idx - 1
                        };
                        st.switch_model(next_idx);
                    }
                }
                HoverTarget::ModelNext => {
                    if let Ok(mut st) = (*APP_STATE).lock() {
                        let next_idx = (st.selected_model_idx + 1) % MODELS.len();
                        st.switch_model(next_idx);
                    }
                }
                HoverTarget::StartStopBtn => {
                    if let Ok(mut st) = (*APP_STATE).lock() {
                        if st.processing {
                            st.stop_processing(true);
                        } else {
                            st.start_processing();
                        }
                    }
                }
                HoverTarget::CopyBtn => trigger_copy_action(hwnd),
                HoverTarget::SaveBtn => trigger_save_action(hwnd),
                HoverTarget::CanvasEmpty => trigger_open_action(hwnd),
                HoverTarget::None => {}
            }
            0
        }
        WM_PAINT => {
            if let Ok(mut st) = (*APP_STATE).lock() {
                paint_window(hwnd, &mut st);
            }
            0
        }
        WM_CLOSE => {
            PostQuitMessage(0);
            0
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn trigger_copy_action(hwnd: HWND) {
    if let Ok(mut st) = unsafe { (*APP_STATE).lock() } {
        if let Some(ref img) = st.result_img {
            match copy_rgba_to_clipboard(hwnd, img) {
                Ok(()) => {
                    st.set_status(
                        "📋 Transparent PNG copied to clipboard!",
                        "📋 Готовая картинка без фона скопирована в буфер обмена!",
                    );
                }
                Err((en, ru)) => {
                    st.set_status(format!("❌ {en}"), format!("❌ {ru}"));
                }
            }
            unsafe { InvalidateRect(hwnd, null(), 0) };
        }
    }
}

fn trigger_save_action(hwnd: HWND) {
    let (img_opt, src_name, model_id, lang) = if let Ok(st) = unsafe { (*APP_STATE).lock() } {
        (
            st.result_img.clone(),
            st.source_name.clone(),
            st.current_spec().id,
            st.lang,
        )
    } else {
        (None, String::new(), "u2net", Lang::En)
    };

    if let Some(img) = img_opt {
        if let Some(save_path) = save_file_dialog(hwnd, &src_name, model_id, lang) {
            let (status_en, status_ru) = match img.save(&save_path) {
                Ok(()) => (
                    format!("💾 Saved to: {}", save_path.display()),
                    format!("💾 Сохранено в файл: {}", save_path.display()),
                ),
                Err(e) => (
                    format!("❌ Save error: {e}"),
                    format!("❌ Ошибка сохранения: {e}"),
                ),
            };
            if let Ok(mut st) = unsafe { (*APP_STATE).lock() } {
                st.set_status(status_en, status_ru);
            }
            unsafe { InvalidateRect(hwnd, null(), 0) };
        }
    }
}

fn trigger_open_action(hwnd: HWND) {
    let lang = unsafe {
        (*APP_STATE)
            .lock()
            .map(|s| s.lang)
            .unwrap_or(Lang::En)
    };
    if let Some(path) = open_file_dialog(hwnd, lang) {
        match image::open(&path) {
            Ok(img) => {
                let name = path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("image.png")
                    .to_string();
                if let Ok(mut st) = unsafe { (*APP_STATE).lock() } {
                    st.load_image_for_staging(img, name);
                }
            }
            Err(e) => {
                if let Ok(mut st) = unsafe { (*APP_STATE).lock() } {
                    st.set_status(
                        format!("❌ Failed to open file: {e}"),
                        format!("❌ Не удалось открыть файл: {e}"),
                    );
                }
                unsafe { InvalidateRect(hwnd, null(), 0) };
            }
        }
    }
}

pub fn run_gui() -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        SetProcessDPIAware();

        let hinstance = GetModuleHandleW(null());
        let class_name = wide_null("RembgRustWindowClass");
        let window_title = wide_null("Rembg Rust — AI Background Remover");

        let wc = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: wnd_proc,
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance,
            hIcon: LoadIconW(0, IDI_APPLICATION),
            hCursor: LoadCursorW(0, IDC_ARROW),
            hbrBackground: GetStockObject(4) as HBRUSH,
            lpszMenuName: null(),
            lpszClassName: class_name.as_ptr(),
            hIconSm: LoadIconW(0, IDI_APPLICATION),
        };
        RegisterClassExW(&wc);

        let (req_tx, req_rx) = mpsc::channel::<WorkerJob>();
        let (ev_tx, ev_rx) = mpsc::channel::<WorkerEvent>();

        let state = Arc::new(Mutex::new(AppState {
            hwnd: 0,
            lang: Lang::En,
            selected_model_idx: 0,
            worker_tx: req_tx,
            worker_rx: ev_rx,
            processing: false,
            active_cancel: None,
            busy_banner_en: String::new(),
            busy_banner_ru: String::new(),
            current_job_id: 0,
            spinner_step: 0,
            status_en: "Ready • Drag & Drop an image or press Ctrl+V, then click '✂ Cut Out'"
                .to_string(),
            status_ru: "Готово • Перетащите картинку или нажмите Ctrl+V, затем «✂ Вырезать»"
                .to_string(),
            source_name: String::new(),
            original_dyn: None,
            original_img: None,
            result_img: None,
            preview_cache: None,
            hover: HoverTarget::None,
            btn_start_rect: RECT::default(),
            btn_copy_rect: RECT::default(),
            btn_save_rect: RECT::default(),
            btn_model_prev_rect: RECT::default(),
            btn_model_drop_rect: RECT::default(),
            btn_model_next_rect: RECT::default(),
            btn_lang_rect: RECT::default(),
            canvas_rect: RECT::default(),
        }));

        APP_STATE = Arc::into_raw(state.clone()) as *mut Mutex<AppState>;

        let win_w = 1000;
        let win_h = 720;
        let screen_w = GetSystemMetrics(0);
        let screen_h = GetSystemMetrics(1);
        let pos_x = ((screen_w - win_w) / 2).max(40);
        let pos_y = ((screen_h - win_h) / 2).max(40);

        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            window_title.as_ptr(),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE | WS_CLIPCHILDREN,
            pos_x,
            pos_y,
            win_w,
            win_h,
            0,
            0,
            hinstance,
            null_mut(),
        );

        if hwnd == 0 {
            return Err("Failed to create application window".into());
        }

        if let Ok(mut st) = state.lock() {
            st.hwnd = hwnd;
        }

        thread::spawn(move || {
            let mut active_engine: Option<(usize, RembgEngine)> = None;

            while let Ok(mut job) = req_rx.recv() {
                while let Ok(newer) = req_rx.try_recv() {
                    job = newer;
                }

                if job.cancel_flag.load(Ordering::Relaxed) {
                    continue;
                }

                let job_id = job.job_id;
                let model_idx = job.model_idx;
                let spec = MODELS[model_idx];

                let need_load = active_engine
                    .as_ref()
                    .map(|(idx, _)| *idx != model_idx)
                    .unwrap_or(true);

                if need_load {
                    if !spec.is_downloaded() {
                        let _ = ev_tx.send(WorkerEvent::DownloadProgress {
                            job_id,
                            model_idx,
                            downloaded_mb: 0.0,
                            total_mb: spec.size_mb as f32,
                        });
                        let _ = PostMessageW(hwnd, WM_APP_WORKER_MSG, 0, 0);
                    } else {
                        let _ = ev_tx.send(WorkerEvent::ModelCompiling { job_id, model_idx });
                        let _ = PostMessageW(hwnd, WM_APP_WORKER_MSG, 0, 0);
                    }

                    let tx_progress = ev_tx.clone();
                    let path_res =
                        crate::ensure_model_spec(&spec, &job.cancel_flag, move |dl, total| {
                            let _ = tx_progress.send(WorkerEvent::DownloadProgress {
                                job_id,
                                model_idx,
                                downloaded_mb: dl as f32 / 1_048_576.0,
                                total_mb: total as f32 / 1_048_576.0,
                            });
                            let _ = PostMessageW(hwnd, WM_APP_WORKER_MSG, 0, 0);
                        });

                    let path = match path_res {
                        Ok(p) => p,
                        Err(e) => {
                            let _ = ev_tx.send(WorkerEvent::JobFinished {
                                job_id,
                                model_idx,
                                result: Err(e.to_string()),
                                source_name: job.source_name,
                            });
                            let _ = PostMessageW(hwnd, WM_APP_WORKER_MSG, 0, 0);
                            continue;
                        }
                    };

                    if job.cancel_flag.load(Ordering::Relaxed) {
                        continue;
                    }

                    let _ = ev_tx.send(WorkerEvent::ModelCompiling { job_id, model_idx });
                    let _ = PostMessageW(hwnd, WM_APP_WORKER_MSG, 0, 0);

                    match crate::build_runnable_model(&path, spec.input_w, spec.input_h) {
                        Ok(model) => {
                            active_engine = Some((model_idx, RembgEngine { spec, model }));
                        }
                        Err(e) => {
                            let _ = ev_tx.send(WorkerEvent::JobFinished {
                                job_id,
                                model_idx,
                                result: Err(e.to_string()),
                                source_name: job.source_name,
                            });
                            let _ = PostMessageW(hwnd, WM_APP_WORKER_MSG, 0, 0);
                            continue;
                        }
                    }
                }

                if job.cancel_flag.load(Ordering::Relaxed) {
                    continue;
                }

                if let Some((_, ref eng)) = active_engine {
                    let _ = ev_tx.send(WorkerEvent::RunningInference { job_id, model_idx });
                    let _ = PostMessageW(hwnd, WM_APP_WORKER_MSG, 0, 0);

                    let t0 = Instant::now();
                    let res = eng
                        .remove_background_image(&job.img)
                        .map(|out| (out, t0.elapsed().as_secs_f32()))
                        .map_err(|e| e.to_string());

                    if !job.cancel_flag.load(Ordering::Relaxed) {
                        let _ = ev_tx.send(WorkerEvent::JobFinished {
                            job_id,
                            model_idx,
                            result: res,
                            source_name: job.source_name,
                        });
                        let _ = PostMessageW(hwnd, WM_APP_WORKER_MSG, 0, 0);
                    }
                }
            }
        });

        let mut msg: MSG = zeroed();
        while GetMessageW(&mut msg, 0, 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    Ok(())
}
