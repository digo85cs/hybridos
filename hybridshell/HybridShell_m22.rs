// ============================================================
// HYBRID OS - CAMADA B / MARCO M22: MONITOR AO VIVO
// M21 + barra de menu viva: RAM usada/total atualiza a cada
// frame (GlobalMemoryStatusEx) + bateria real (porcentagem e
// na tomada?) via GetSystemPowerStatus - o híbrido se observa.
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

// ---------- M22: bateria (GetSystemPowerStatus) ----------
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

// (percentual 0-100, na_tomada) - None se nao tem bateria
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

// RAM usada/total ao vivo (MB)
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
    modo: &'static str,
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
        // M19: modo persistido sobrescreve (AUTO = sem override)
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

// ---------- M19: config persistente (hybridos_config.json) ----------
const CONFIG_PATH: &str = "C:/Users/Rodrigo/hybridos_config.json";
const NOTAS_PATH: &str = "C:/Users/Rodrigo/hybridos_notas.txt";

// le o modo salvo (EFFICIENT/FULL/AUTO) - None se nao existe/invalido
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

// grava o modo no disco
fn config_salvar_modo(modo: &str) {
    let conteudo = format!("{{\n  \"tecnica\": \"{}\"\n}}\n", modo);
    if let Ok(mut f) = std::fs::File::create(CONFIG_PATH) {
        let _ = f.write_all(conteudo.as_bytes());
        log_msg(&format!("M19: config salva -> {}", modo));
    }
}

// ---------- Cores do tema ----------
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
    ("CONFIG", "@CONFIG"),
    ("NOTAS", "@NOTAS"),
    ("CALC", "@CALC"),
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

// ---------- App CALCULADORA nativo (M21) ----------
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
}

impl CalcApp {
    fn novo() -> Self {
        CalcApp { aberto: false, x: 340, y: 110, w: 300, h: 400, arrastando: false, dx: 0, dy: 0, display: "0".to_string(), acc: 0.0, op: ' ', digitando: true }
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

    // tecla de digito/operador/acao (teclado e mouse usam o mesmo caminho)
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

    // grade de botoes clicaveis (4 colunas + linha do 0)
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

// ---------- App NOTAS nativo (M20, editor de texto) ----------
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
}

impl NotasApp {
    fn novo() -> Self {
        NotasApp { aberto: false, x: 240, y: 90, w: 520, h: 360, arrastando: false, dx: 0, dy: 0, linhas: vec![String::new()], sujo: false }
    }

