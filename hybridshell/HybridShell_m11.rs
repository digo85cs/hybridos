// ============================================================
// HYBRID OS - CAMADA B / MARCO M11: ICONES REAIS NA DOCK
// M10 + dock com icones desenhados (engrenagem, raio, pasta, X)
// em vez de circulos coloridos - visual de sistema de verdade.
// ============================================================

#![windows_subsystem = "windows"]

use std::io::Write;
use std::rc::Rc;
use softbuffer::{Context, Surface};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowLevel},
};

fn log_msg(msg: &str) {
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("C:/Users/Rodrigo/hs.log") {
        let _ = writeln!(f, "{}", msg);
    }
}

// ---------- Perfil de hardware ----------
#[derive(Debug, Clone, Copy, PartialEq)]
enum RenderTechnique {
    Efficient,
    Full,
}

#[derive(Debug)]
struct HardwareProfile {
    technique: RenderTechnique,
    total_ram_mb: u64,
    fonte: &'static str,
}

impl HardwareProfile {
    // M6: ResetExplode e a fonte de verdade - relatorio.json
    fn ram_do_relatorio() -> Option<u64> {
        let txt = match std::fs::read_to_string("C:/Users/Rodrigo/ResetExplode/relatorio.json") {
            Ok(t) => t,
            Err(_) => std::fs::read_to_string("C:/Users/Rodrigo/relatorio.json").ok()?,
        };
        let chaves = ["\"ram_total_mb\"", "\"total_ram_mb\"", "\"ramMb\"", "\"totalRamMb\"", "\"memoria_total_mb\"", "\"ram_mb\""];
        for chave in chaves.iter() {
            if let Some(pos) = txt.find(chave) {
                let depois = &txt[pos + chave.len()..];
                let depois = depois.trim_start_matches(|c: char| c == ':' || c == ' ' || c == '"' || c == '\t' || c == '\r' || c == '\n');
                let num: String = depois.chars().take_while(|c| c.is_ascii_digit()).collect();
                if let Ok(v) = num.parse::<u64>() {
                    if v > 0 {
                        log_msg(&format!("M6: relatorio.json -> {} MB", v));
                        return Some(v);
                    }
                }
            }
        }
        log_msg("M6: relatorio.json sem chave de RAM conhecida");
        None
    }

    fn detect() -> Self {
        if let Some(ram) = Self::ram_do_relatorio() {
            let technique = if ram < 8192 { RenderTechnique::Efficient } else { RenderTechnique::Full };
            return HardwareProfile { technique, total_ram_mb: ram, fonte: "JSON" };
        }
        log_msg("M6: relatorio.json ausente - fallback local");
        let total_ram_mb = Self::ram_mb();
        let has_dx12 = Self::has_dx12_gpu();
        let technique = if total_ram_mb < 8192 || !has_dx12 {
            RenderTechnique::Efficient
        } else {
            RenderTechnique::Full
        };
        HardwareProfile { technique, total_ram_mb, fonte: "LOCAL" }
    }

    fn ram_mb() -> u64 {
        use std::mem;
        #[repr(C)]
        struct MemoryStatusEx {
            dw_length: u32,
            dw_memory_load: u32,
            ull_total_phys: u64,
            ull_avail_phys: u64,
        }
        extern "system" {
            fn GlobalMemoryStatusEx(buf: *mut MemoryStatusEx) -> i32;
        }
        let mut ms = MemoryStatusEx {
            dw_length: mem::size_of::<MemoryStatusEx>() as u32,
            dw_memory_load: 0,
            ull_total_phys: 0,
            ull_avail_phys: 0,
        };
        unsafe {
            if GlobalMemoryStatusEx(&mut ms) != 0 {
                ms.ull_total_phys / (1024 * 1024)
            } else {
                4096
            }
        }
    }

    fn has_dx12_gpu() -> bool {
        false
    }
}

// ---------- Cores do tema ----------
const BG: u32 = 0xFF1E1E2E;
const DOCK_BG: u32 = 0xE6303046;
const ICON: u32 = 0xFFF84808;
const ICON2: u32 = 0xFF89B4FA;
const ICON3: u32 = 0xFFA6E3A1;
const ICON4: u32 = 0xFFF9E2AF;
const DOCK_H: i32 = 84;
const ITEM_BASE: i32 = 52;

