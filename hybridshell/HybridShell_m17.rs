// ============================================================
// HYBRID OS - CAMADA B / MARCO M17: SPOTLIGHT GLOBAL
// M16 + Ctrl+Espaço funciona de QUALQUER lugar do sistema
// (GetAsyncKeyState polling a 60fps, com cooldown anti-duplo).
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

// ---------- Teclado global (M17: GetAsyncKeyState, leitura de estado) ----------
extern "system" {
    fn GetAsyncKeyState(vkey: i32) -> i16;
}

const VK_CONTROL: i32 = 0x11;
const VK_SPACE: i32 = 0x20;

// retorna true se a tecla esta pressionada AGORA
fn tecla_pressionada(vkey: i32) -> bool {
    unsafe { GetAsyncKeyState(vkey) as u16 & 0x8000 != 0 }
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
        Dock { items: vec![ICON, ICON2, ICON3, ICON4, 0xFF94E2D0], mouse_x: -1.0, hover: -1 }
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

// Catalogo de apps (@ = interno do shell)
const SPOT_CATALOG: &[(&str, &str)] = &[
    ("RESET", "C:\\Users\\Rodrigo\\ResetExplode\\ResetExplode.exe"),
    ("ARQUIVOS", "@ARQUIVOS"),
    ("TERMINAL", "@TERMINAL"),
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

// ---------- Papel de parede gradiente (M12, estilo Big Sur) ----------
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

// Sol suave no horizonte (circulos translucidos empilhados)
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

// ---------- Helpers de centro da tela (M9) ----------
fn w_center(win: &Option<Rc<Window>>) -> i32 {
    win.as_ref().map(|w| w.inner_size().width as i32).unwrap_or(1024)
}

fn h_center(win: &Option<Rc<Window>>) -> i32 {
    win.as_ref().map(|w| w.inner_size().height as i32).unwrap_or(768)
}

// Tabela unica de glifos 5x7 (usada pelo shell e pelas notificacoes)
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

// ---------- Notificacoes (M16, toasts estilo macOS) ----------
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
        // max 3 na fila - a mais velha sai
        if self.fila.len() >= 3 {
            self.fila.remove(0);
        }
        self.fila.push(Toast { titulo: titulo.to_string(), corpo: corpo.to_string(), nasceu: frame });
        log_msg(&format!("toast: {} - {}", titulo, corpo));
    }

    // remove as que venceram (240 frames = 4s a 60fps)
    fn podar(&mut self, frame: u32) {
        self.fila.retain(|t| frame.wrapping_sub(t.nasceu) < 240);
    }

    fn draw(&self, buf: &mut [u32], w: i32, h: i32, frame: u32) {
        const TW: i32 = 280;
        const TH: i32 = 58;
        for (k, t) in self.fila.iter().enumerate() {
            let k = k as i32;
            // animacao: desliza da direita nos primeiros 20 frames, some nos ultimos 20
            let idade = frame.wrapping_sub(t.nasceu) as i32;
            let offset = if idade < 20 { (20 - idade) * TW / 20 } else if idade > 220 { (idade - 220) * TW / 20 } else { 0 };
            let x1 = w - 16 - offset;
            let y1 = MENU_H + 12 + k * (TH + 10);
            // sombra + corpo + barra colorida
            Self::rr(buf, w, h, x1 + 3, y1 + 3, TW, TH, 10, 0x66101014);
            Self::rr(buf, w, h, x1, y1, TW, TH, 10, 0xF2313244);
            Self::rr(buf, w, h, x1, y1, 4, TH, 2, 0xFFF84808);
            // titulo + corpo (truncados)
            let maxn = ((TW - 30) / 6) as usize;
            let mut tit = t.titulo.clone();
            if tit.len() > maxn { tit = tit[..maxn].to_string(); }
            let mut cor_ = t.corpo.clone();
            if cor_.len() > maxn { cor_ = cor_[..maxn].to_string(); }
            Self::texto(buf, w, h, x1 + 14, y1 + 10, &tit, 1, 0xFFCDD6F4);
            Self::texto(buf, w, h, x1 + 14, y1 + 30, &cor_, 1, 0xFFA6ADC8);
        }
    }

    // wrappers estaticos (mesmas primitivas do shell)
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

// ---------- App TERMINAL nativo (M15) ----------
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
    dx: i32,
    dy: i32,
}

