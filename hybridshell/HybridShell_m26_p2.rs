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
    ultimo_clique: (i32, i32, u32),
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
        ShellApp { profile, dock: Dock::new(), menu: MenuBar::new(), spot: Spotlight { aberto: false, query: String::new() }, janelas: Vec::new(), fb: FileBrowser::novo(), term: Terminal::novo(), cfg: ConfigApp::novo(), notas: NotasApp::novo(), calc: CalcApp::novo(), notif: Notificacoes::novo(), mission: false, mission_rects: Vec::new(), menu_aberto: false, menu_drop_hover: -1, hotkey_prev: false, hotkey_cooldown: 0, ultimo_clique: (0, 0, 0), mouse: (0, 0), window: None, surface: None, spot_window: None, spot_surface: None, spot_id: None, frame: 0, ligado_em: std::time::Instant::now(), mon_ram: (0, 0), mon_bat: None }
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

    fn draw_app_icone(buf: &mut [u32], w: i32, h: i32, cx: i32, cy: i32, r: i32, app: i32, fundo: u32, detalhe: u32) {
        match app {
            1 => Self::draw_icone_bolt(buf, w, h, cx, cy, r * 2 / 3, detalhe),
            2 => Self::draw_icone_folder(buf, w, h, cx, cy, r * 2 / 3, detalhe),
            3 => {
                Self::draw_line(buf, w, h, cx - r / 2, cy - r / 4, cx - r / 6, cy, r / 3, detalhe);
                Self::draw_line(buf, w, h, cx - r / 6, cy, cx - r / 2, cy + r / 4, r / 3, detalhe);
                Self::draw_line(buf, w, h, cx, cy + r / 4, cx + r / 2, cy + r / 4, r / 3, detalhe);
            }
            4 => {
                Self::draw_circle(buf, w, h, cx - r / 5, cy - r / 5, r / 3, detalhe);
                Self::draw_circle(buf, w, h, cx - r / 5, cy - r / 5, r / 6, fundo);
                Self::draw_line(buf, w, h, cx, cy, cx + r / 2, cy + r / 2, r / 3, detalhe);
                Self::draw_line(buf, w, h, cx + r / 6, cy + r / 3, cx + r / 3, cy + r / 6, r / 5, detalhe);
            }
            5 => {
                let fw = r;
                let fh = r * 5 / 4;
                let fx = cx - fw / 2;
                let fy = cy - fh / 2;
                Self::draw_rounded_rect(buf, w, h, fx, fy, fw, fh, 3, detalhe);
                for k in 0..3 {
                    let ly = fy + fh / 4 + k * (fh / 4);
                    Self::draw_line(buf, w, h, fx + fw / 5, ly, fx + fw * 4 / 5, ly, 2, fundo);
                }
            }
            6 => {
                let fw = r * 4 / 5;
                let fh = r;
                let fx = cx - fw / 2;
                let fy = cy - fh / 2;
                Self::draw_rounded_rect(buf, w, h, fx, fy, fw, fh, 3, detalhe);
                Self::draw_rounded_rect(buf, w, h, fx + fw / 6, fy + fh / 8, fw * 2 / 3, fh / 5, 2, fundo);
                for k in 0..4 {
                    let px = fx + fw / 4 + (k % 2) * (fw / 2);
                    let py = fy + fh / 2 + (k / 2) * (fh / 4);
                    Self::draw_circle(buf, w, h, px, py, 2, fundo);
                }
            }
            _ => {}
        }
    }

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

    fn draw_icone_folder(buf: &mut [u32], w: i32, h: i32, cx: i32, cy: i32, r: i32, color: u32) {
        let fw = r * 2;
        let fh = r * 3 / 2;
        let fx = cx - r;
        let fy = cy - r / 2;
        Self::draw_rounded_rect(buf, w, h, fx, fy, fw, fh, 4, color);
        Self::draw_rounded_rect(buf, w, h, fx, fy - r / 3, fw / 2, r / 2 + 2, 3, color);
    }

    fn draw_icone_x(buf: &mut [u32], w: i32, h: i32, cx: i32, cy: i32, r: i32, color: u32) {
        let t = (r / 2).max(3);
        Self::draw_line(buf, w, h, cx - r / 2, cy - r / 2, cx + r / 2, cy + r / 2, t, color);
        Self::draw_line(buf, w, h, cx + r / 2, cy - r / 2, cx - r / 2, cy + r / 2, t, color);
    }

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

        for (i, nome) in DESKTOP_APPS.iter().enumerate() {
            let iy = MENU_H + 24 + i as i32 * 84;
            let ix = 24 + 26;
            let fundo = DESKTOP_ICON_COR[i];
            Self::draw_rounded_rect(&mut buf, w, h, ix - 26, iy - 2, 52, 52, 10, fundo);
            Self::draw_app_icone(&mut buf, w, h, ix, iy + 24, 16, i as i32 + 1, fundo, 0xFF1E1E2E);
            let lw = nome.len() as i32 * 6;
            Self::draw_rounded_rect(&mut buf, w, h, ix + 26 - lw / 2 - 4, iy + 56, lw + 8, 14, 3, 0x90101014);
            Self::draw_text(&mut buf, w, h, ix + 26 - lw / 2, iy + 58, nome, 1, 0xFFCDD6F4);
        }

        Self::draw_rounded_rect(&mut buf, w, h, 0, 0, w, MENU_H, 0, 0xE61E1E2E);
        Self::draw_circle(&mut buf, w, h, 20, MENU_H / 2, 8, ICON);
        for (i, nome) in self.menu.itens.iter().enumerate() {
            let mx = 40 + i as i32 * 90;
            let cor = if self.menu.hover == i as i32 { 0xFFCDD6F4 } else { 0xFFA6ADC8 };
            Self::draw_text(&mut buf, w, h, mx, 9, nome, 1, cor);
        }
        let (hh, mm) = agora_hhmm();
        let mut rel = String::new();
        if hh < 10 { rel.push('0'); }
        rel.push_str(&hh.to_string());
        rel.push(':');
        if mm < 10 { rel.push('0'); }
        rel.push_str(&mm.to_string());
        let rw = rel.len() as i32 * 6 * 2;
        Self::draw_text(&mut buf, w, h, w - rw - 16, 9, &rel, 2, 0xFFCDD6F4);
        let (usada, total_m) = if self.mon_ram.1 > 0 { self.mon_ram } else { ram_usada_mb() };
        let mon_txt = format!("RAM {} / {} MB", usada, total_m);
        let mw_txt = mon_txt.len() as i32 * 6;
        Self::draw_text(&mut buf, w, h, w - rw - 16 - mw_txt - 30, 11, &mon_txt, 1, 0xFFA6E3A1);
        if let Some((pct, tomada)) = self.mon_bat {
            let bat_txt = format!("{}%{}", pct, if tomada { " T" } else { "" });
            let bw_txt = bat_txt.len() as i32 * 6;
            let cor_bat = if tomada { 0xFFA6E3A1 } else if pct <= 20 { 0xFFF38BA8 } else { 0xFFF9E2AF };
            Self::draw_text(&mut buf, w, h, w - rw - 16 - mw_txt - 30 - bw_txt - 24, 11, &bat_txt, 1, cor_bat);
            let bx = w - rw - 16 - mw_txt - 30 - bw_txt - 44;
            Self::draw_rounded_rect(&mut buf, w, h, bx, 10, 16, 9, 2, cor_bat);
            Self::draw_rounded_rect(&mut buf, w, h, bx + 16, 12, 2, 5, 1, cor_bat);
            let cheia = (pct as i32 * 14 / 100).max(1);
            Self::draw_rounded_rect(&mut buf, w, h, bx + 1, 11, cheia, 7, 1, cor_bat);
        }

        if self.mission {
            self.mission_rects.clear();
            Self::draw_rounded_rect(&mut buf, w, h, 0, 0, w, h, 0, 0xE6101014);
            let nj = self.janelas.iter().filter(|j| !j.minimizada).count();
            for (idx, j) in self.janelas.iter().filter(|j| !j.minimizada).enumerate() {
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

        let nj = self.janelas.iter().filter(|j| !j.minimizada).count();
        if !self.mission {
        for (idx, j) in self.janelas.iter().enumerate() {
            let foco = idx + 1 == nj;
            if j.minimizada {
                continue;
            }
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
            Self::draw_circle(&mut buf, w, h, jx + jw - 36, jy + 15, 7, 0xFFF9E2AF);
            Self::draw_circle(&mut buf, w, h, jx + jw - 54, jy + 15, 7, 0xFFA6E3A1);
            Self::draw_text(&mut buf, w, h, jx + 20, jy + 55, &format!("JANELA {} DE {}", idx + 1, nj), 2, 0xFFCDD6F4);
            Self::draw_text(&mut buf, w, h, jx + 20, jy + 95, &format!("RAM: {} MB", self.profile.total_ram_mb), 2, 0xFFA6ADC8);
            Self::draw_text(&mut buf, w, h, jx + 20, jy + 125, &format!("FONTE: {}", self.profile.fonte), 2, 0xFFA6ADC8);
            Self::draw_text(&mut buf, w, h, jx + 20, jy + 170, "CLIQUE PARA TRAZER PARA FRENTE", 1, 0xFF6C7086);
            Self::draw_text(&mut buf, w, h, jx + 20, jy + 190, "ESC OU X FECHA", 1, 0xFF6C7086);
            if foco {
                let ax = jx + jw - 14;
                let ay = jy + jh - 14;
                Self::draw_line(&mut buf, w, h, ax + 8, ay + 2, ax + 2, ay + 8, 2, 0xFF6C7086);
                Self::draw_line(&mut buf, w, h, ax + 10, ay + 6, ax + 6, ay + 10, 2, 0xFF6C7086);
            }
        }
        }

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
            Self::draw_rounded_rect(&mut buf, w, h, kx + 16, ky + 38, kw - 32, 40, 6, 0xFF11111B);
            let mut disp = self.calc.display.clone();
            if disp.len() > 14 { disp = disp[disp.len() - 14..].to_string(); }
            let dw = disp.len() as i32 * 6 * 2;
            Self::draw_text(&mut buf, w, h, kx + kw - 24 - dw, ky + 52, &disp, 2, 0xFFA6E3A1);
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
            let estado = if self.notas.sujo { "NAO SALVO - CTRL+S SALVA" } else { "SALVO NO DISCO" };
            Self::draw_text(&mut buf, w, h, nx + 90, ny + 11, estado, 1, if self.notas.sujo { 0xFFF84808 } else { 0xFF3F8A3F });
            let vis = ((nh - 80) / 14).max(1) as usize;
            let total = self.notas.linhas.len();
            let inicio = total.saturating_sub(vis);
            for (vi, idx) in (inicio..total).enumerate() {
                let mut linha = self.notas.linhas[idx].clone();
                let maxn = ((nw - 40) / 6) as usize;
                if linha.len() > maxn { linha = linha[..maxn].to_string(); }
                Self::draw_text(&mut buf, w, h, nx + 16, ny + 44 + vi as i32 * 14, &linha, 1, 0xFF3A3A3A);
            }
            if (self.frame / 30) % 2 == 0 {
                let ultima = total - 1;
                let cur_len = self.notas.linhas[ultima].len() as i32;
                Self::draw_line(&mut buf, w, h, nx + 16 + cur_len * 6, ny + 44 + (ultima - inicio) as i32 * 14, nx + 16 + cur_len * 6, ny + 54 + (ultima - inicio) as i32 * 14, 1, 0xFF1E1E2E);
            }
            Self::draw_text(&mut buf, w, h, nx + 14, ny + nh - 20, "CTRL+S SALVA - ESC FECHA E SALVA - CONTEUDO PERSISTE", 1, 0xFF8A7A50);
        }

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
            let secs = self.ligado_em.elapsed().as_secs();
            let min_up = secs / 60;
            Self::draw_text(&mut buf, w, h, gx + 20, gy + 50, "HYBRID OS - M19", 2, 0xFFCBA6F7);
            Self::draw_text(&mut buf, w, h, gx + 20, gy + 85, &format!("RAM: {} MB", self.profile.total_ram_mb), 1, 0xFFCDD6F4);
            Self::draw_text(&mut buf, w, h, gx + 20, gy + 105, &format!("FONTE DE HARDWARE: {}", self.profile.fonte), 1, 0xFFCDD6F4);
            Self::draw_text(&mut buf, w, h, gx + 20, gy + 125, &format!("MODO ATIVO: {}", self.profile.modo), 1, 0xFFCDD6F4);
            Self::draw_text(&mut buf, w, h, gx + 20, gy + 145, &format!("LIGADO HÁ: {} MIN", min_up), 1, 0xFFCDD6F4);
            Self::draw_text(&mut buf, w, h, gx + 20, gy + 180, "MODO DE RENDERIZAÇÃO (SALVA NO DISCO):", 1, 0xFFA6ADC8);
            let (ce, cf, ca) = if self.profile.modo == "EFFICIENT" { (0xFFA6E3A1, 0xFF6C7086, 0xFF6C7086) } else if self.profile.modo == "FULL" { (0xFF6C7086, 0xFFA6E3A1, 0xFF6C7086) } else { (0xFF6C7086, 0xFF6C7086, 0xFFA6E3A1) };
            Self::draw_text(&mut buf, w, h, gx + 30, gy + 205, "E = EFFICIENT (ECONÔMICO)", 1, ce);
            Self::draw_text(&mut buf, w, h, gx + 30, gy + 225, "F = FULL (COMPLETO)", 1, cf);
            Self::draw_text(&mut buf, w, h, gx + 30, gy + 245, "A = AUTO (AUTOMÁTICO)", 1, ca);
            Self::draw_text(&mut buf, w, h, gx + 20, gy + 280, "ESC FECHA - ESCOLHA SOBREVIVE AO REBOOT", 1, 0xFF6C7086);
        }

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
            let vis = ((th - 80) / 14).max(1) as usize;
            let total = self.term.linhas.len();
            let inicio = total.saturating_sub(vis - 1);
            for (vi, idx) in (inicio..total).enumerate() {
                let mut linha = self.term.linhas[idx].clone();
                let maxn = ((tw - 40) / 6) as usize;
                if linha.len() > maxn { linha = linha[..maxn].to_string(); }
                Self::draw_text(&mut buf, w, h, tx + 16, ty + 42 + vi as i32 * 14, &linha, 1, 0xFFA6E3A1);
            }
            let mut inp = format!("{} {}", self.term.cwd, self.term.input);
            if (self.frame / 30) % 2 == 0 { inp.push('_'); }
            if inp.len() > ((tw - 40) / 6) as usize {
                let take = ((tw - 40) / 6) as usize;
                inp = inp[inp.len() - take..].to_string();
            }
            Self::draw_text(&mut buf, w, h, tx + 16, ty + th - 28, &inp, 1, 0xFFCDD6F4);
        }

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
            let mut cam = self.fb.caminho.clone();
            let maxc = (fw - 40) / 12;
            if cam.len() > maxc as usize {
                let take = maxc as usize;
                cam = format!("/{}", &cam[cam.len() - take..]);
            }
            Self::draw_text(&mut buf, w, h, fx + 14, fy + 40, &cam, 1, 0xFFA6ADC8);
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

        Self::draw_rounded_rect(&mut buf, w, h, 0, h - DOCK_H, w, DOCK_H, 12, DOCK_BG);

        let n = self.dock.items.len() as i32;
        let spacing = 72;
        let start_x = (w - n * spacing) / 2 + spacing / 2;
        for i in 0..n {
            let cx = start_x + i * spacing;
            let cy = h - DOCK_H / 2;
            let scale = self.dock.scale_for(cx as f32);
            let r = (ITEM_BASE as f32 / 2.0 * scale) as i32;
            Self::draw_rounded_rect(&mut buf, w, h, cx - r, cy - r, r * 2, r * 2, r / 3, self.dock.items[i as usize]);
            match i {
                0 => Self::draw_icone_gear(&mut buf, w, h, cx, cy, r * 2 / 3, 0xFFE6E9F0),
                1 => Self::draw_app_icone(&mut buf, w, h, cx, cy, r * 2 / 3, 1, self.dock.items[1], 0xFF1E1E2E),
                2 => Self::draw_app_icone(&mut buf, w, h, cx, cy, r * 2 / 3, 2, self.dock.items[2], 0xFF1E1E2E),
                3 => Self::draw_app_icone(&mut buf, w, h, cx, cy, r * 2 / 3, 3, self.dock.items[3], 0xFF1E1E2E),
                4 => Self::draw_app_icone(&mut buf, w, h, cx, cy, r * 2 / 3, 4, self.dock.items[4], 0xFF1E1E2E),
                5 => Self::draw_app_icone(&mut buf, w, h, cx, cy, r * 2 / 3, 5, self.dock.items[5], 0xFF1E1E2E),
                6 => Self::draw_app_icone(&mut buf, w, h, cx, cy, r * 2 / 3, 6, self.dock.items[6], 0xFF1E1E2E),
                7 => Self::draw_icone_x(&mut buf, w, h, cx, cy, r * 2 / 3, 0xFFE6E9F0),
                _ => {}
            }
        }

        let nomes = ["SISTEMA", "RESET", "ARQUIVOS", "TERMINAL", "CONFIG", "NOTAS", "CALC", "SAIR"];
        if self.dock.hover >= 0 && (self.dock.hover as usize) < nomes.len() {
            let cx = start_x + self.dock.hover * spacing;
            let tw = nomes[self.dock.hover as usize].len() as i32 * 6 * 2;
            Self::draw_text(&mut buf, w, h, cx - tw / 2, h - DOCK_H - 24, nomes[self.dock.hover as usize], 2, 0xFFCDD6F4);
        }

        if self.janelas.iter().any(|j| !j.minimizada) {
            let cx = start_x;
            Self::draw_circle(&mut buf, w, h, cx, h - 8, 3, 0xFFCDD6F4);
        }

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