// ---------- Menu global (estilo Apple) ----------
const MENU_H: i32 = 28;

struct MenuBar {
    itens: [&'static str; 3],
    hover: i32,
}

impl MenuBar {
    fn new() -> Self {
        MenuBar { itens: ["SISTEMA", "RESET", "ARQUIVOS"], hover: -1 }
    }

    fn hit_test(&self, x: f32) -> i32 {
        if x < 40.0 { return -1; }
        let idx = ((x - 40.0) / 90.0) as i32;
        if idx < 3 { idx } else { -1 }
    }
}

// ---------- Dock ----------
struct Dock {
    items: Vec<u32>,
    mouse_x: f32,
    hover: i32,
}

impl Dock {
    fn new() -> Self {
        Dock { items: vec![ICON, ICON2, ICON3, ICON4], mouse_x: -1.0, hover: -1 }
    }

    fn scale_for(&self, item_center_x: f32) -> f32 {
        if self.mouse_x < 0.0 {
            return 1.0;
        }
        let dist = (self.mouse_x - item_center_x).abs();
        let influence = 120.0;
        if dist < influence {
            1.0 + 0.6 * (1.0 - dist / influence)
        } else {
            1.0
        }
    }

    fn hit_test(&self, w: i32, h: i32, mouse_y: f32) -> i32 {
        if mouse_y < (h - DOCK_H) as f32 {
            return -1;
        }
        let n = self.items.len() as i32;
        let spacing = 72;
        let start_x = (w - n * spacing) / 2 + spacing / 2;
        for i in 0..n {
            let cx = start_x + i * spacing;
            let cy = h - DOCK_H / 2;
            let dx = (self.mouse_x - cx as f32).abs();
            let dy = (mouse_y - cy as f32).abs();
            if dx < 26.0 && dy < 26.0 {
                return i;
            }
        }
        -1
    }
}

// ---------- Spotlight (busca de apps, estilo Apple) ----------
struct Spotlight {
    aberto: bool,
    query: String,
}

impl Spotlight {
    // Primeiro app do catalogo que casa com a query (por prefixo)
    fn buscar(&self) -> Option<(&'static str, &'static str)> {
        let q = self.query.to_uppercase();
        if q.is_empty() { return None; }
        for (nome, caminho) in SPOT_CATALOG {
            if nome.starts_with(&q) {
                return Some((nome, caminho));
            }
        }
        None
    }
}

// Catalogo de apps
const SPOT_CATALOG: &[(&str, &str)] = &[
    ("RESET", "C:\\Users\\Rodrigo\\ResetExplode\\ResetExplode.exe"),
    ("ARQUIVOS", "C:\\Windows\\explorer.exe"),
    ("SAIR", "@SAIR"),
];

// ---------- Hora real do sistema (M8) ----------
fn agora_hhmm() -> (u32, u32) {
    let ok = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH);
    let secs = ok.map(|d| d.as_secs()).unwrap_or(0) as i64;
    // Brasil = UTC-3
    let mins = (secs / 60 - 3 * 60).rem_euclid(1440);
    ((mins / 60) as u32, (mins % 60) as u32)
}

// ---------- Helpers de centro da tela (M9) ----------
fn w_center(win: &Option<Rc<Window>>) -> i32 {
    win.as_ref().map(|w| w.inner_size().width as i32).unwrap_or(1024)
}

fn h_center(win: &Option<Rc<Window>>) -> i32 {
    win.as_ref().map(|w| w.inner_size().height as i32).unwrap_or(768)
}

// ---------- Janela real (M7: barra + fechar + arrastar) ----------
struct Janela {
    aberta: bool,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    arrastando: bool,
    dx: i32,
    dy: i32,
}

impl Janela {
    fn new() -> Self {
        Janela { aberta: true, x: 120, y: 90, w: 420, h: 260, arrastando: false, dx: 0, dy: 0 }
    }

    fn hit_close(&self, mx: i32, my: i32) -> bool {
        let cx = self.x + self.w - 18;
        let cy = self.y + 15;
        let ddx = (mx - cx) as f32;
        let ddy = (my - cy) as f32;
        (ddx * ddx + ddy * ddy).sqrt() <= 8.0
    }

    fn hit_barra(&self, mx: i32, my: i32) -> bool {
        mx >= self.x && mx <= self.x + self.w && my >= self.y && my <= self.y + 30 && !self.hit_close(mx, my)
    }
}