    fn abrir(&mut self) {
        // carrega o arquivo se existir (conteudo persiste entre sessoes)
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
            // backspace: apaga char ou junta com a linha de cima
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

// ---------- App CONFIGURAÇÕES nativo (M19) ----------
struct ConfigApp {
    aberto: bool,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    arrastando: bool,
    dx: i32,
    dy: i32,
}

impl ConfigApp {
    fn novo() -> Self {
        ConfigApp { aberto: false, x: 300, y: 120, w: 480, h: 330, arrastando: false, dx: 0, dy: 0 }
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
    cfg: ConfigApp,
    notas: NotasApp,
    calc: CalcApp,
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
    spot_window: Option<Rc<Window>>,
    spot_surface: Option<Surface<Rc<Window>, Rc<Window>>>,
    spot_id: Option<winit::window::WindowId>,
    frame: u32,
    ligado_em: std::time::Instant,
    mon_ram: (u64, u64),
    mon_bat: Option<(u32, bool)>,
}

impl ShellApp {
    fn new(profile: HardwareProfile) -> Self {
        ShellApp { profile, dock: Dock::new(), menu: MenuBar::new(), spot: Spotlight { aberto: false, query: String::new() }, janelas: Vec::new(), fb: FileBrowser::novo(), term: Terminal::novo(), cfg: ConfigApp::novo(), notas: NotasApp::novo(), calc: CalcApp::novo(), notif: Notificacoes::novo(), mission: false, mission_rects: Vec::new(), menu_aberto: false, menu_drop_hover: -1, hotkey_prev: false, hotkey_cooldown: 0, mouse: (0, 0), window: None, surface: None, spot_window: None, spot_surface: None, spot_id: None, frame: 0, ligado_em: std::time::Instant::now(), mon_ram: (0, 0), mon_bat: None }
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
        // M22: monitor ao vivo (RAM usada/total + bateria) - atualiza a cada frame
        let (usada, total_m) = if self.mon_ram.1 > 0 { self.mon_ram } else { ram_usada_mb() };
        let mon_txt = format!("RAM {} / {} MB", usada, total_m);
        let mw_txt = mon_txt.len() as i32 * 6;
        Self::draw_text(&mut buf, w, h, w - rw - 16 - mw_txt - 30, 11, &mon_txt, 1, 0xFFA6E3A1);
        if let Some((pct, tomada)) = self.mon_bat {
            let bat_txt = format!("{}%{}", pct, if tomada { " T" } else { "" });
            let bw_txt = bat_txt.len() as i32 * 6;
            let cor_bat = if tomada { 0xFFA6E3A1 } else if pct <= 20 { 0xFFF38BA8 } else { 0xFFF9E2AF };
            Self::draw_text(&mut buf, w, h, w - rw - 16 - mw_txt - 30 - bw_txt - 24, 11, &bat_txt, 1, cor_bat);
            // iconezinho de bateria
            let bx = w - rw - 16 - mw_txt - 30 - bw_txt - 44;
            Self::draw_rounded_rect(&mut buf, w, h, bx, 10, 16, 9, 2, cor_bat);
            Self::draw_rounded_rect(&mut buf, w, h, bx + 16, 12, 2, 5, 1, cor_bat);
            let cheia = (pct as i32 * 14 / 100).max(1);
            Self::draw_rounded_rect(&mut buf, w, h, bx + 1, 11, cheia, 7, 1, cor_bat);
        }

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

        // ===== APP CALCULADORA NATIVO (M21) =====
        if self.calc.aberto {
            let kx = self.calc.x;
            let ky = self.calc.y;
            let kw = self.calc.w;
            let kh = self.calc.h;
            Self::draw_rounded_rect(&mut buf, w, h, kx + 5, ky + 5, kw, kh, 12, 0x66101014);
            Self::draw_rounded_rect(&mut buf, w, h, kx, ky, kw, kh, 12, 0xFF1E1E2E);
            Self::draw_rounded_rect(&mut buf, w, h, kx, ky, kw, 30, 12, 0xFF45475A);
            Self::draw_text(&mut buf, w, h, kx + 14, ky + 10, "CALCULADORA", 2, 0xFFCDD6F4);
            Self::draw_circle(&mut buf, w, h, kx + kw - 18, ky + 15, 7, 0xFFF38BA8);
            Self::draw_text(&mut buf, w, h, kx + kw - 23, ky + 11, "X", 1, 0xFF1E1E2E);
            // display
            Self::draw_rounded_rect(&mut buf, w, h, kx + 16, ky + 38, kw - 32, 40, 6, 0xFF11111B);
            let mut disp = self.calc.display.clone();
            if disp.len() > 14 { disp = disp[disp.len() - 14..].to_string(); }
            let dw = disp.len() as i32 * 6 * 2;
            Self::draw_text(&mut buf, w, h, kx + kw - 24 - dw, ky + 52, &disp, 2, 0xFFA6E3A1);
            // botoes clicaveis
            for (ch, bx, by, bw, bh) in self.calc.layout_botoes() {
                let (fundo, txt) = if ch == '=' {
                    (0xFFF84808, 0xFFFFFFFF)
                } else if ch == 'C' {
                    (0xFFF38BA8, 0xFF1E1E2E)
                } else if ch == '+' || ch == '-' || ch == 'x' || ch == '/' {
                    (0xFF585B70, 0xFFCDD6F4)
                } else {
                    (0xFF313244, 0xFFCDD6F4)
                };
                Self::draw_rounded_rect(&mut buf, w, h, bx, by, bw, bh, 6, fundo);
                let tw2 = ch.to_string().len() as i32 * 6 * 2;
                let tx = bx + bw / 2 - tw2 / 2;
                let ty2 = by + bh / 2 - 7;
                Self::draw_text(&mut buf, w, h, tx, ty2, &ch.to_string(), 2, txt);
            }
            Self::draw_text(&mut buf, w, h, kx + 16, ky + kh - 18, "TECLADO: NUMEROS + - * / ENTER = ESC FECHA", 1, 0xFF6C7086);
        }

        // ===== APP NOTAS NATIVO (M20) =====
        if self.notas.aberto {
            let nx = self.notas.x;
            let ny = self.notas.y;
            let nw = self.notas.w;
            let nh = self.notas.h;
            Self::draw_rounded_rect(&mut buf, w, h, nx + 5, ny + 5, nw, nh, 12, 0x66101014);
            Self::draw_rounded_rect(&mut buf, w, h, nx, ny, nw, nh, 12, 0xFFFBF5DC);
            Self::draw_rounded_rect(&mut buf, w, h, nx, ny, nw, 30, 12, 0xFFF2E3B3);
            Self::draw_text(&mut buf, w, h, nx + 14, ny + 10, "NOTAS", 2, 0xFF1E1E2E);
            Self::draw_circle(&mut buf, w, h, nx + nw - 18, ny + 15, 7, 0xFFF38BA8);
            Self::draw_text(&mut buf, w, h, nx + nw - 23, ny + 11, "X", 1, 0xFF1E1E2E);
            // indicador salvo/sujo
            let estado = if self.notas.sujo { "NAO SALVO - CTRL+S SALVA" } else { "SALVO NO DISCO" };
            Self::draw_text(&mut buf, w, h, nx + 90, ny + 11, estado, 1, if self.notas.sujo { 0xFFF84808 } else { 0xFF3F8A3F });
            // linhas do texto (scroll no fim)
            let vis = ((nh - 80) / 14).max(1) as usize;
            let total = self.notas.linhas.len();
            let inicio = total.saturating_sub(vis);
            for (vi, idx) in (inicio..total).enumerate() {
                let mut linha = self.notas.linhas[idx].clone();
                let maxn = ((nw - 40) / 6) as usize;
                if linha.len() > maxn { linha = linha[..maxn].to_string(); }
                Self::draw_text(&mut buf, w, h, nx + 16, ny + 44 + vi as i32 * 14, &linha, 1, 0xFF3A3A3A);
            }
            // cursor na linha atual
            if (self.frame / 30) % 2 == 0 {
                let ultima = total - 1;
                let cur_len = self.notas.linhas[ultima].len() as i32;
                Self::draw_line(&mut buf, w, h, nx + 16 + cur_len * 6, ny + 44 + (ultima - inicio) as i32 * 14, nx + 16 + cur_len * 6, ny + 54 + (ultima - inicio) as i32 * 14, 1, 0xFF1E1E2E);
            }
            Self::draw_text(&mut buf, w, h, nx + 14, ny + nh - 20, "CTRL+S SALVA - ESC FECHA E SALVA - CONTEUDO PERSISTE", 1, 0xFF8A7A50);
        }

        // ===== APP CONFIGURAÇÕES NATIVO (M19) =====
        if self.cfg.aberto {
            let gx = self.cfg.x;
            let gy = self.cfg.y;
            let gw = self.cfg.w;
            let gh = self.cfg.h;
            Self::draw_rounded_rect(&mut buf, w, h, gx + 5, gy + 5, gw, gh, 12, 0x66101014);
            Self::draw_rounded_rect(&mut buf, w, h, gx, gy, gw, gh, 12, 0xFF1E1E2E);
            Self::draw_rounded_rect(&mut buf, w, h, gx, gy, gw, 30, 12, 0xFF45475A);
            Self::draw_text(&mut buf, w, h, gx + 14, gy + 10, "CONFIGURAÇÕES", 2, 0xFFCDD6F4);
            Self::draw_circle(&mut buf, w, h, gx + gw - 18, gy + 15, 7, 0xFFF38BA8);
            Self::draw_text(&mut buf, w, h, gx + gw - 23, gy + 11, "X", 1, 0xFF1E1E2E);
            // informacoes do sistema
            let secs = self.ligado_em.elapsed().as_secs();
            let min_up = secs / 60;
            Self::draw_text(&mut buf, w, h, gx + 20, gy + 50, "HYBRID OS - M19", 2, 0xFFCBA6F7);
            Self::draw_text(&mut buf, w, h, gx + 20, gy + 85, &format!("RAM: {} MB", self.profile.total_ram_mb), 1, 0xFFCDD6F4);
            Self::draw_text(&mut buf, w, h, gx + 20, gy + 105, &format!("FONTE DE HARDWARE: {}", self.profile.fonte), 1, 0xFFCDD6F4);
            Self::draw_text(&mut buf, w, h, gx + 20, gy + 125, &format!("MODO ATIVO: {}", self.profile.modo), 1, 0xFFCDD6F4);
            Self::draw_text(&mut buf, w, h, gx + 20, gy + 145, &format!("LIGADO HÁ: {} MIN", min_up), 1, 0xFFCDD6F4);
            // modos
            Self::draw_text(&mut buf, w, h, gx + 20, gy + 180, "MODO DE RENDERIZAÇÃO (SALVA NO DISCO):", 1, 0xFFA6ADC8);
            let (ce, cf, ca) = if self.profile.modo == "EFFICIENT" { (0xFFA6E3A1, 0xFF6C7086, 0xFF6C7086) } else if self.profile.modo == "FULL" { (0xFF6C7086, 0xFFA6E3A1, 0xFF6C7086) } else { (0xFF6C7086, 0xFF6C7086, 0xFFA6E3A1) };
            Self::draw_text(&mut buf, w, h, gx + 30, gy + 205, "E = EFFICIENT (ECONÔMICO)", 1, ce);
            Self::draw_text(&mut buf, w, h, gx + 30, gy + 225, "F = FULL (COMPLETO)", 1, cf);
            Self::draw_text(&mut buf, w, h, gx + 30, gy + 245, "A = AUTO (AUTOMÁTICO)", 1, ca);
            Self::draw_text(&mut buf, w, h, gx + 20, gy + 280, "ESC FECHA - ESCOLHA SOBREVIVE AO REBOOT", 1, 0xFF6C7086);
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
                4 => {
                    // M19: icone CONFIGURACOES = chave inglesa (anel + haste)
                    Self::draw_circle(&mut buf, w, h, cx - r / 5, cy - r / 5, r / 3, 0xFF1E1E2E);
                    Self::draw_circle(&mut buf, w, h, cx - r / 5, cy - r / 5, r / 6, self.dock.items[4]);
                    Self::draw_line(&mut buf, w, h, cx, cy, cx + r / 2, cy + r / 2, r / 3, 0xFF1E1E2E);
                    Self::draw_line(&mut buf, w, h, cx + r / 6, cy + r / 3, cx + r / 3, cy + r / 6, r / 5, 0xFF1E1E2E);
                }
                5 => {
                    // M20: icone NOTAS = folha com linhas
                    let fw = r;
                    let fh = r * 5 / 4;
                    let fx = cx - fw / 2;
                    let fy = cy - fh / 2;
                    Self::draw_rounded_rect(&mut buf, w, h, fx, fy, fw, fh, 3, 0xFF1E1E2E);
                    for k in 0..3 {
                        let ly = fy + fh / 4 + k * (fh / 4);
                        Self::draw_line(&mut buf, w, h, fx + fw / 5, ly, fx + fw * 4 / 5, ly, 2, self.dock.items[5]);
                    }
                }
                6 => {
                    // M21: icone CALCULADORA = corpo + display + 4 pontos
                    let fw = r * 4 / 5;
                    let fh = r;
                    let fx = cx - fw / 2;
                    let fy = cy - fh / 2;
                    Self::draw_rounded_rect(&mut buf, w, h, fx, fy, fw, fh, 3, 0xFF1E1E2E);
                    Self::draw_rounded_rect(&mut buf, w, h, fx + fw / 6, fy + fh / 8, fw * 2 / 3, fh / 5, 2, self.dock.items[6]);
                    for k in 0..4 {
                        let px = fx + fw / 4 + (k % 2) * (fw / 2);
                        let py = fy + fh / 2 + (k / 2) * (fh / 4);
                        Self::draw_circle(&mut buf, w, h, px, py, 2, self.dock.items[6]);
                    }
                }
                7 => Self::draw_icone_x(&mut buf, w, h, cx, cy, r * 2 / 3, 0xFFE6E9F0),
                _ => {}
            }
        }

        // nome do app em hover
        let nomes = ["SISTEMA", "RESET", "ARQUIVOS", "TERMINAL", "CONFIG", "NOTAS", "CALC", "SAIR"];
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

        // M18: o Spotlight agora vive na janela flutuante AlwaysOnTop (draw_spot)

        if self.frame == 0 {
            log_msg("primeiro frame OK");
        }
        buf.present().unwrap();
        self.frame += 1;
    }

    fn fechar_spot(&mut self) {
        self.spot.aberto = false;
        self.spot.query.clear();
        if let Some(sw) = self.spot_window.clone() {
            sw.set_visible(false);
        }
    }

    fn alternar_spot(&mut self) {
        if self.spot.aberto {
            self.fechar_spot();
            log_msg("spotlight fechado");
        } else {
            self.spot.aberto = true;
            self.spot.query.clear();
            if let (Some(sw), Some(mw)) = (self.spot_window.clone(), self.window.clone()) {
                let size = mw.inner_size();
                let x = ((size.width as i32 - 560) / 2).max(0);
                let y = (size.height as i32 / 3).max(0);
                sw.set_outer_position(winit::dpi::LogicalPosition::new(x as f64, y as f64));
                sw.set_visible(true);
                sw.focus_window();
                log_msg("spotlight aberto (janela flutuante sempre por cima)");
            }
        }
        self.hotkey_cooldown = self.frame + 15;
        self.draw();
        self.draw_spot();
    }

    // M18: conteudo da janela flutuante do Spotlight
    fn draw_spot(&mut self) {
        let Some(win) = self.spot_window.clone() else { return };
        let Some(surf) = self.spot_surface.as_mut() else { return };
        let size = win.inner_size();
        let (w, h) = (size.width as i32, size.height as i32);
        if w == 0 || h == 0 { return; }
        let (w32, h32) = (w.max(1) as u32, h.max(1) as u32);
        surf.resize(std::num::NonZeroU32::new(w32).unwrap(), std::num::NonZeroU32::new(h32).unwrap()).ok();
        let mut buf = surf.buffer_mut().unwrap();
        for y in 0..h {
            for x in 0..w {
                buf[(y * w + x) as usize] = 0xF82A2A3C;
            }
        }
        Self::draw_rounded_rect(&mut buf, w, h, 0, 0, w, 3, 0, ICON);
        let mut mostrar = self.spot.query.clone();
        if (self.frame / 30) % 2 == 0 { mostrar.push('|'); }
        Self::draw_text(&mut buf, w, h, 20, 24, &mostrar, 2, 0xFFCDD6F4);
        if let Some((nome, _)) = self.spot.buscar() {
            Self::draw_text(&mut buf, w, h, 20, 76, nome, 1, 0xFFA6E3A1);
        }
        Self::draw_text(&mut buf, w, h, 20, h - 24, "CTRL+ESPAÇO ABRE/FECHA - ESC SAI - ENTER ABRE", 1, 0xFF6C7086);
        buf.present().unwrap();
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
                log_msg("abrindo CONFIGURACOES da dock");
                self.cfg.abrir();
                self.notif.disparar(self.frame, "CONFIGURAÇÕES", "SISTEMA HYBRID OS");
            }
            5 => {
                log_msg("abrindo NOTAS da dock");
                self.notas.abrir();
                self.notif.disparar(self.frame, "NOTAS", "EDITOR DE TEXTO");
            }
            6 => {
                log_msg("abrindo CALCULADORA da dock");
                self.calc.abrir();
                self.notif.disparar(self.frame, "CALCULADORA", "PRONTA PARA USO");
            }
            7 => {
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
        // M18: janela flutuante do Spotlight (AlwaysOnTop, oculta por padrão)
        if self.spot_window.is_none() {
            let attrs = Window::default_attributes()
                .with_title("Spotlight")
                .with_window_level(WindowLevel::AlwaysOnTop)
                .with_decorations(false)
                .with_visible(false)
                .with_inner_size(winit::dpi::LogicalSize::new(560.0, 140.0));
            match el.create_window(attrs) {
                Ok(w) => {
                    let win = Rc::new(w);
                    match Context::new(win.clone()) {
                        Ok(ctx) => {
                            match Surface::new(&ctx, win.clone()) {
                                Ok(surf) => {
                                    self.spot_id = Some(win.id());
                                    self.spot_surface = Some(surf);
                                    self.spot_window = Some(win);
                                    log_msg("janela spotlight criada OK");
                                }
                                Err(e) => log_msg(&format!("surface spot ERRO: {:?}", e)),
                            }
                        }
                        Err(e) => log_msg(&format!("contexto spot ERRO: {:?}", e)),
                    }
                }
                Err(e) => log_msg(&format!("janela spot ERRO: {:?}", e)),
            }
        }
    }

    fn window_event(&mut self, _el: &ActiveEventLoop, id: winit::window::WindowId, event: WindowEvent) {
        // M18: eventos da janela flutuante - redesenha ela; teclado passa pro
        // handler principal (que é idempotente ao foco); mouse é ignorado
        let da_spot = self.spot_id.map(|s| s == id).unwrap_or(false);
        if da_spot {
            match &event {
                WindowEvent::RedrawRequested => { self.draw_spot(); return; }
                WindowEvent::CursorMoved { .. } | WindowEvent::MouseInput { .. } | WindowEvent::CloseRequested => { return; }
                _ => {}
            }
        }
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
                if self.cfg.arrastando {
                    self.cfg.x = mx - self.cfg.dx;
                    self.cfg.y = my - self.cfg.dy;
                }
                if self.notas.arrastando {
                    self.notas.x = mx - self.notas.dx;
                    self.notas.y = my - self.notas.dy;
                }
                if self.calc.arrastando {
                    self.calc.x = mx - self.calc.dx;
                    self.calc.y = my - self.calc.dy;
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
                    // M21: CALC por cima de tudo (X fecha, barra arrasta, botoes clicam)
                    if self.calc.aberto {
                        let (mk, nk) = self.mouse;
                        if self.calc.hit_close(mk, nk) {
                            self.calc.aberto = false;
                            self.calc.arrastando = false;
                            log_msg("calc fechado (x)");
                            self.draw();
                            return;
                        }
                        if self.calc.hit_barra(mk, nk) {
                            self.calc.arrastando = true;
                            self.calc.dx = mk - self.calc.x;
                            self.calc.dy = nk - self.calc.y;
                            log_msg("calc pego pela barra");
                            return;
                        }
                        if self.calc.clique(mk, nk) {
                            log_msg("calc botao clicado");
                            self.draw();
                            return;
                        }
                    }
                    // M20: NOTAS por cima de tudo (X fecha, barra arrasta)
                    if self.notas.aberto {
                        let (mn, nn) = self.mouse;
                        if self.notas.hit_close(mn, nn) {
                            // fecha salvando se tiver coisa suja
                            if self.notas.sujo {
                                self.notas.salvar();
                            }
                            self.notas.aberto = false;
                            self.notas.arrastando = false;
                            log_msg("notas fechado (x)");
                            self.draw();
                            return;
                        }
                        if self.notas.hit_barra(mn, nn) {
                            self.notas.arrastando = true;
                            self.notas.dx = mn - self.notas.x;
                            self.notas.dy = nn - self.notas.y;
                            log_msg("notas pego pela barra");
                            return;
                        }
                    }
                    // M19: CONFIG por cima de tudo (X fecha, barra arrasta)
                    if self.cfg.aberto {
                        let (mc, nc) = self.mouse;
                        if self.cfg.hit_close(mc, nc) {
                            self.cfg.aberto = false;
                            self.cfg.arrastando = false;
                            log_msg("config fechado (x)");
                            self.draw();
                            return;
                        }
                        if self.cfg.hit_barra(mc, nc) {
                            self.cfg.arrastando = true;
                            self.cfg.dx = mc - self.cfg.x;
                            self.cfg.dy = nc - self.cfg.y;
                            log_msg("config pego pela barra");
                            return;
                        }
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
                    if self.cfg.arrastando {
                        self.cfg.arrastando = false;
                        log_msg("config solto");
                    }
                    if self.notas.arrastando {
                        self.notas.arrastando = false;
                        log_msg("notas solto");
                    }
                    if self.calc.arrastando {
                        self.calc.arrastando = false;
                        log_msg("calc solto");
                    }
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == winit::event::ElementState::Pressed {
                    // Espaco abre/fecha o spotlight (M17: fora do cooldown do global)
                    if event.logical_key == winit::keyboard::Key::Named(winit::keyboard::NamedKey::Space) {
                        if self.frame >= self.hotkey_cooldown {
                            self.alternar_spot();
                        }
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
                                self.fechar_spot();
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
                                        self.fechar_spot();
                                        self.draw();
                                        return;
                                    }
                                    if caminho == "@TERMINAL" {
                                        log_msg("spotlight: TERMINAL nativo");
                                        self.term.abrir();
                                        self.fechar_spot();
                                        self.draw();
                                        return;
                                    }
                                    if caminho == "@CONFIG" {
                                        log_msg("spotlight: CONFIGURACOES");
                                        self.cfg.abrir();
                                        self.fechar_spot();
                                        self.draw();
                                        return;
                                    }
                                    if caminho == "@NOTAS" {
                                        log_msg("spotlight: NOTAS");
                                        self.notas.abrir();
                                        self.fechar_spot();
                                        self.draw();
                                        return;
                                    }
                                    if caminho == "@CALC" {
                                        log_msg("spotlight: CALCULADORA");
                                        self.calc.abrir();
                                        self.fechar_spot();
                                        self.draw();
                                        return;
                                    }
                                    log_msg(&format!("spotlight abrindo: {}", nome));
                                    let _ = std::process::Command::new(caminho).spawn();
                                    self.notif.disparar(self.frame, "SPOTLIGHT", &format!("ABRINDO {}", nome));
                                }
                                self.fechar_spot();
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
                    // M19: CONFIG captura E/F/A (modo) e ESC
                    if !self.spot.aberto && self.cfg.aberto {
                        let mut consumiu_cfg = true;
                        if let winit::keyboard::Key::Character(c) = &event.logical_key {
                            if c.as_str().eq_ignore_ascii_case("e") {
                                config_salvar_modo("EFFICIENT");
                                self.profile.technique = RenderTechnique::Efficient;
                                self.profile.modo = "EFFICIENT";
                                self.notif.disparar(self.frame, "CONFIGURAÇÕES", "MODO EFFICIENT SALVO");
                                log_msg("modo EFFICIENT salvo");
                            } else if c.as_str().eq_ignore_ascii_case("f") {
                                config_salvar_modo("FULL");
                                self.profile.technique = RenderTechnique::Full;
                                self.profile.modo = "FULL";
                                self.notif.disparar(self.frame, "CONFIGURAÇÕES", "MODO FULL SALVO");
                                log_msg("modo FULL salvo");
                            } else if c.as_str().eq_ignore_ascii_case("a") {
                                config_salvar_modo("AUTO");
                                let auto = if self.profile.total_ram_mb < 8192 { RenderTechnique::Efficient } else { RenderTechnique::Full };
                                self.profile.technique = auto;
                                self.profile.modo = "AUTO";
                                self.notif.disparar(self.frame, "CONFIGURAÇÕES", "MODO AUTO SALVO");
                                log_msg("modo AUTO salvo");
                            } else {
                                consumiu_cfg = false;
                            }
                        } else {
                            consumiu_cfg = false;
                        }
                        if event.logical_key == winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape) {
                            self.cfg.aberto = false;
                            log_msg("config fechado (esc)");
                        }
                        if consumiu_cfg || event.logical_key == winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape) {
                            self.draw();
                            return;
                        }
                    }
                    // M20: NOTAS captura digitacao + Ctrl+S salva
                    if !self.spot.aberto && self.notas.aberto {
                        let ctrl = tecla_pressionada(VK_CONTROL);
                        if ctrl {
                            if let winit::keyboard::Key::Character(c) = &event.logical_key {
                                if c.as_str().eq_ignore_ascii_case("s") {
                                    self.notas.salvar();
                                    self.notif.disparar(self.frame, "NOTAS", "TEXTO SALVO NO DISCO");
                                    self.draw();
                                    return;
                                }
                            }
                        }
                        let mut consumiu_notas = true;
                        match &event.logical_key {
                            winit::keyboard::Key::Character(c) => {
                                let ch = c.as_str().chars().next();
                                if let Some(ch) = ch {
                                    if ch.is_ascii_graphic() {
                                        self.notas.tecla(ch);
                                    }
                                }
                            }
                            winit::keyboard::Key::Named(winit::keyboard::NamedKey::Space) => {
                                self.notas.tecla(' ');
                            }
                            winit::keyboard::Key::Named(winit::keyboard::NamedKey::Backspace) => {
                                self.notas.tecla('\u{8}');
                            }
                            winit::keyboard::Key::Named(winit::keyboard::NamedKey::Enter) => {
                                self.notas.tecla('\n');
                            }
                            winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape) => {
                                if self.notas.sujo {
                                    self.notas.salvar();
                                }
                                self.notas.aberto = false;
                                log_msg("notas fechado (esc, salvo)");
                            }
                            _ => consumiu_notas = false,
                        }
                        if consumiu_notas {
                            self.draw();
                            return;
                        }
                    }
                    // M21: CALC captura digitos, operadores, ENTER (=), ESC fecha
                    if !self.spot.aberto && self.calc.aberto {
                        let mut consumiu_calc = true;
                        match &event.logical_key {
                            winit::keyboard::Key::Character(c) => {
                                let ch = c.as_str().chars().next();
                                if let Some(ch) = ch {
                                    if ch.is_ascii_digit() || ch == '.' || ch == '+' || ch == '-' || ch == '*' || ch == '/' {
                                        self.calc.tecla_calc(ch);
                                    } else if ch == 'c' || ch == 'C' {
                                        self.calc.tecla_calc('C');
                                    } else {
                                        consumiu_calc = false;
                                    }
                                } else {
                                    consumiu_calc = false;
                                }
                            }
                            winit::keyboard::Key::Named(winit::keyboard::NamedKey::Enter) => {
                                self.calc.tecla_calc('=');
                            }
                            winit::keyboard::Key::Named(winit::keyboard::NamedKey::Backspace) => {
                                self.calc.limpar();
                            }
                            winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape) => {
                                self.calc.aberto = false;
                                log_msg("calc fechado (esc)");
                            }
                            _ => consumiu_calc = false,
                        }
                        if consumiu_calc {
                            self.draw();
                            return;
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
                            // M19: tecla C abre CONFIGURAÇÕES
                            if c.as_str().eq_ignore_ascii_case("c") {
                                self.cfg.abrir();
                                log_msg("configuracoes aberta (tecla c)");
                                self.draw();
                                return;
                            }
                            // M20: tecla N abre NOTAS
                            if c.as_str().eq_ignore_ascii_case("n") {
                                self.notas.abrir();
                                log_msg("notas aberta (tecla n)");
                                self.draw();
                                return;
                            }
                            // M21: tecla L abre CALCULADORA
                            if c.as_str().eq_ignore_ascii_case("l") {
                                self.calc.abrir();
                                log_msg("calculadora aberta (tecla l)");
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
            self.alternar_spot();
            log_msg("spotlight global alternado (ctrl+espaco)");
        }
        self.hotkey_prev = agora;
        // M22: monitor ao vivo - RAM e bateria a cada ~1s
        if self.frame % 60 == 0 {
            self.mon_ram = ram_usada_mb();
            self.mon_bat = bateria_ler();
        }
        if let Some(swin) = self.spot_window.clone() {
            if self.spot.aberto {
                swin.request_redraw();
            }
        }
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
