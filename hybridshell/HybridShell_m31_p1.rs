// ============================================================
// HYBRID OS - CAMADA B / MARCO M25: BOTÕES DE TRÁFEGO
// M24 + as 3 luzes do macOS: vermelho fecha, AMARELO minimiza
// (janela some; clique no SISTEMA da dock restaura) e VERDE
// maximiza/restaura (preenche entre menu e dock).
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

extern "system" {
    fn GetAsyncKeyState(vkey: i32) -> i16;
}

const VK_CONTROL: i32 = 0x11;
const VK_SPACE: i32 = 0x20;

fn tecla_pressionada(vkey: i32) -> bool {
    unsafe { GetAsyncKeyState(vkey) as u16 & 0x8000 != 0 }
}

#[repr(C)]
struct PowerStatus {
    ac_line: u8,
    battery_flag: u8,
    battery_percent: u8,
    _reservado: u8,
    _segundos_vida: u32,
    _segundos_cheia: u32,
}

extern "system" {
    fn GetSystemPowerStatus(ps: *mut PowerStatus) -> i32;
}

fn bateria_ler() -> Option<(u32, bool)> {
    unsafe {
        let mut ps = PowerStatus { ac_line: 0, battery_flag: 0, battery_percent: 0, _reservado: 0, _segundos_vida: 0, _segundos_cheia: 0 };
        if GetSystemPowerStatus(&mut ps) == 0 {
            return None;
        }
        if ps.battery_flag == 128 || ps.battery_flag == 0 {
            return None;
        }
        Some((ps.battery_percent as u32, ps.ac_line == 1))
    }
}

fn ram_usada_mb() -> (u64, u64) {
    let total = HardwareProfile::ram_mb();
    #[repr(C)]
    struct MemoryStatusEx2 {
        dw_length: u32,
        dw_memory_load: u32,
        ull_total_phys: u64,
        ull_avail_phys: u64,
    }
    extern "system" {
        fn GlobalMemoryStatusEx(buf: *mut MemoryStatusEx2) -> i32;
    }
    let mut ms = MemoryStatusEx2 {
        dw_length: std::mem::size_of::<MemoryStatusEx2>() as u32,
        dw_memory_load: 0,
        ull_total_phys: 0,
        ull_avail_phys: 0,
    };
    unsafe {
        if GlobalMemoryStatusEx(&mut ms) != 0 && ms.ull_total_phys > 0 {
            let total2 = ms.ull_total_phys / (1024 * 1024);
            let usada = total2.saturating_sub(ms.ull_avail_phys / (1024 * 1024));
            return (usada, total2);
        }
    }
    (0, total)
}

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
    modo: &'static str,
}