struct ShellApp {
    profile: HardwareProfile,
    dock: Dock,
    menu: MenuBar,
    spot: Spotlight,
    janelas: Vec<Janela>,
    mission: bool,
    mission_rects: Vec<(usize, i32, i32, i32, i32)>,
    mouse: (i32, i32),
    window: Option<Rc<Window>>,
    surface: Option<Surface<Rc<Window>, Rc<Window>>>,
    frame: u32,
}

impl ShellApp {
    fn new(profile: HardwareProfile) -> Self {
        ShellApp { profile, dock: Dock::new(), menu: MenuBar::new(), spot: Spotlight { aberto: false, query: String::new() }, janelas: Vec::new(), mission: false, mission_rects: Vec::new(), mouse: (0, 0), window: None, surface: None, frame: 0 }
    }

    fn draw_circle(buf: &mut [u32], w: i32, h: i32, cx: i32, cy: i32, r: i32, color: u32) {
        for y in (cy - r - 1).max(0)..=(cy + r + 1).min(h - 1) {
            for x in (cx - r - 1).max(0)..=(cx + r + 1).min(w - 1) {
                let dx = (x - cx) as f32;
                let dy = (y - cy) as f32;
                let d = (dx * dx + dy * dy).sqrt();
                if d <= r as f32 {
                    let idx = (y * w + x) as usize;
                    buf[idx] = color;
                }
            }
        }
    }

    // M11: icone ENGRAGEM (sistema)
    fn draw_icone_gear(buf: &mut [u32], w: i32, h: i32, cx: i32, cy: i32, r: i32, color: u32) {
        Self::draw_circle(buf, w, h, cx, cy, r, color);
        Self::draw_circle(buf, w, h, cx, cy, r * 5 / 10, 0xFF1E1E2E);
        for k in 0..8 {
            let ang = k as f32 * std::f32::consts::PI / 4.0;
            let tx = cx + (ang.cos() * (r as f32 + 3.0)) as i32;
            let ty = cy + (ang.sin() * (r as f32 + 3.0)) as i32;
            Self::draw_circle(buf, w, h, tx, ty, r / 4, color);
        }
    }

    // M11: icone RAIO (reset)
    fn draw_icone_bolt(buf: &mut [u32], w: i32, h: i32, cx: i32, cy: i32, r: i32, color: u32) {
        let pts: [(i32, i32); 7] = [
            (cx + r / 3, cy - r), (cx - r / 2, cy + r / 5), (cx - r / 10, cy + r / 5),
            (cx - r / 3, cy + r), (cx + r / 2, cy - r / 5), (cx + r / 10, cy - r / 5),
            (cx + r / 3, cy - r),
        ];
        for i in 0..pts.len() - 1 {
            Self::draw_line(buf, w, h, pts[i].0, pts[i].1, pts[i + 1].0, pts[i + 1].1, (r / 2).max(3), color);
        }
    }

    // M11: icone PASTA (arquivos)
    fn draw_icone_folder(buf: &mut [u32], w: i32, h: i32, cx: i32, cy: i32, r: i32, color: u32) {
        let fw = r * 2;
        let fh = r * 3 / 2;
        let fx = cx - r;
        let fy = cy - r / 2;
        Self::draw_rounded_rect(buf, w, h, fx, fy, fw, fh, 4, color);
        Self::draw_rounded_rect(buf, w, h, fx, fy - r / 3, fw / 2, r / 2 + 2, 3, color);
    }

    // M11: icone X (sair)
    fn draw_icone_x(buf: &mut [u32], w: i32, h: i32, cx: i32, cy: i32, r: i32, color: u32) {
        let t = (r / 2).max(3);
        Self::draw_line(buf, w, h, cx - r / 2, cy - r / 2, cx + r / 2, cy + r / 2, t, color);
        Self::draw_line(buf, w, h, cx + r / 2, cy - r / 2, cx - r / 2, cy + r / 2, t, color);
    }