impl Terminal {
    fn novo() -> Self {
        Terminal { aberto: false, x: 180, y: 60, w: 560, h: 400, cwd: "C:/Users/Rodrigo".to_string(), linhas: vec!["HYBRID OS TERMINAL - DIGITE HELP".to_string()], input: String::new(), arrastando: false, dx: 0, dy: 0 }
    }

    fn abrir(&mut self) {
        self.aberto = true;
    }

    fn print(&mut self, s: &str) {
        for parte in s.split('\n') {
            self.linhas.push(parte.to_string());
        }
        // mantem as ultimas 200 linhas
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

// ---------- App ARQUIVOS nativo (M14) ----------
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
    dx: i32,
    dy: i32,
}

impl FileBrowser {
    fn novo() -> Self {
        FileBrowser { aberto: false, x: 160, y: 70, w: 460, h: 380, caminho: String::new(), entradas: Vec::new(), selecionado: 0, scroll: 0, arrastando: false, dx: 0, dy: 0 }
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
        // ajusta scroll pra manter visivel (vis = linhas que cabem)
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
    fb: FileBrowser,
    term: Terminal,
    notif: Notificacoes,
    mission: bool,
    mission_rects: Vec<(usize, i32, i32, i32, i32)>,
    menu_aberto: bool,
    menu_drop_hover: i32,
    hotkey_prev: bool,
    hotkey_cooldown: u32,
    mouse: (i32, i32),
    window: Option<Rc<Window>>,
    surface: Option<Surface<Rc<Window>, Rc<Window>>>,
    frame: u32,
}

impl ShellApp {
    fn new(profile: HardwareProfile) -> Self {
        ShellApp { profile, dock: Dock::new(), menu: MenuBar::new(), spot: Spotlight { aberto: false, query: String::new() }, janelas: Vec::new(), fb: FileBrowser::novo(), term: Terminal::novo(), notif: Notificacoes::novo(), mission: false, mission_rects: Vec::new(), menu_aberto: false, menu_drop_hover: -1, hotkey_prev: false, hotkey_cooldown: 0, mouse: (0, 0), window: None, surface: None, frame: 0 }
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

    // Fonte bitmap 5x7 (tabela unica com acentos pt-BR)
    fn glyph(ch: char) -> [u8; 7] {
        glyph_livre(ch)
    }

    fn draw_text(buf: &mut [u32], w: i32, h: i32, x0: i32, y0: i32, text: &str, scale: i32, color: u32) {
        let mut x = x0;
        for c in text.to_uppercase().chars() {
            let g = Self::glyph(c);
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

        draw_papel_parede(&mut buf, w, h);

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
            let dica = format!("{} JANELAS - CLIQUE PARA FOCAR / ESC VOLTA", nj);
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
            Self::draw_text(&mut buf, w, h, jx + 20, jy + 170, "CLIQUE PARA TRAZER PARA FRENTE", 1, 0xFF6C7086);
            Self::draw_text(&mut buf, w, h, jx + 20, jy + 190, "ESC OU X FECHA", 1, 0xFF6C7086);
        }
        }

        // ===== APP TERMINAL NATIVO (M15) =====
        if self.term.aberto {
            let tx = self.term.x;
            let ty = self.term.y;
            let tw = self.term.w;
            let th = self.term.h;
            Self::draw_rounded_rect(&mut buf, w, h, tx + 5, ty + 5, tw, th, 12, 0x66101014);
            Self::draw_rounded_rect(&mut buf, w, h, tx, ty, tw, th, 12, 0xFF181825);
            Self::draw_rounded_rect(&mut buf, w, h, tx, ty, tw, 30, 12, 0xFF313244);
            Self::draw_text(&mut buf, w, h, tx + 14, ty + 10, "TERMINAL", 2, 0xFFCDD6F4);
            Self::draw_circle(&mut buf, w, h, tx + tw - 18, ty + 15, 7, 0xFFF38BA8);
            Self::draw_text(&mut buf, w, h, tx + tw - 23, ty + 11, "X", 1, 0xFF1E1E2E);
            // linhas visiveis (scroll automatico pro fim)
            let vis = ((th - 80) / 14).max(1) as usize;
            let total = self.term.linhas.len();
            let inicio = total.saturating_sub(vis - 1);
            for (vi, idx) in (inicio..total).enumerate() {
                let mut linha = self.term.linhas[idx].clone();
                let maxn = ((tw - 40) / 6) as usize;
                if linha.len() > maxn { linha = linha[..maxn].to_string(); }
                Self::draw_text(&mut buf, w, h, tx + 16, ty + 42 + vi as i32 * 14, &linha, 1, 0xFFA6E3A1);
            }
            // linha de input com cursor
            let mut inp = format!("{} {}", self.term.cwd, self.term.input);
            if (self.frame / 30) % 2 == 0 { inp.push('_'); }
            if inp.len() > ((tw - 40) / 6) as usize {
                let take = ((tw - 40) / 6) as usize;
                inp = inp[inp.len() - take..].to_string();
            }
            Self::draw_text(&mut buf, w, h, tx + 16, ty + th - 28, &inp, 1, 0xFFCDD6F4);
        }

        // ===== APP ARQUIVOS NATIVO (M14) =====
        if self.fb.aberto {
            let fx = self.fb.x;
            let fy = self.fb.y;
            let fw = self.fb.w;
            let fh = self.fb.h;
            Self::draw_rounded_rect(&mut buf, w, h, fx + 5, fy + 5, fw, fh, 12, 0x66101014);
            Self::draw_rounded_rect(&mut buf, w, h, fx, fy, fw, fh, 12, 0xFF313244);
            Self::draw_rounded_rect(&mut buf, w, h, fx, fy, fw, 30, 12, 0xFF45475A);
            Self::draw_text(&mut buf, w, h, fx + 14, fy + 10, "ARQUIVOS", 2, 0xFFCDD6F4);
            Self::draw_circle(&mut buf, w, h, fx + fw - 18, fy + 15, 7, 0xFFF38BA8);
            Self::draw_text(&mut buf, w, h, fx + fw - 23, fy + 11, "X", 1, 0xFF1E1E2E);
            // caminho atual (truncado se longo)
            let mut cam = self.fb.caminho.clone();
            let maxc = (fw - 40) / 12;
            if cam.len() > maxc as usize {
                let take = maxc as usize;
                cam = format!("/{}", &cam[cam.len() - take..]);
            }
            Self::draw_text(&mut buf, w, h, fx + 14, fy + 40, &cam, 1, 0xFFA6ADC8);
            // entradas visiveis
            let vis = ((fh - 90) / 16).max(1) as usize;
            for vi in 0..vis {
                let idx = self.fb.scroll + vi;
                if idx >= self.fb.entradas.len() { break; }
                let (nome, pasta) = &self.fb.entradas[idx];
                let ly = fy + 62 + vi as i32 * 16;
                let sel = idx == self.fb.selecionado;
                if sel {
                    Self::draw_rounded_rect(&mut buf, w, h, fx + 8, ly - 2, fw - 16, 15, 3, 0xFF45475A);
                }
                let cor = if *pasta { 0xFF89B4FA } else { 0xFFCDD6F4 };
                let mut rot = if *pasta { format!("[{}]", nome) } else { nome.clone() };
                let maxn = ((fw - 50) / 6) as usize;
                if rot.len() > maxn { rot = rot[..maxn].to_string(); }
                Self::draw_text(&mut buf, w, h, fx + 18, ly, &rot, 1, cor);
            }
            let dica = "SETAS NAVEGAM / ENTER ABRE / BACKSPACE SOBE / ESC FECHA";
            Self::draw_text(&mut buf, w, h, fx + 14, fy + fh - 22, dica, 1, 0xFF6C7086);
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
                3 => {
                    // M15: icone TERMINAL = prompt >_
                    Self::draw_line(&mut buf, w, h, cx - r / 2, cy - r / 4, cx - r / 6, cy, r / 3, 0xFF1E1E2E);
                    Self::draw_line(&mut buf, w, h, cx - r / 6, cy, cx - r / 2, cy + r / 4, r / 3, 0xFF1E1E2E);
                    Self::draw_line(&mut buf, w, h, cx, cy + r / 4, cx + r / 2, cy + r / 4, r / 3, 0xFF1E1E2E);
                }
                4 => Self::draw_icone_x(&mut buf, w, h, cx, cy, r * 2 / 3, 0xFFE6E9F0),
                _ => {}
            }
        }