impl HardwareProfile {
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
        let (mut technique, total_ram_mb, fonte) = if let Some(ram) = Self::ram_do_relatorio() {
            let t = if ram < 8192 { RenderTechnique::Efficient } else { RenderTechnique::Full };
            (t, ram, "JSON")
        } else {
            log_msg("M6: relatorio.json ausente - fallback local");
            let ram = Self::ram_mb();
            let has_dx12 = Self::has_dx12_gpu();
            let t = if ram < 8192 || !has_dx12 { RenderTechnique::Efficient } else { RenderTechnique::Full };
            (t, ram, "LOCAL")
        };
        let mut modo: &'static str = "AUTO";
        if let Some(m) = config_carregar_modo() {
            if m == "EFFICIENT" {
                technique = RenderTechnique::Efficient;
                modo = "EFFICIENT";
            } else if m == "FULL" {
                technique = RenderTechnique::Full;
                modo = "FULL";
            }
            log_msg(&format!("M19: hibridos_config.json -> modo {}", modo));
        }
        HardwareProfile { technique, total_ram_mb, fonte, modo }
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

const CONFIG_PATH: &str = "C:/Users/Rodrigo/hybridos_config.json";
const NOTAS_PATH: &str = "C:/Users/Rodrigo/hybridos_notas.txt";

fn config_carregar_modo() -> Option<String> {
    let txt = std::fs::read_to_string(CONFIG_PATH).ok()?;
    let pos = txt.find("\"tecnica\"")?;
    let depois = &txt[pos + 9..];
    let depois = depois.trim_start_matches(|c: char| c == ':' || c == ' ' || c == '\t' || c == '\r' || c == '\n');
    let depois = depois.strip_prefix('"')?;
    let fim = depois.find('"')?;
    let valor = &depois[..fim];
    if valor == "EFFICIENT" || valor == "FULL" || valor == "AUTO" {
        Some(valor.to_string())
    } else {
        None
    }
}

fn config_salvar_modo(modo: &str) {
    let conteudo = format!("{{\n  \"tecnica\": \"{}\"\n}}\n", modo);
    if let Ok(mut f) = std::fs::File::create(CONFIG_PATH) {
        let _ = f.write_all(conteudo.as_bytes());
        log_msg(&format!("M19: config salva -> {}", modo));
    }
}

const DOCK_BG: u32 = 0xE6303046;
const ICON: u32 = 0xFFF84808;
const ICON2: u32 = 0xFF89B4FA;
const ICON3: u32 = 0xFFA6E3A1;
const ICON4: u32 = 0xFFF9E2AF;
const ITEM_BASE: i32 = 52;
const ICON5: u32 = 0xFFCBA6F7;
const ICON6: u32 = 0xFFFAB387;
const ICON7: u32 = 0xFF94E2D0;
const DOCK_H: i32 = 84;

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

struct Dock {
    items: Vec<u32>,
    mouse_x: f32,
    hover: i32,
}

impl Dock {
    fn new() -> Self {
        Dock { items: vec![ICON, ICON2, ICON3, ICON4, ICON5, ICON6, ICON7, 0xFFF5E0DC], mouse_x: -1.0, hover: -1 }
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

struct Spotlight {
    aberto: bool,
    query: String,
}

impl Spotlight {
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

const SPOT_CATALOG: &[(&str, &str)] = &[
    ("RESET", "C:\\Users\\Rodrigo\\ResetExplode\\ResetExplode.exe"),
    ("ARQUIVOS", "@ARQUIVOS"),
    ("TERMINAL", "@TERMINAL"),
    ("CONFIG", "@CONFIG"),
    ("NOTAS", "@NOTAS"),
    ("CALC", "@CALC"),
    ("SAIR", "@SAIR"),
];

const DESKTOP_APPS: [&str; 6] = ["RESET", "ARQUIVOS", "TERMINAL", "CONFIG", "NOTAS", "CALC"];
const DESKTOP_ICON_COR: [u32; 6] = [0xFFF84808, 0xFF89B4FA, 0xFFA6E3A1, 0xFFCBA6F7, 0xFFFAB387, 0xFF94E2D0];

// M30: hit da luz verde de maximizar (usada por todos os apps nativos)
fn luz_max_hit(wx: i32, wy: i32, ww: i32, mx: i32, my: i32) -> bool {
    let cx = wx + ww - 54;
    let cy = wy + 15;
    let ddx = (mx - cx) as f32;
    let ddy = (my - cy) as f32;
    (ddx * ddx + ddy * ddy).sqrt() <= 8.0
}

fn desktop_icon_hit(mx: i32, my: i32) -> i32 {
    let x0 = 24;
    let y0 = MENU_H + 24;
    let step = 84;
    for (i, _) in DESKTOP_APPS.iter().enumerate() {
        let iy = y0 + i as i32 * step;
        if mx >= x0 - 6 && mx <= x0 + 58 && my >= iy - 6 && my <= iy + 76 {
            return i as i32;
        }
    }
    -1
}

fn agora_hhmm() -> (u32, u32) {
    let ok = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH);
    let secs = ok.map(|d| d.as_secs()).unwrap_or(0) as i64;
    let mins = (secs / 60 - 3 * 60).rem_euclid(1440);
    ((mins / 60) as u32, (mins % 60) as u32)
}

fn lerp_cor(a: u32, b: u32, t: f32) -> u32 {
    let ar = (a >> 16) & 0xFF;
    let ag = (a >> 8) & 0xFF;
    let ab = a & 0xFF;
    let br = (b >> 16) & 0xFF;
    let bg2 = (b >> 8) & 0xFF;
    let bb = b & 0xFF;
    let r = (ar as f32 + (br as f32 - ar as f32) * t) as u32;
    let g = (ag as f32 + (bg2 as f32 - ag as f32) * t) as u32;
    let bl = (ab as f32 + (bb as f32 - ab as f32) * t) as u32;
    0xFF000000 | (r << 16) | (g << 8) | bl
}

fn draw_sol(buf: &mut [u32], w: i32, h: i32, cx: i32, cy: i32, r: i32, color: u32) {
    for y in (cy - r - 1).max(0)..=(cy + r + 1).min(h - 1) {
        for x in (cx - r - 1).max(0)..=(cx + r + 1).min(w - 1) {
            let dx = (x - cx) as f32;
            let dy = (y - cy) as f32;
            if (dx * dx + dy * dy).sqrt() <= r as f32 {
                buf[(y * w + x) as usize] = color;
            }
        }
    }
}

fn draw_papel_parede(buf: &mut [u32], w: i32, h: i32) {
    let c_topo = 0xFF0F0F23;
    let c_meio = 0xFF2D2B55;
    let c_base = 0xFF8A4A3C;
    let meio_y = (h as f32 * 0.55) as i32;
    for y in 0..h {
        let cor = if y < meio_y {
            let t = y as f32 / meio_y as f32;
            lerp_cor(c_topo, c_meio, t)
        } else {
            let t = (y - meio_y) as f32 / (h - meio_y) as f32;
            lerp_cor(c_meio, c_base, t)
        };
        for x in 0..w {
            buf[(y * w + x) as usize] = cor;
        }
    }
    let sol_x = w * 3 / 4;
    let sol_y = h * 2 / 3;
    draw_sol(buf, w, h, sol_x, sol_y, h / 8, 0x30FFB86C);
    draw_sol(buf, w, h, sol_x, sol_y, h / 14, 0x50FFD9A0);
}

fn w_center(win: &Option<Rc<Window>>) -> i32 {
    win.as_ref().map(|w| w.inner_size().width as i32).unwrap_or(1024)
}

fn h_center(win: &Option<Rc<Window>>) -> i32 {
    win.as_ref().map(|w| w.inner_size().height as i32).unwrap_or(768)
}

fn glyph_livre(ch: char) -> [u8; 7] {
    match ch {
        'A' => [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'Á' => [0b00100, 0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001],
        'À' => [0b01000, 0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001],
        'Â' => [0b01010, 0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001],
        'Ã' => [0b01010, 0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001],
        'B' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110],
        'C' => [0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110],
        'Ç' => [0b01110, 0b10001, 0b10000, 0b10000, 0b10001, 0b01110, 0b00100],
        'D' => [0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110],
        'E' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
        'É' => [0b00100, 0b11111, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
        'Ê' => [0b01010, 0b11111, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
        'F' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000],
        'G' => [0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01110],
        'H' => [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'I' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111],
        'Í' => [0b00100, 0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111],
        'J' => [0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100],
        'K' => [0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001],
        'L' => [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111],
        'M' => [0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001],
        'N' => [0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001],
        'O' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'Ó' => [0b00100, 0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'Ô' => [0b01010, 0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'Õ' => [0b01010, 0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'P' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000],
        'Q' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101],
        'R' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
        'S' => [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110],
        'T' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
        'U' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'Ú' => [0b00100, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'V' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100],
        'W' => [0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001],
        'X' => [0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001],
        'Y' => [0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100],
        'Z' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111],
        '0' => [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110],
        '1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        '2' => [0b01110, 0b10001, 0b00001, 0b00110, 0b01000, 0b10000, 0b11111],
        '3' => [0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110],
        '4' => [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
        '5' => [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110],
        '6' => [0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110],
        '7' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
        '8' => [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
        '9' => [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100],
        ' ' => [0, 0, 0, 0, 0, 0, 0],
        '.' => [0, 0, 0, 0, 0, 0b00110, 0b00110],
        ':' => [0, 0b00110, 0b00110, 0, 0b00110, 0b00110, 0],
        '/' => [0b00001, 0b00010, 0b00010, 0b00100, 0b01000, 0b01000, 0b10000],
        '-' => [0, 0, 0, 0b11111, 0, 0, 0],
        '>' => [0b01000, 0b00100, 0b00010, 0b00001, 0b00010, 0b00100, 0b01000],
        '<' => [0b00010, 0b00100, 0b01000, 0b10000, 0b01000, 0b00100, 0b00010],
        '_' => [0, 0, 0, 0, 0, 0, 0b11111],
        _ => [0, 0, 0, 0, 0, 0, 0],
    }
}

struct Toast {
    titulo: String,
    corpo: String,
    nasceu: u32,
}

struct Notificacoes {
    fila: Vec<Toast>,
}

impl Notificacoes {
    fn novo() -> Self {
        Notificacoes { fila: Vec::new() }
    }

    fn disparar(&mut self, frame: u32, titulo: &str, corpo: &str) {
        if self.fila.len() >= 3 {
            self.fila.remove(0);
        }
        self.fila.push(Toast { titulo: titulo.to_string(), corpo: corpo.to_string(), nasceu: frame });
        log_msg(&format!("toast: {} - {}", titulo, corpo));
    }

    fn podar(&mut self, frame: u32) {
        self.fila.retain(|t| frame.wrapping_sub(t.nasceu) < 240);
    }

    fn draw(&self, buf: &mut [u32], w: i32, h: i32, frame: u32) {
        const TW: i32 = 280;
        const TH: i32 = 58;
        for (k, t) in self.fila.iter().enumerate() {
            let k = k as i32;
            let idade = frame.wrapping_sub(t.nasceu) as i32;
            let offset = if idade < 20 { (20 - idade) * TW / 20 } else if idade > 220 { (idade - 220) * TW / 20 } else { 0 };
            let x1 = w - 16 - offset;
            let y1 = MENU_H + 12 + k * (TH + 10);
            Self::rr(buf, w, h, x1 + 3, y1 + 3, TW, TH, 10, 0x66101014);
            Self::rr(buf, w, h, x1, y1, TW, TH, 10, 0xF2313244);
            Self::rr(buf, w, h, x1, y1, 4, TH, 2, 0xFFF84808);
            let maxn = ((TW - 30) / 6) as usize;
            let mut tit = t.titulo.clone();
            if tit.len() > maxn { tit = tit[..maxn].to_string(); }
            let mut cor_ = t.corpo.clone();
            if cor_.len() > maxn { cor_ = cor_[..maxn].to_string(); }
            Self::texto(buf, w, h, x1 + 14, y1 + 10, &tit, 1, 0xFFCDD6F4);
            Self::texto(buf, w, h, x1 + 14, y1 + 30, &cor_, 1, 0xFFA6ADC8);
        }
    }

    fn rr(buf: &mut [u32], w: i32, h: i32, x0: i32, y0: i32, rw: i32, rh: i32, rad: i32, color: u32) {
        for y in y0.max(0)..=(y0 + rh).min(h - 1) {
            for x in x0.max(0)..=(x0 + rw).min(w - 1) {
                let in_cx = x < x0 + rad || x >= x0 + rw - rad;
                let in_cy = y < y0 + rad || y >= y0 + rh - rad;
                if in_cx && in_cy {
                    let ccx = if x < x0 + rad { x0 + rad } else { x0 + rw - rad - 1 };
                    let ccy = if y < y0 + rad { y0 + rad } else { y0 + rh - rad - 1 };
                    let dx = (x - ccx) as f32;
                    let dy = (y - ccy) as f32;
                    if (dx * dx + dy * dy).sqrt() > rad as f32 { continue; }
                }
                buf[(y * w + x) as usize] = color;
            }
        }
    }

    fn texto(buf: &mut [u32], w: i32, h: i32, x0: i32, y0: i32, text: &str, scale: i32, color: u32) {
        let mut x = x0;
        for c in text.to_uppercase().chars() {
            let g = glyph_livre(c);
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
}

struct Terminal {
    aberto: bool,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    cwd: String,
    linhas: Vec<String>,
    input: String,
    arrastando: bool,
    maximizada: bool,
    pre_max: (i32, i32, i32, i32),
    dx: i32,
    dy: i32,
}

impl Terminal {
    fn novo() -> Self {
        Terminal { aberto: false, x: 180, y: 60, w: 560, h: 400, cwd: "C:/Users/Rodrigo".to_string(), linhas: vec!["HYBRID OS TERMINAL - DIGITE HELP".to_string()], input: String::new(), arrastando: false, maximizada: false, pre_max: (0, 0, 0, 0), dx: 0, dy: 0 }
    }

    fn abrir(&mut self) {
        self.aberto = true;
    }

    fn print(&mut self, s: &str) {
        for parte in s.split('\n') {
            self.linhas.push(parte.to_string());
        }
        if self.linhas.len() > 200 {
            let corte = self.linhas.len() - 200;
            self.linhas.drain(0..corte);
        }
    }

    fn executar(&mut self, cmd: &str) {
        let cmd = cmd.trim().to_string();
        self.print(&format!("> {}", cmd));
        if cmd.is_empty() { return; }
        let partes: Vec<String> = cmd.split_whitespace().map(|s| s.to_string()).collect();
        let c0 = partes[0].to_uppercase();
        match c0.as_str() {
            "HELP" => {
                self.print("COMANDOS:");
                self.print("  LS           - lista a pasta atual");
                self.print("  CD <pasta>   - entra (CD .. sobe)");
                self.print("  CAT <arq>    - mostra texto do arquivo");
                self.print("  SYSINFO      - RAM e fonte de hardware");
                self.print("  OPEN RESET   - abre o ResetExplode");
                self.print("  DATE         - hora do sistema");
                self.print("  ECHO <txt>   - repete o texto");
                self.print("  CLEAR        - limpa a tela");
            }
            "LS" | "DIR" => {
                match std::fs::read_dir(&self.cwd) {
                    Ok(rd) => {
                        let mut nomes: Vec<(String, bool)> = Vec::new();
                        for e in rd.flatten() {
                            if let Some(n) = e.file_name().to_str() {
                                let p = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
                                nomes.push((n.to_string(), p));
                            }
                        }
                        nomes.sort();
                        if nomes.is_empty() { self.print("(vazio)"); }
                        for (n, p) in nomes {
                            self.print(&format!("  {}{}", if p { "[DIR] " } else { "      " }, n));
                        }
                    }
                    Err(e) => self.print(&format!("erro: {}", e)),
                }
            }
            "CD" => {
                if partes.len() < 2 {
                    self.print(&self.cwd.clone());
                } else {
                    let alvo = partes[1].clone();
                    let novo = if alvo == ".." {
                        match self.cwd.rfind('/') {
                            Some(pos) if self.cwd.len() > 3 => self.cwd[..pos].to_string(),
                            _ => self.cwd.clone(),
                        }
                    } else if alvo.contains(':') || alvo.starts_with('/') {
                        alvo.replace('\\', "/")
                    } else {
                        format!("{}/{}", self.cwd.trim_end_matches('/'), alvo.replace('\\', "/"))
                    };
                    if std::fs::read_dir(&novo).is_ok() {
                        self.cwd = novo;
                        self.print(&format!("cwd: {}", self.cwd));
                    } else {
                        self.print(&format!("pasta não existe: {}", novo));
                    }
                }
            }
            "CAT" | "TYPE" => {
                if partes.len() < 2 {
                    self.print("uso: CAT <arquivo>");
                } else {
                    let alvo = partes[1].replace('\\', "/");
                    let caminho = if alvo.contains(':') { alvo.clone() } else { format!("{}/{}", self.cwd.trim_end_matches('/'), alvo) };
                    match std::fs::read_to_string(&caminho) {
                        Ok(txt) => {
                            let linhas: Vec<&str> = txt.lines().take(20).collect();
                            for l in linhas { self.print(&format!("  {}", l)); }
                            let total = txt.lines().count();
                            if total > 20 { self.print(&format!("  ... ({} linhas no total)", total)); }
                        }
                        Err(e) => self.print(&format!("erro: {}", e)),
                    }
                }
            }
            "SYSINFO" => {
                let prof = HardwareProfile::detect();
                self.print(&format!("  RAM: {} MB", prof.total_ram_mb));
                self.print(&format!("  FONTE: {}", prof.fonte));
                self.print(&format!("  TECNICA: {:?}", prof.technique));
            }
            "OPEN" => {
                if partes.len() >= 2 && partes[1].to_uppercase() == "RESET" {
                    let _ = std::process::Command::new("C:\\Users\\Rodrigo\\ResetExplode\\ResetExplode.exe").spawn();
                    self.print("abrindo ResetExplode...");
                } else if partes.len() >= 2 && partes[1].to_uppercase() == "ARQUIVOS" {
                    self.print("dica: ARQUIVOS abre pela dock (pasta)");
                } else {
                    self.print("uso: OPEN RESET");
                }
            }
            "DATE" => {
                let (hh, mm) = agora_hhmm();
                self.print(&format!("  {:02}:{:02} (UTC-3)", hh, mm));
            }
            "ECHO" => {
                self.print(&partes[1..].join(" "));
            }
            "CLEAR" | "CLS" => {
                self.linhas.clear();
            }
            "SAIR" | "EXIT" => {
                self.print("dica: SAIR na dock fecha o shell");
            }
            _ => self.print(&format!("comando desconhecido: {} (digite HELP)", c0)),
        }
    }

    fn tecla(&mut self, ch: char) {
        if ch == '\u{8}' {
            self.input.pop();
            return;
        }
        if ch == '\n' || ch == '\r' {
            let cmd = self.input.clone();
            self.input.clear();
            self.executar(&cmd);
            return;
        }
        if ch.is_ascii_graphic() || ch == ' ' {
            self.input.push(ch);
        }
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

struct CalcApp {
    aberto: bool,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    arrastando: bool,
    dx: i32,
    dy: i32,
    display: String,
    acc: f64,
    op: char,
    digitando: bool,
    maximizada: bool,
    pre_max: (i32, i32, i32, i32),
}

impl CalcApp {
    fn novo() -> Self {
        CalcApp { aberto: false, x: 340, y: 110, w: 300, h: 400, arrastando: false, dx: 0, dy: 0, display: "0".to_string(), acc: 0.0, op: ' ', digitando: true, maximizada: false, pre_max: (0, 0, 0, 0) }
    }

    fn abrir(&mut self) {
        self.aberto = true;
    }

    fn limpar(&mut self) {
        self.display = "0".to_string();
        self.acc = 0.0;
        self.op = ' ';
        self.digitando = true;
    }

    fn aplicar(&mut self, b: f64) -> f64 {
        match self.op {
            '+' => self.acc + b,
            '-' => self.acc - b,
            'x' | '*' => self.acc * b,
            '/' => if b != 0.0 { self.acc / b } else { 0.0 },
            _ => b,
        }
    }

    fn tecla_calc(&mut self, tecla: char) {
        if tecla.is_ascii_digit() {
            if !self.digitando {
                self.display.clear();
                self.digitando = true;
            }
            if self.display.len() < 12 {
                if self.display == "0" {
                    self.display.clear();
                }
                self.display.push(tecla);
            }
            return;
        }
        if tecla == '.' {
            if !self.digitando {
                self.display.clear();
                self.digitando = true;
            }
            if !self.display.contains('.') {
                self.display.push('.');
            }
            return;
        }
        if tecla == '+' || tecla == '-' || tecla == 'x' || tecla == '*' || tecla == '/' {
            let atual = self.display.parse::<f64>().unwrap_or(0.0);
            if self.op != ' ' && !self.digitando {
                self.acc = self.aplicar(atual);
            } else {
                self.acc = atual;
            }
            self.op = tecla;
            self.digitando = false;
            self.display = format!("{}", self.acc);
            return;
        }
        if tecla == '=' {
            let atual = self.display.parse::<f64>().unwrap_or(0.0);
            if self.op != ' ' {
                let r = self.aplicar(atual);
                self.display = format!("{}", r);
                self.acc = r;
                self.op = ' ';
                self.digitando = false;
            }
            return;
        }
        if tecla == 'C' || tecla == 'c' {
            self.limpar();
        }
    }

    fn layout_botoes(&self) -> Vec<(char, i32, i32, i32, i32)> {
        let rotulos: Vec<(char, i32, i32)> = vec![
            ('C', 0, 0), ('/', 1, 0), ('x', 2, 0), ('-', 3, 0),
            ('7', 0, 1), ('8', 1, 1), ('9', 2, 1), ('+', 3, 1),
            ('4', 0, 2), ('5', 1, 2), ('6', 2, 2), ('+', 3, 2),
            ('1', 0, 3), ('2', 1, 3), ('3', 2, 3), ('=', 3, 3),
            ('0', 0, 4), ('.', 1, 4), ('=', 2, 4),
        ];
        let bw = (self.w - 40) / 4;
        let bh = 48;
        let y0 = self.y + 90;
        let x0 = self.x + 20;
        let mut v = Vec::new();
        for (ch, col, lin) in rotulos {
            let bx = x0 + col * bw;
            let by = y0 + lin * bh;
            v.push((ch, bx, by, bw - 6, bh - 6));
        }
        v
    }

    fn clique(&mut self, mx: i32, my: i32) -> bool {
        for (ch, bx, by, bw, bh) in self.layout_botoes() {
            if mx >= bx && mx <= bx + bw && my >= by && my <= by + bh {
                self.tecla_calc(ch);
                return true;
            }
        }
        false
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

struct NotasApp {
    aberto: bool,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    arrastando: bool,
    dx: i32,
    dy: i32,
    linhas: Vec<String>,
    sujo: bool,
    maximizada: bool,
    pre_max: (i32, i32, i32, i32),
}

impl NotasApp {
    fn novo() -> Self {
        NotasApp { aberto: false, x: 240, y: 90, w: 520, h: 360, arrastando: false, dx: 0, dy: 0, linhas: vec![String::new()], sujo: false, maximizada: false, pre_max: (0, 0, 0, 0) }
    }

    fn abrir(&mut self) {
        if let Ok(txt) = std::fs::read_to_string(NOTAS_PATH) {
            self.linhas = txt.lines().map(|s| s.to_string()).collect();
            if self.linhas.is_empty() {
                self.linhas = vec![String::new()];
            }
        } else {
            self.linhas = vec![String::new()];
        }
        self.sujo = false;
        self.aberto = true;
    }

    fn salvar(&mut self) {
        let conteudo = self.linhas.join("\n");
        if let Ok(mut f) = std::fs::File::create(NOTAS_PATH) {
            let _ = f.write_all(conteudo.as_bytes());
            self.sujo = false;
            log_msg("M20: notas salvas");
        }
    }

    fn tecla(&mut self, ch: char) {
        if ch == '\u{8}' {
            let n = self.linhas.len();
            if n > 0 && !self.linhas[n - 1].is_empty() {
                self.linhas[n - 1].pop();
            } else if n > 1 {
                let linha = self.linhas.pop().unwrap_or_default();
                let ultima = n - 2;
                self.linhas[ultima].push_str(&linha);
            }
            self.sujo = true;
            return;
        }
        if ch == '\n' || ch == '\r' {
            self.linhas.push(String::new());
            if self.linhas.len() > 200 {
                self.linhas.remove(0);
            }
            self.sujo = true;
            return;
        }
        if ch.is_ascii_graphic() || ch == ' ' {
            let n = self.linhas.len();
            if n == 0 {
                self.linhas.push(String::new());
            }
            let n = self.linhas.len();
            self.linhas[n - 1].push(ch);
            self.sujo = true;
        }
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

struct ConfigApp {
    aberto: bool,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    arrastando: bool,
    maximizada: bool,
    pre_max: (i32, i32, i32, i32),
    dx: i32,
    dy: i32,
}

impl ConfigApp {
    fn novo() -> Self {
        ConfigApp { aberto: false, x: 300, y: 120, w: 480, h: 330, arrastando: false, maximizada: false, pre_max: (0, 0, 0, 0), dx: 0, dy: 0 }
    }

    fn abrir(&mut self) {
        self.aberto = true;
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

struct FileBrowser {
    aberto: bool,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    caminho: String,
    entradas: Vec<(String, bool)>,
    selecionado: usize,
    scroll: usize,
    arrastando: bool,
    maximizada: bool,
    pre_max: (i32, i32, i32, i32),
    dx: i32,
    dy: i32,
}

impl FileBrowser {
    fn novo() -> Self {
        FileBrowser { aberto: false, x: 160, y: 70, w: 460, h: 380, caminho: String::new(), entradas: Vec::new(), selecionado: 0, scroll: 0, arrastando: false, maximizada: false, pre_max: (0, 0, 0, 0), dx: 0, dy: 0 }
    }

    fn abrir(&mut self, caminho: &str) {
        self.caminho = caminho.to_string();
        self.recargar();
        self.aberto = true;
        self.selecionado = 0;
        self.scroll = 0;
    }

    fn recargar(&mut self) {
        self.entradas.clear();
        let mut pastas: Vec<String> = Vec::new();
        let mut arqs: Vec<String> = Vec::new();
        if let Ok(rd) = std::fs::read_dir(&self.caminho) {
            for e in rd.flatten() {
                if let Some(nome) = e.file_name().to_str() {
                    if nome.starts_with('.') { continue; }
                    let pasta = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
                    if pasta { pastas.push(nome.to_string()); } else { arqs.push(nome.to_string()); }
                }
            }
        }
        pastas.sort();
        arqs.sort();
        for p in pastas { self.entradas.push((p, true)); }
        for a in arqs { self.entradas.push((a, false)); }
    }

    fn mover(&mut self, delta: i32) {
        if self.entradas.is_empty() { return; }
        let n = self.entradas.len();
        let sel = self.selecionado as i32 + delta;
        self.selecionado = sel.clamp(0, n as i32 - 1) as usize;
        let vis = (((self.h - 90) / 16) as usize).max(1);
        if self.selecionado < self.scroll { self.scroll = self.selecionado; }
        if self.selecionado >= self.scroll + vis { self.scroll = self.selecionado + 1 - vis; }
    }

    fn entrar_sel(&mut self) {
        if let Some((nome, pasta)) = self.entradas.get(self.selecionado) {
            let nome = nome.clone();
            if *pasta {
                let novo = format!("{}/{}", self.caminho.trim_end_matches('/'), nome);
                self.abrir(&novo);
                log_msg(&format!("arquivos: entrou {}", novo));
            } else {
                log_msg(&format!("arquivos: arquivo {}", nome));
            }
        }
    }

    fn subir(&mut self) {
        if let Some(pos) = self.caminho.rfind('/') {
            let pai = self.caminho[..pos].to_string();
            if pai.len() >= 2 {
                self.abrir(&pai);
                log_msg(&format!("arquivos: subiu {}", pai));
            }
        }
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

struct Janela {
    aberta: bool,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    arrastando: bool,
    redimensionando: bool,
    minimizada: bool,
    maximizada: bool,
    pre_max: (i32, i32, i32, i32),
    dx: i32,
    dy: i32,
}

impl Janela {
    fn new() -> Self {
        Janela { aberta: true, x: 120, y: 90, w: 420, h: 260, arrastando: false, redimensionando: false, minimizada: false, maximizada: false, pre_max: (0, 0, 0, 0), dx: 0, dy: 0 }
    }

    fn hit_min(&self, mx: i32, my: i32) -> bool {
        let cx = self.x + self.w - 36;
        let cy = self.y + 15;
        let ddx = (mx - cx) as f32;
        let ddy = (my - cy) as f32;
        (ddx * ddx + ddy * ddy).sqrt() <= 8.0
    }

    fn hit_max(&self, mx: i32, my: i32) -> bool {
        let cx = self.x + self.w - 54;
        let cy = self.y + 15;
        let ddx = (mx - cx) as f32;
        let ddy = (my - cy) as f32;
        (ddx * ddx + ddy * ddy).sqrt() <= 8.0
    }

    fn hit_alca(&self, mx: i32, my: i32) -> bool {
        let ax = self.x + self.w - 14;
        let ay = self.y + self.h - 14;
        mx >= ax && mx <= self.x + self.w && my >= ay && my <= self.y + self.h
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