    // M11: linha grossa (Bresenham com espessura)
    fn draw_line(buf: &mut [u32], w: i32, h: i32, x0: i32, y0: i32, x1: i32, y1: i32, thick: i32, color: u32) {
        let dx = (x1 - x0).abs();
        let dy = (y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx - dy;
        let mut x = x0;
        let mut y = y0;
        loop {
            for oy in -(thick / 2)..=(thick / 2) {
                for ox in -(thick / 2)..=(thick / 2) {
                    let px = x + ox;
                    let py = y + oy;
                    if px >= 0 && px < w && py >= 0 && py < h {
                        buf[(py * w + px) as usize] = color;
                    }
                }
            }
            if x == x1 && y == y1 { break; }
            let e2 = 2 * err;
            if e2 > -dy { err -= dy; x += sx; }
            if e2 < dx { err += dx; y += sy; }
        }
    }

    fn draw_rounded_rect(buf: &mut [u32], w: i32, h: i32, x0: i32, y0: i32, rw: i32, rh: i32, rad: i32, color: u32) {
        for y in y0.max(0)..=(y0 + rh).min(h - 1) {
            for x in x0.max(0)..=(x0 + rw).min(w - 1) {
                let in_corner_x = x < x0 + rad || x >= x0 + rw - rad;
                let in_corner_y = y < y0 + rad || y >= y0 + rh - rad;
                if in_corner_x && in_corner_y {
                    let ccx = if x < x0 + rad { x0 + rad } else { x0 + rw - rad - 1 };
                    let ccy = if y < y0 + rad { y0 + rad } else { y0 + rh - rad - 1 };
                    let dx = (x - ccx) as f32;
                    let dy = (y - ccy) as f32;
                    if (dx * dx + dy * dy).sqrt() > rad as f32 {
                        continue;
                    }
                }
                let idx = (y * w + x) as usize;
                buf[idx] = color;
            }
        }
    }

    // Fonte bitmap 5x7 embutida (A-Z 0-9 espaco . :)
    fn glyph(ch: u8) -> [u8; 7] {
        match ch {
            b'A' => [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
            b'B' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110],
            b'C' => [0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110],
            b'D' => [0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110],
            b'E' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
            b'F' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000],
            b'G' => [0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01110],
            b'H' => [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
            b'I' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111],
            b'J' => [0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100],
            b'K' => [0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001],
            b'L' => [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111],
            b'M' => [0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001],
            b'N' => [0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001],
            b'O' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
            b'P' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000],
            b'Q' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101],
            b'R' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
            b'S' => [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110],
            b'T' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
            b'U' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
            b'V' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100],
            b'W' => [0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001],
            b'X' => [0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001],
            b'Y' => [0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100],
            b'Z' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111],
            b'0' => [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110],
            b'1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
            b'2' => [0b01110, 0b10001, 0b00001, 0b00110, 0b01000, 0b10000, 0b11111],
            b'3' => [0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110],
            b'4' => [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
            b'5' => [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110],
            b'6' => [0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110],
            b'7' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
            b'8' => [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
            b'9' => [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100],
            b' ' => [0, 0, 0, 0, 0, 0, 0],
            b'.' => [0, 0, 0, 0, 0, 0b00110, 0b00110],
            b':' => [0, 0b00110, 0b00110, 0, 0b00110, 0b00110, 0],
            _ => [0, 0, 0, 0, 0, 0, 0],
        }
    }

    fn draw_text(buf: &mut [u32], w: i32, h: i32, x0: i32, y0: i32, text: &str, scale: i32, color: u32) {
        let mut x = x0;
        for b in text.bytes() {
            let g = Self::glyph(b.to_ascii_uppercase());
            for (row, bits) in g.iter().enumerate() {
                for col in 0..5 {
                    if bits & (0b10000 >> col) != 0 {
                        for sy in 0..scale {
                            for sx in 0..scale {
                                let px = x + col * scale + sx;
                                let py = y0 + (row as i32) * scale + sy;
                                if px >= 0 && px < w && py >= 0 && py < h {
                                    buf[(py * w + px) as usize] = color;
                                }
                            }
                        }
                    }
                }
            }
            x += 6 * scale;
        }
    }

    fn draw(&mut self) {
        let Some(win) = self.window.clone() else { return };
        let Some(surf) = self.surface.as_mut() else { return };
        let size = win.inner_size();
        let (w, h) = (size.width as i32, size.height as i32);
        if w == 0 || h == 0 {
            return;
        }

        let (w32, h32) = (w.max(1) as u32, h.max(1) as u32);
        surf.resize(std::num::NonZeroU32::new(w32).unwrap(), std::num::NonZeroU32::new(h32).unwrap()).ok();
        let mut buf = surf.buffer_mut().unwrap();

        for p in buf.iter_mut() {
            *p = BG;
        }

        // ===== MENU GLOBAL no topo (estilo Apple) =====
        Self::draw_rounded_rect(&mut buf, w, h, 0, 0, w, MENU_H, 0, 0xE61E1E2E);
        Self::draw_circle(&mut buf, w, h, 20, MENU_H / 2, 8, ICON);
        for (i, nome) in self.menu.itens.iter().enumerate() {
            let mx = 40 + i as i32 * 90;
            let cor = if self.menu.hover == i as i32 { 0xFFCDD6F4 } else { 0xFFA6ADC8 };
            Self::draw_text(&mut buf, w, h, mx, 9, nome, 1, cor);
        }
        // relogio REAL do sistema (M8: hora de verdade, UTC-3)
        let (hh, mm) = agora_hhmm();
        let mut rel = String::new();
        if hh < 10 { rel.push('0'); }
        rel.push_str(&hh.to_string());
        rel.push(':');
        if mm < 10 { rel.push('0'); }
        rel.push_str(&mm.to_string());
        let rw = rel.len() as i32 * 6 * 2;
        Self::draw_text(&mut buf, w, h, w - rw - 16, 9, &rel, 2, 0xFFCDD6F4);

        // ===== MISSION CONTROL (M10: grade de miniaturas) =====
        if self.mission {
            self.mission_rects.clear();
            Self::draw_rounded_rect(&mut buf, w, h, 0, 0, w, h, 0, 0xE6101014);
            let nj = self.janelas.len();
            for (idx, j) in self.janelas.iter().enumerate() {
                let col = (idx % 2) as i32;
                let row = (idx / 2) as i32;
                let mw = w / 2 - 80;
                let mh = h / 2 - 70;
                let mx = 40 + col * (mw + 40);
                let my = MENU_H + 30 + row * (mh + 30);
                Self::draw_rounded_rect(&mut buf, w, h, mx + 4, my + 4, mw, mh, 10, 0x88101014);
                Self::draw_rounded_rect(&mut buf, w, h, mx, my, mw, mh, 10, 0xFF313244);
                Self::draw_rounded_rect(&mut buf, w, h, mx, my, mw, 24, 10, 0xFF45475A);
                Self::draw_text(&mut buf, w, h, mx + 10, my + 8, &format!("JANELA {}", idx + 1), 1, 0xFFCDD6F4);
                Self::draw_text(&mut buf, w, h, mx + 10, my + 45, &format!("RAM: {} MB", self.profile.total_ram_mb), 1, 0xFFA6ADC8);
                Self::draw_text(&mut buf, w, h, mx + 10, my + 65, &format!("FONTE: {}", self.profile.fonte), 1, 0xFFA6ADC8);
                self.mission_rects.push((idx, mx, my, mw, mh));
            }
            let dica = format!("{} JANELAS - CLIQUE PRA FOCAR / ESC VOLTA", nj);
            let dw = dica.len() as i32 * 6;
            Self::draw_text(&mut buf, w, h, (w - dw) / 2, h - DOCK_H - 30, &dica, 1, 0xFF6C7086);
        }

        // ===== MULTI-JANELAS (M9: ultima da lista = foco, no topo) =====
        let nj = self.janelas.len();
        if !self.mission {
        for (idx, j) in self.janelas.iter().enumerate() {
            let foco = idx + 1 == nj;
            let jx = j.x;
            let jy = j.y;
            let jw = j.w;
            let jh = j.h;
            let barra_cor = if foco { 0xFF45475A } else { 0xFF3B3B4E };
            let corpo = if foco { 0xFF313244 } else { 0xFF2B2B3C };
            Self::draw_rounded_rect(&mut buf, w, h, jx + 5, jy + 5, jw, jh, 12, 0x66101014);
            Self::draw_rounded_rect(&mut buf, w, h, jx, jy, jw, jh, 12, corpo);
            Self::draw_rounded_rect(&mut buf, w, h, jx, jy, jw, 30, 12, barra_cor);
            let titulo = if foco { "SOBRE" } else { "SOBRE (fundo)" };
            Self::draw_text(&mut buf, w, h, jx + 14, jy + 10, titulo, 2, 0xFFCDD6F4);
            Self::draw_circle(&mut buf, w, h, jx + jw - 18, jy + 15, 7, 0xFFF38BA8);
            Self::draw_text(&mut buf, w, h, jx + jw - 23, jy + 11, "X", 1, 0xFF1E1E2E);
            Self::draw_text(&mut buf, w, h, jx + 20, jy + 55, &format!("JANELA {} DE {}", idx + 1, nj), 2, 0xFFCDD6F4);
            Self::draw_text(&mut buf, w, h, jx + 20, jy + 95, &format!("RAM: {} MB", self.profile.total_ram_mb), 2, 0xFFA6ADC8);
            Self::draw_text(&mut buf, w, h, jx + 20, jy + 125, &format!("FONTE: {}", self.profile.fonte), 2, 0xFFA6ADC8);
            Self::draw_text(&mut buf, w, h, jx + 20, jy + 170, "CLIQUE PRA TRAZER PRA FRENTE", 1, 0xFF6C7086);
            Self::draw_text(&mut buf, w, h, jx + 20, jy + 190, "ESC OU X FECHA", 1, 0xFF6C7086);
        }
        }

        // dock
        Self::draw_rounded_rect(&mut buf, w, h, 0, h - DOCK_H, w, DOCK_H, 12, DOCK_BG);

        // itens com amplificacao (M11: icones reais)
        let n = self.dock.items.len() as i32;
        let spacing = 72;
        let start_x = (w - n * spacing) / 2 + spacing / 2;
        for i in 0..n {
            let cx = start_x + i * spacing;
            let cy = h - DOCK_H / 2;
            let scale = self.dock.scale_for(cx as f32);
            let r = (ITEM_BASE as f32 / 2.0 * scale) as i32;
            // M11: fundo arredondado + icone real de cada app
            Self::draw_rounded_rect(&mut buf, w, h, cx - r, cy - r, r * 2, r * 2, r / 3, self.dock.items[i as usize]);
            match i {
                0 => Self::draw_icone_gear(&mut buf, w, h, cx, cy, r * 2 / 3, 0xFFE6E9F0),
                1 => Self::draw_icone_bolt(&mut buf, w, h, cx, cy, r * 2 / 3, 0xFF1E1E2E),
                2 => Self::draw_icone_folder(&mut buf, w, h, cx, cy, r * 2 / 3, 0xFF1E1E2E),
                3 => Self::draw_icone_x(&mut buf, w, h, cx, cy, r * 2 / 3, 0xFFE6E9F0),
                _ => {}
            }
        }

        // nome do app em hover
        let nomes = ["SISTEMA", "RESET", "ARQUIVOS", "SAIR"];
        if self.dock.hover >= 0 && (self.dock.hover as usize) < nomes.len() {
            let cx = start_x + self.dock.hover * spacing;
            let tw = nomes[self.dock.hover as usize].len() as i32 * 6 * 2;
            Self::draw_text(&mut buf, w, h, cx - tw / 2, h - DOCK_H - 24, nomes[self.dock.hover as usize], 2, 0xFFCDD6F4);
        }

        // ===== SPOTLIGHT (caixa de busca no centro, quando aberto) =====
        if self.spot.aberto {
            Self::draw_rounded_rect(&mut buf, w, h, 0, 0, w, h, 0, 0x90101014);
            let bw = 560;
            let bx = (w - bw) / 2;
            let by = h / 3;
            Self::draw_rounded_rect(&mut buf, w, h, bx, by, bw, 64, 14, 0xF82A2A3C);
            Self::draw_rounded_rect(&mut buf, w, h, bx, by, bw, 3, 0, ICON);
            let mut mostrar = self.spot.query.clone();
            if (self.frame / 30) % 2 == 0 { mostrar.push('|'); }
            Self::draw_text(&mut buf, w, h, bx + 20, by + 24, &mostrar, 2, 0xFFCDD6F4);
            if let Some((nome, _)) = self.spot.buscar() {
                Self::draw_text(&mut buf, w, h, bx + 20, by + 76, nome, 1, 0xFFA6E3A1);
            }
        }

        if self.frame == 0 {
            log_msg("primeiro frame OK");
        }
        buf.present().unwrap();
        self.frame += 1;
    }

    fn abrir_janela(&mut self) {
        let n = self.janelas.len() as i32;
        let mut j = Janela::new();
        j.x = (w_center(&self.window) - j.w) / 2 + n * 30;
        j.y = (h_center(&self.window) - j.h) / 2 + n * 24;
        self.janelas.push(j);
        log_msg(&format!("janela SOBRE {} aberta", n + 1));
    }

    fn activate_menu(&mut self, item: i32) {
        match item {
            0 => self.abrir_janela(),
            1 => {
                log_msg("menu RESET: abrindo ResetExplode");
                let _ = std::process::Command::new("C:\\Users\\Rodrigo\\ResetExplode\\ResetExplode.exe").spawn();
            }
            2 => {
                log_msg("menu ARQUIVOS: abrindo explorer");
                let _ = std::process::Command::new("C:\\Windows\\explorer.exe").spawn();
            }
            _ => log_msg("menu clicado"),
        }
    }

    fn activate(&mut self, item: i32) {
        match item {
            0 => self.abrir_janela(),
            1 => {
                log_msg("abrindo ResetExplode da dock");
                let _ = std::process::Command::new("C:\\Users\\Rodrigo\\ResetExplode\\ResetExplode.exe").spawn();
            }
            2 => {
                log_msg("abrindo explorer da dock");
                let _ = std::process::Command::new("C:\\Windows\\explorer.exe").spawn();
            }
            3 => {
                log_msg("SAIR solicitado da dock");
                std::process::exit(0);
            }
            _ => {
                log_msg("item da dock sem app ainda");
            }
        }
    }
}