        // nome do app em hover
        let nomes = ["SISTEMA", "RESET", "ARQUIVOS", "TERMINAL", "SAIR"];
        if self.dock.hover >= 0 && (self.dock.hover as usize) < nomes.len() {
            let cx = start_x + self.dock.hover * spacing;
            let tw = nomes[self.dock.hover as usize].len() as i32 * 6 * 2;
            Self::draw_text(&mut buf, w, h, cx - tw / 2, h - DOCK_H - 24, nomes[self.dock.hover as usize], 2, 0xFFCDD6F4);
        }

        // M13: bolinha embaixo do SISTEMA quando ha janelas abertas (macOS real)
        if !self.janelas.is_empty() {
            let cx = start_x;
            Self::draw_circle(&mut buf, w, h, cx, h - 8, 3, 0xFFCDD6F4);
        }

        // M13: DROPDOWN do menu Apple (quando menu aberto via clique no logo)
        if self.menu_aberto {
            let dx0 = 8;
            let dy0 = MENU_H + 4;
            let dw2 = 170;
            let dh2 = 3 * 30 + 10;
            Self::draw_rounded_rect(&mut buf, w, h, dx0 + 4, dy0 + 4, dw2, dh2, 10, 0x88101014);
            Self::draw_rounded_rect(&mut buf, w, h, dx0, dy0, dw2, dh2, 10, 0xFF313244);
            let itens_menu = ["SOBRE", "MISSION CONTROL", "SAIR"];
            for (k, item) in itens_menu.iter().enumerate() {
                let iy = dy0 + 10 + k as i32 * 30;
                let cor = if self.menu_drop_hover == k as i32 { 0xFFF84808 } else { 0xFFCDD6F4 };
                Self::draw_text(&mut buf, w, h, dx0 + 14, iy, item, 1, cor);
            }
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

        // ===== NOTIFICACOES (M16: toasts no canto, ultima camada) =====
        self.notif.podar(self.frame);
        self.notif.draw(&mut buf, w, h, self.frame);

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
        self.notif.disparar(self.frame, "SISTEMA", &format!("JANELA {} ABERTA", n + 1));
    }

