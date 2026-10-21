// ============================================================
// HYBRID OS - CAMADA B / MARCO M3.1: SHELL COM LOG DE BOOT
// M3 (dock clicavel + texto + ResetExplode) + testemunha:
// grava C:/Users/Rodrigo/hs.log em cada etapa do boot.
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
}

impl HardwareProfile {
    fn detect() -> Self {
        let total_ram_mb = Self::ram_mb();
        let has_dx12 = Self::has_dx12_gpu();
        let technique = if total_ram_mb < 8192 || !has_dx12 {
            RenderTechnique::Efficient
        } else {
            RenderTechnique::Full
        };
        HardwareProfile { technique, total_ram_mb }
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

// ---------- App ----------
struct ShellApp {
    profile: HardwareProfile,
    dock: Dock,
    window: Option<Rc<Window>>,
    surface: Option<Surface<Rc<Window>, Rc<Window>>>,
    frame: u32,
}

impl ShellApp {
    fn new(profile: HardwareProfile) -> Self {
        ShellApp { profile, dock: Dock::new(), window: None, surface: None, frame: 0 }
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

        // janela de exemplo com titulo
        Self::draw_rounded_rect(&mut buf, w, h, w / 2 - 200, h / 2 - 150, 400, 300, 12, 0xFF313244);
        Self::draw_rounded_rect(&mut buf, w, h, w / 2 - 200, h / 2 - 150, 400, 30, 12, 0xFF45475A);
        Self::draw_text(&mut buf, w, h, w / 2 - 190, h / 2 - 145, "HYBRID OS", 2, 0xFFCDD6F4);

        // dock
        Self::draw_rounded_rect(&mut buf, w, h, 0, h - DOCK_H, w, DOCK_H, 12, DOCK_BG);

        // itens com amplificacao
        let n = self.dock.items.len() as i32;
        let spacing = 72;
        let start_x = (w - n * spacing) / 2 + spacing / 2;
        for i in 0..n {
            let cx = start_x + i * spacing;
            let cy = h - DOCK_H / 2;
            let scale = self.dock.scale_for(cx as f32);
            let r = (ITEM_BASE as f32 / 2.0 * scale) as i32;
            Self::draw_circle(&mut buf, w, h, cx, cy, r, self.dock.items[i as usize]);
        }

        // nome do app em hover
        let nomes = ["SISTEMA", "RESET", "ARQUIVOS", "SAIR"];
        if self.dock.hover >= 0 && (self.dock.hover as usize) < nomes.len() {
            let cx = start_x + self.dock.hover * spacing;
            let tw = nomes[self.dock.hover as usize].len() as i32 * 6 * 2;
            Self::draw_text(&mut buf, w, h, cx - tw / 2, h - DOCK_H - 24, nomes[self.dock.hover as usize], 2, 0xFFCDD6F4);
        }

        // relogio simples
        if (self.frame / 60) % 2 == 0 {
            Self::draw_circle(&mut buf, w, h, w - 40, 24, 4, ICON3);
        }

        if self.frame == 0 {
            log_msg("primeiro frame OK");
        }
        buf.present().unwrap();
        self.frame += 1;
    }

    fn activate(&self, item: i32) {
        match item {
            1 => {
                log_msg("abrindo ResetExplode da dock");
                let _ = std::process::Command::new("C:\\Users\\Rodrigo\\ResetExplode\\ResetExplode.exe").spawn();
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
                let win = self.window.clone();
                if let Some(win) = win {
                    let size = win.inner_size();
                    self.dock.mouse_x = position.x as f32;
                    self.dock.hover = self.dock.hit_test(size.width as i32, size.height as i32, position.y as f32);
                }
                self.draw();
            }
            WindowEvent::MouseInput { state, .. } => {
                if state == winit::event::ElementState::Pressed {
                    if self.dock.hover >= 0 {
                        self.activate(self.dock.hover);
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
        // M4: loop continuo com timer
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