impl ApplicationHandler for ShellApp {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.window.is_none() {
            let attrs = Window::default_attributes()
                .with_title("HybridShell")
                .with_window_level(WindowLevel::AlwaysOnBottom)
                .with_decorations(false);
            match el.create_window(attrs) {
                Ok(w) => {
                    let win = Rc::new(w);
                    match Context::new(win.clone()) {
                        Ok(ctx) => {
                            match Surface::new(&ctx, win.clone()) {
                                Ok(surf) => {
                                    self.surface = Some(surf);
                                    self.window = Some(win);
                                    log_msg("janela criada OK");
                                }
                                Err(e) => log_msg(&format!("surface ERRO: {:?}", e)),
                            }
                        }
                        Err(e) => log_msg(&format!("contexto ERRO: {:?}", e)),
                    }
                }
                Err(e) => log_msg(&format!("janela ERRO: {:?}", e)),
            }
        }
    }

    fn window_event(&mut self, _el: &ActiveEventLoop, _id: winit::window::WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse = (position.x as i32, position.y as i32);
                let win = self.window.clone();
                if let Some(win) = win {
                    let size = win.inner_size();
                    self.dock.mouse_x = position.x as f32;
                    self.dock.hover = self.dock.hit_test(size.width as i32, size.height as i32, position.y as f32);
                    if position.y < MENU_H as f64 {
                        self.menu.hover = self.menu.hit_test(position.x as f32);
                    } else {
                        self.menu.hover = -1;
                    }
                }
                let (mx, my) = self.mouse;
                if let Some(j) = self.janelas.iter_mut().find(|j| j.arrastando) {
                    j.x = mx - j.dx;
                    j.y = my - j.dy;
                }
                self.draw();
            }
            WindowEvent::MouseInput { state, .. } => {
                if state == winit::event::ElementState::Pressed {
                    // M10: clique no mission control foca a janela clicada
                    if self.mission {
                        let (mx, my) = self.mouse;
                        let mut focar: Option<usize> = None;
                        for (idx, rx, ry, rw2, rh2) in self.mission_rects.iter() {
                            if mx >= *rx && mx <= *rx + *rw2 && my >= *ry && my <= *ry + *rh2 {
                                focar = Some(*idx);
                                break;
                            }
                        }
                        if let Some(idx) = focar {
                            let mut j = self.janelas.remove(idx);
                            j.arrastando = false;
                            self.janelas.push(j);
                            log_msg("mission: janela focada");
                        }
                        self.mission = false;
                        self.draw();
                        return;
                    }
                    // M9: varre de CIMA pra baixo - a primeira janela sob o mouse manda
                    let (mx, my) = self.mouse;
                    let mut fechar: Option<usize> = None;
                    let mut pegar: Option<usize> = None;
                    for idx in (0..self.janelas.len()).rev() {
                        let j = &self.janelas[idx];
                        if j.hit_close(mx, my) { fechar = Some(idx); break; }
                        if j.hit_barra(mx, my) { pegar = Some(idx); break; }
                    }
                    if let Some(idx) = fechar {
                        self.janelas.remove(idx);
                        log_msg("janela fechada (x)");
                        self.draw();
                        return;
                    }
                    if let Some(idx) = pegar {
                        let mut j = self.janelas.remove(idx);
                        j.arrastando = true;
                        j.dx = mx - j.x;
                        j.dy = my - j.y;
                        self.janelas.push(j);
                        log_msg("janela pega pela barra (foco)");
                        self.draw();
                        return;
                    }
                    if self.menu.hover >= 0 {
                        self.activate_menu(self.menu.hover);
                    } else if self.dock.hover >= 0 {
                        self.activate(self.dock.hover);
                    }
                } else if state == winit::event::ElementState::Released {
                    if let Some(mut j) = self.janelas.iter_mut().find(|j| j.arrastando) {
                        j.arrastando = false;
                        log_msg("janela solta");
                    }
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == winit::event::ElementState::Pressed {
                    // Espaco abre/fecha o spotlight
                    if event.logical_key == winit::keyboard::Key::Named(winit::keyboard::NamedKey::Space) {
                        self.spot.aberto = !self.spot.aberto;
                        self.spot.query.clear();
                        log_msg("spotlight alternado");
                        self.draw();
                        return;
                    }
                    if self.spot.aberto {
                        match &event.logical_key {
                            winit::keyboard::Key::Character(c) => {
                                let ch = c.as_str().chars().next();
                                if let Some(ch) = ch {
                                    if ch.is_ascii_graphic() {
                                        self.spot.query.push(ch);
                                    }
                                }
                                self.draw();
                            }
                            winit::keyboard::Key::Named(winit::keyboard::NamedKey::Backspace) => {
                                self.spot.query.pop();
                                self.draw();
                            }
                            winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape) => {
                                self.spot.aberto = false;
                                self.spot.query.clear();
                                log_msg("spotlight fechado (esc)");
                                self.draw();
                            }
                            winit::keyboard::Key::Named(winit::keyboard::NamedKey::Enter) => {
                                if let Some((nome, caminho)) = self.spot.buscar() {
                                    if caminho == "@SAIR" {
                                        log_msg("spotlight SAIR");
                                        std::process::exit(0);
                                    }
                                    log_msg(&format!("spotlight abrindo: {}", nome));
                                    let _ = std::process::Command::new(caminho).spawn();
                                }
                                self.spot.aberto = false;
                                self.spot.query.clear();
                                self.draw();
                            }
                            _ => {}
                        }
                    }
                    // M7: tecla I abre a janela SOBRE / ESC fecha a do topo (spotlight fechado)
                    if !self.spot.aberto {
                        if let winit::keyboard::Key::Character(c) = &event.logical_key {
                            if c.as_str().eq_ignore_ascii_case("i") {
                                self.abrir_janela();
                                self.draw();
                                return;
                            }
                            // M10: tecla M alterna o Mission Control
                            if c.as_str().eq_ignore_ascii_case("m") {
                                self.mission = !self.mission;
                                log_msg("mission control alternado");
                                self.draw();
                                return;
                            }
                        }
                        if self.mission && event.logical_key == winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape) {
                            self.mission = false;
                            log_msg("mission fechado (esc)");
                            self.draw();
                            return;
                        }
                        if !self.janelas.is_empty() && event.logical_key == winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape) {
                            self.janelas.pop();
                            log_msg("janela do topo fechada (esc)");
                            self.draw();
                        }
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                self.draw();
            }
            WindowEvent::CloseRequested => {
                log_msg("close solicitado - permanece");
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _el: &ActiveEventLoop) {
        // M8: render loop continuo ~60fps - relogio anda e cursor pisca sozinhos
        if let Some(win) = self.window.clone() {
            std::thread::sleep(std::time::Duration::from_millis(16));
            win.request_redraw();
        }
    }
}

fn main() {
    std::panic::set_hook(Box::new(|info| {
        log_msg(&format!("PANIC: {}", info));
    }));
    log_msg("shell start");
    let profile = HardwareProfile::detect();
    log_msg(&format!("perfil: {:?} ", profile));

    let event_loop = EventLoop::new().expect("event loop");
    let mut app = ShellApp::new(profile);
    event_loop.run_app(&mut app).expect("rodar shell");
}