    fn activate_menu(&mut self, item: i32) {
        match item {
            0 => self.abrir_janela(),
            1 => {
                log_msg("menu RESET: abrindo ResetExplode");
                let _ = std::process::Command::new("C:\\Users\\Rodrigo\\ResetExplode\\ResetExplode.exe").spawn();
            }
            2 => {
                log_msg("menu ARQUIVOS: app nativo");
                self.fb.abrir("C:/Users/Rodrigo");
                self.notif.disparar(self.frame, "ARQUIVOS", "PASTA DO USUÁRIO");
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
                self.notif.disparar(self.frame, "RESET", "RESET EXPLODE INICIADO");
            }
            2 => {
                log_msg("abrindo ARQUIVOS nativo da dock");
                self.fb.abrir("C:/Users/Rodrigo");
                self.notif.disparar(self.frame, "ARQUIVOS", "PASTA DO USUÁRIO");
            }
            3 => {
                log_msg("abrindo TERMINAL nativo da dock");
                self.term.abrir();
                self.notif.disparar(self.frame, "TERMINAL", "CONSOLE DO SISTEMA");
            }
            4 => {
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
                if self.fb.arrastando {
                    self.fb.x = mx - self.fb.dx;
                    self.fb.y = my - self.fb.dy;
                }
                if self.term.arrastando {
                    self.term.x = mx - self.term.dx;
                    self.term.y = my - self.term.dy;
                }
                // M13: hover no dropdown
                if self.menu_aberto {
                    self.menu_drop_hover = if mx >= 8 && mx <= 178 && my >= MENU_H + 4 && my <= MENU_H + 4 + 100 {
                        ((my - MENU_H - 14) / 30).max(0).min(2)
                    } else {
                        -1
                    };
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
                    // M15: TERMINAL por cima de tudo (X fecha, barra arrasta)
                    if self.term.aberto {
                        let (mx4, my4) = self.mouse;
                        if self.term.hit_close(mx4, my4) {
                            self.term.aberto = false;
                            self.term.arrastando = false;
                            log_msg("terminal fechado (x)");
                            self.draw();
                            return;
                        }
                        if self.term.hit_barra(mx4, my4) {
                            self.term.arrastando = true;
                            self.term.dx = mx4 - self.term.x;
                            self.term.dy = my4 - self.term.y;
                            log_msg("terminal pego pela barra");
                            return;
                        }
                    }
                    // M14: app ARQUIVOS por cima de tudo (X fecha, barra arrasta)
                    if self.fb.aberto {
                        let (mx3, my3) = self.mouse;
                        if self.fb.hit_close(mx3, my3) {
                            self.fb.aberto = false;
                            self.fb.arrastando = false;
                            log_msg("arquivos fechado (x)");
                            self.draw();
                            return;
                        }
                        if self.fb.hit_barra(mx3, my3) {
                            self.fb.arrastando = true;
                            self.fb.dx = mx3 - self.fb.x;
                            self.fb.dy = my3 - self.fb.y;
                            log_msg("arquivos pego pela barra");
                            return;
                        }
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
                    // M13: clique no dropdown executa e fecha
                    if self.menu_aberto {
                        let (mx2, my2) = self.mouse;
                        if mx2 >= 8 && mx2 <= 178 && my2 >= MENU_H + 4 && my2 <= MENU_H + 4 + 100 {
                            let k = ((my2 - MENU_H - 14) / 30).max(0).min(2);
                            self.menu_aberto = false;
                            match k {
                                0 => self.abrir_janela(),
                                1 => { self.mission = true; log_msg("dropdown: mission aberto"); }
                                _ => { log_msg("dropdown: SAIR"); std::process::exit(0); }
                            }
                            self.draw();
                            return;
                        }
                        self.menu_aberto = false;
                        self.draw();
                        return;
                    }
                    // M13: clique no LOGO abre/fecha o dropdown
                    if self.mouse.0 < 36 && self.mouse.1 < MENU_H as i32 {
                        self.menu_aberto = !self.menu_aberto;
                        log_msg("menu apple alternado");
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
                    if self.fb.arrastando {
                        self.fb.arrastando = false;
                        log_msg("arquivos solto");
                    }
                    if self.term.arrastando {
                        self.term.arrastando = false;
                        log_msg("terminal solto");
                    }
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == winit::event::ElementState::Pressed {
                    // Espaco abre/fecha o spotlight (M17: fora do cooldown do global)
                    if event.logical_key == winit::keyboard::Key::Named(winit::keyboard::NamedKey::Space) {
                        if self.frame >= self.hotkey_cooldown {
                            self.spot.aberto = !self.spot.aberto;
                            self.spot.query.clear();
                            log_msg("spotlight alternado");
                            self.hotkey_cooldown = self.frame + 15;
                        }
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
                                    if caminho == "@ARQUIVOS" {
                                        log_msg("spotlight: ARQUIVOS nativo");
                                        self.fb.abrir("C:/Users/Rodrigo");
                                        self.spot.aberto = false;
                                        self.spot.query.clear();
                                        self.draw();
                                        return;
                                    }
                                    if caminho == "@TERMINAL" {
                                        log_msg("spotlight: TERMINAL nativo");
                                        self.term.abrir();
                                        self.spot.aberto = false;
                                        self.spot.query.clear();
                                        self.draw();
                                        return;
                                    }
                                    log_msg(&format!("spotlight abrindo: {}", nome));
                                    let _ = std::process::Command::new(caminho).spawn();
                                    self.notif.disparar(self.frame, "SPOTLIGHT", &format!("ABRINDO {}", nome));
                                }
                                self.spot.aberto = false;
                                self.spot.query.clear();
                                self.draw();
                            }
                            _ => {}
                        }
                    }
                    // M15: TERMINAL captura tudo que digita
                    if !self.spot.aberto && self.term.aberto {
                        match &event.logical_key {
                            winit::keyboard::Key::Character(c) => {
                                let ch = c.as_str().chars().next();
                                if let Some(ch) = ch {
                                    self.term.tecla(ch);
                                }
                                self.draw();
                                return;
                            }
                            winit::keyboard::Key::Named(winit::keyboard::NamedKey::Space) => {
                                self.term.tecla(' ');
                                self.draw();
                                return;
                            }
                            winit::keyboard::Key::Named(winit::keyboard::NamedKey::Backspace) => {
                                self.term.tecla('\u{8}');
                                self.draw();
                                return;
                            }
                            winit::keyboard::Key::Named(winit::keyboard::NamedKey::Enter) => {
                                self.term.tecla('\n');
                                self.draw();
                                return;
                            }
                            winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape) => {
                                self.term.aberto = false;
                                log_msg("terminal fechado (esc)");
                                self.draw();
                                return;
                            }
                            _ => {}
                        }
                    }
                    // M14: app ARQUIVOS captura setas/enter/bksp/esc
                    if !self.spot.aberto && self.fb.aberto {
                        let mut consumiu = true;
                        match &event.logical_key {
                            winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowUp) => self.fb.mover(-1),
                            winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowDown) => self.fb.mover(1),
                            winit::keyboard::Key::Named(winit::keyboard::NamedKey::Enter) => self.fb.entrar_sel(),
                            winit::keyboard::Key::Named(winit::keyboard::NamedKey::Backspace) => self.fb.subir(),
                            winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape) => {
                                self.fb.aberto = false;
                                log_msg("arquivos fechado (esc)");
                            }
                            _ => consumiu = false,
                        }
                        if consumiu {
                            self.draw();
                            return;
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
        // M17: Ctrl+Espaco GLOBAL - detecta borda de subida em qualquer foco
        let ctrl = tecla_pressionada(VK_CONTROL);
        let espaco = tecla_pressionada(VK_SPACE);
        let agora = ctrl && espaco;
        if agora && !self.hotkey_prev && self.frame >= self.hotkey_cooldown {
            self.spot.aberto = !self.spot.aberto;
            self.spot.query.clear();
            self.hotkey_cooldown = self.frame + 15;
            log_msg("spotlight global alternado (ctrl+espaco)");
            self.draw();
        }
        self.hotkey_prev = agora;
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
