fn abrir_janela(&mut self) {
        if let Some(idx) = self.janelas.iter().position(|j| j.minimizada) {
            let mut j = self.janelas.remove(idx);
            j.minimizada = false;
            self.janelas.push(j);
            log_msg("janela restaurada da dock (estava minimizada)");
            self.notif.disparar(self.frame, "SISTEMA", "JANELA RESTAURADA");
            return;
        }
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
                if self.fb.minimizada {
                    self.fb.minimizada = false;
                    log_msg("arquivos restaurado da dock (estava minimizada)");
                    self.notif.disparar(self.frame, "SISTEMA", "ARQUIVOS RESTAURADA");
                } else {
                    self.notif.disparar(self.frame, "ARQUIVOS", "PASTA DO USUÁRIO");
                }
            }
            3 => {
                log_msg("abrindo TERMINAL nativo da dock");
                self.term.abrir();
                if self.term.minimizada {
                    self.term.minimizada = false;
                    log_msg("terminal restaurado da dock (estava minimizada)");
                    self.notif.disparar(self.frame, "SISTEMA", "TERMINAL RESTAURADA");
                } else {
                    self.notif.disparar(self.frame, "TERMINAL", "CONSOLE DO SISTEMA");
                }
            }
            4 => {
                log_msg("abrindo CONFIGURACOES da dock");
                self.cfg.abrir();
                if self.cfg.minimizada {
                    self.cfg.minimizada = false;
                    log_msg("config restaurado da dock (estava minimizada)");
                    self.notif.disparar(self.frame, "SISTEMA", "CONFIG RESTAURADA");
                } else {
                    self.notif.disparar(self.frame, "CONFIGURAÇÕES", "SISTEMA HYBRID OS");
                }
            }
            5 => {
                log_msg("abrindo NOTAS da dock");
                self.notas.abrir();
                if self.notas.minimizada {
                    self.notas.minimizada = false;
                    log_msg("notas restaurado da dock (estava minimizada)");
                    self.notif.disparar(self.frame, "SISTEMA", "NOTAS RESTAURADA");
                } else {
                    self.notif.disparar(self.frame, "NOTAS", "EDITOR DE TEXTO");
                }
            }
            6 => {
                log_msg("abrindo CALCULADORA da dock");
                self.calc.abrir();
                if self.calc.minimizada {
                    self.calc.minimizada = false;
                    log_msg("calc restaurado da dock (estava minimizada)");
                    self.notif.disparar(self.frame, "SISTEMA", "CALC RESTAURADA");
                } else {
                    self.notif.disparar(self.frame, "CALCULADORA", "PRONTA PARA USO");
                }
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
                if let Some(j) = self.janelas.iter_mut().find(|j| j.redimensionando) {
                    j.w = (mx - j.x).max(260);
                    j.h = (my - j.y).max(160);
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
                    let hit_desk = desktop_icon_hit(self.mouse.0, self.mouse.1);
                    if hit_desk >= 0 && !self.mission {
                        self.activate(hit_desk);
                        self.draw();
                        return;
                    }
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
                    if self.calc.aberto {
                        let (mk, nk) = self.mouse;
                        if self.calc.hit_close(mk, nk) {
                            self.calc.aberto = false;
                            self.calc.arrastando = false;
                            log_msg("calc fechado (x)");
                            self.draw();
                            return;
                        }
                        if luz_max_hit(self.calc.x, self.calc.y, self.calc.w, mk, nk) {
                            if self.calc.maximizada {
                                let (px, py, pw, ph) = self.calc.pre_max;
                                self.calc.x = px;
                                self.calc.y = py;
                                self.calc.w = pw;
                                self.calc.h = ph;
                                self.calc.maximizada = false;
                                log_msg("calc restaurado (verde)");
                            } else {
                                self.calc.pre_max = (self.calc.x, self.calc.y, self.calc.w, self.calc.h);
                                self.calc.x = 0;
                                self.calc.y = MENU_H;
                                self.calc.w = w_center(&self.window);
                                self.calc.h = h_center(&self.window) - MENU_H - DOCK_H;
                                self.calc.maximizada = true;
                                log_msg("calc maximizado (verde)");
                            }
                            self.draw();
                            return;
                        }
                        if luz_min_hit(self.calc.x, self.calc.y, self.calc.w, mk, nk) {
                            self.calc.minimizada = true;
                            self.calc.arrastando = false;
                            log_msg("calc minimizado (amarelo)");
                            self.notif.disparar(self.frame, "SISTEMA", "CALC MINIMIZADA - DOCK RESTAURA");
                            self.draw();
                            return;
                        }
                        if self.calc.hit_barra(mk, nk) {
                            let (lx, ly, lf) = self.ultimo_clique_nat;
                            let duplo = self.frame.wrapping_sub(lf) < 30 && (mk - lx).abs() < 8 && (nk - ly).abs() < 8;
                            self.ultimo_clique_nat = (mk, nk, self.frame);
                            if duplo {
                                if self.calc.maximizada {
                                    let (px, py, pw, ph) = self.calc.pre_max;
                                    self.calc.x = px;
                                    self.calc.y = py;
                                    self.calc.w = pw;
                                    self.calc.h = ph;
                                    self.calc.maximizada = false;
                                    log_msg("calc restaurado (duplo clique)");
                                } else {
                                    self.calc.pre_max = (self.calc.x, self.calc.y, self.calc.w, self.calc.h);
                                    self.calc.x = 0;
                                    self.calc.y = MENU_H;
                                    self.calc.w = w_center(&self.window);
                                    self.calc.h = h_center(&self.window) - MENU_H - DOCK_H;
                                    self.calc.maximizada = true;
                                    log_msg("calc maximizado (duplo clique)");
                                }
                            } else {
                                self.calc.arrastando = true;
                                self.calc.dx = mk - self.calc.x;
                                self.calc.dy = nk - self.calc.y;
                                log_msg("calc pego pela barra");
                            }
                            self.draw();
                            return;
                        }
                        if self.calc.clique(mk, nk) {
                            log_msg("calc botao clicado");
                            self.draw();
                            return;
                        }
                    }
                    if self.notas.aberto {
                        let (mn, nn) = self.mouse;
                        if self.notas.hit_close(mn, nn) {
                            if self.notas.sujo {
                                self.notas.salvar();
                            }
                            self.notas.aberto = false;
                            self.notas.arrastando = false;
                            log_msg("notas fechado (x)");
                            self.draw();
                            return;
                        }
                        if luz_max_hit(self.notas.x, self.notas.y, self.notas.w, mn, nn) {
                            if self.notas.maximizada {
                                let (px, py, pw, ph) = self.notas.pre_max;
                                self.notas.x = px;
                                self.notas.y = py;
                                self.notas.w = pw;
                                self.notas.h = ph;
                                self.notas.maximizada = false;
                                log_msg("notas restaurado (verde)");
                            } else {
                                self.notas.pre_max = (self.notas.x, self.notas.y, self.notas.w, self.notas.h);
                                self.notas.x = 0;
                                self.notas.y = MENU_H;
                                self.notas.w = w_center(&self.window);
                                self.notas.h = h_center(&self.window) - MENU_H - DOCK_H;
                                self.notas.maximizada = true;
                                log_msg("notas maximizado (verde)");
                            }
                            self.draw();
                            return;
                        }
                        if luz_min_hit(self.notas.x, self.notas.y, self.notas.w, mn, nn) {
                            self.notas.minimizada = true;
                            self.notas.arrastando = false;
                            log_msg("notas minimizado (amarelo)");
                            self.notif.disparar(self.frame, "SISTEMA", "NOTAS MINIMIZADA - DOCK RESTAURA");
                            self.draw();
                            return;
                        }
                        if self.notas.hit_barra(mn, nn) {
                            let (lx, ly, lf) = self.ultimo_clique_nat;
                            let duplo = self.frame.wrapping_sub(lf) < 30 && (mn - lx).abs() < 8 && (nn - ly).abs() < 8;
                            self.ultimo_clique_nat = (mn, nn, self.frame);
                            if duplo {
                                if self.notas.maximizada {
                                    let (px, py, pw, ph) = self.notas.pre_max;
                                    self.notas.x = px;
                                    self.notas.y = py;
                                    self.notas.w = pw;
                                    self.notas.h = ph;
                                    self.notas.maximizada = false;
                                    log_msg("notas restaurado (duplo clique)");
                                } else {
                                    self.notas.pre_max = (self.notas.x, self.notas.y, self.notas.w, self.notas.h);
                                    self.notas.x = 0;
                                    self.notas.y = MENU_H;
                                    self.notas.w = w_center(&self.window);
                                    self.notas.h = h_center(&self.window) - MENU_H - DOCK_H;
                                    self.notas.maximizada = true;
                                    log_msg("notas maximizado (duplo clique)");
                                }
                            } else {
                                self.notas.arrastando = true;
                                self.notas.dx = mn - self.notas.x;
                                self.notas.dy = nn - self.notas.y;
                                log_msg("notas pego pela barra");
                            }
                            self.draw();
                            return;
                        }
                    }
                    if self.cfg.aberto {
                        let (mc, nc) = self.mouse;
                        if self.cfg.hit_close(mc, nc) {
                            self.cfg.aberto = false;
                            self.cfg.arrastando = false;
                            log_msg("config fechado (x)");
                            self.draw();
                            return;
                        }
                        if luz_max_hit(self.cfg.x, self.cfg.y, self.cfg.w, mc, nc) {
                            if self.cfg.maximizada {
                                let (px, py, pw, ph) = self.cfg.pre_max;
                                self.cfg.x = px;
                                self.cfg.y = py;
                                self.cfg.w = pw;
                                self.cfg.h = ph;
                                self.cfg.maximizada = false;
                                log_msg("config restaurado (verde)");
                            } else {
                                self.cfg.pre_max = (self.cfg.x, self.cfg.y, self.cfg.w, self.cfg.h);
                                self.cfg.x = 0;
                                self.cfg.y = MENU_H;
                                self.cfg.w = w_center(&self.window);
                                self.cfg.h = h_center(&self.window) - MENU_H - DOCK_H;
                                self.cfg.maximizada = true;
                                log_msg("config maximizado (verde)");
                            }
                            self.draw();
                            return;
                        }
                        if luz_min_hit(self.cfg.x, self.cfg.y, self.cfg.w, mc, nc) {
                            self.cfg.minimizada = true;
                            self.cfg.arrastando = false;
                            log_msg("config minimizado (amarelo)");
                            self.notif.disparar(self.frame, "SISTEMA", "CONFIG MINIMIZADA - DOCK RESTAURA");
                            self.draw();
                            return;
                        }
                        if self.cfg.hit_barra(mc, nc) {
                            let (lx, ly, lf) = self.ultimo_clique_nat;
                            let duplo = self.frame.wrapping_sub(lf) < 30 && (mc - lx).abs() < 8 && (nc - ly).abs() < 8;
                            self.ultimo_clique_nat = (mc, nc, self.frame);
                            if duplo {
                                if self.cfg.maximizada {
                                    let (px, py, pw, ph) = self.cfg.pre_max;
                                    self.cfg.x = px;
                                    self.cfg.y = py;
                                    self.cfg.w = pw;
                                    self.cfg.h = ph;
                                    self.cfg.maximizada = false;
                                    log_msg("config restaurado (duplo clique)");
                                } else {
                                    self.cfg.pre_max = (self.cfg.x, self.cfg.y, self.cfg.w, self.cfg.h);
                                    self.cfg.x = 0;
                                    self.cfg.y = MENU_H;
                                    self.cfg.w = w_center(&self.window);
                                    self.cfg.h = h_center(&self.window) - MENU_H - DOCK_H;
                                    self.cfg.maximizada = true;
                                    log_msg("config maximizado (duplo clique)");
                                }
                            } else {
                                self.cfg.arrastando = true;
                                self.cfg.dx = mc - self.cfg.x;
                                self.cfg.dy = nc - self.cfg.y;
                                log_msg("config pego pela barra");
                            }
                            self.draw();
                            return;
                        }
                    }
                    if self.term.aberto {
                        let (mx4, my4) = self.mouse;
                        if self.term.hit_close(mx4, my4) {
                            self.term.aberto = false;
                            self.term.arrastando = false;
                            log_msg("terminal fechado (x)");
                            self.draw();
                            return;
                        }
                        if luz_max_hit(self.term.x, self.term.y, self.term.w, mx4, my4) {
                            if self.term.maximizada {
                                let (px, py, pw, ph) = self.term.pre_max;
                                self.term.x = px;
                                self.term.y = py;
                                self.term.w = pw;
                                self.term.h = ph;
                                self.term.maximizada = false;
                                log_msg("terminal restaurado (verde)");
                            } else {
                                self.term.pre_max = (self.term.x, self.term.y, self.term.w, self.term.h);
                                self.term.x = 0;
                                self.term.y = MENU_H;
                                self.term.w = w_center(&self.window);
                                self.term.h = h_center(&self.window) - MENU_H - DOCK_H;
                                self.term.maximizada = true;
                                log_msg("terminal maximizado (verde)");
                            }
                            self.draw();
                            return;
                        }
                        if luz_min_hit(self.term.x, self.term.y, self.term.w, mx4, my4) {
                            self.term.minimizada = true;
                            self.term.arrastando = false;
                            log_msg("terminal minimizado (amarelo)");
                            self.notif.disparar(self.frame, "SISTEMA", "TERMINAL MINIMIZADA - DOCK RESTAURA");
                            self.draw();
                            return;
                        }
                        if self.term.hit_barra(mx4, my4) {
                            let (lx, ly, lf) = self.ultimo_clique_nat;
                            let duplo = self.frame.wrapping_sub(lf) < 30 && (mx4 - lx).abs() < 8 && (my4 - ly).abs() < 8;
                            self.ultimo_clique_nat = (mx4, my4, self.frame);
                            if duplo {
                                if self.term.maximizada {
                                    let (px, py, pw, ph) = self.term.pre_max;
                                    self.term.x = px;
                                    self.term.y = py;
                                    self.term.w = pw;
                                    self.term.h = ph;
                                    self.term.maximizada = false;
                                    log_msg("terminal restaurado (duplo clique)");
                                } else {
                                    self.term.pre_max = (self.term.x, self.term.y, self.term.w, self.term.h);
                                    self.term.x = 0;
                                    self.term.y = MENU_H;
                                    self.term.w = w_center(&self.window);
                                    self.term.h = h_center(&self.window) - MENU_H - DOCK_H;
                                    self.term.maximizada = true;
                                    log_msg("terminal maximizado (duplo clique)");
                                }
                            } else {
                                self.term.arrastando = true;
                                self.term.dx = mx4 - self.term.x;
                                self.term.dy = my4 - self.term.y;
                                log_msg("terminal pego pela barra");
                            }
                            self.draw();
                            return;
                        }
                    }
                    if self.fb.aberto {
                        let (mx3, my3) = self.mouse;
                        if self.fb.hit_close(mx3, my3) {
                            self.fb.aberto = false;
                            self.fb.arrastando = false;
                            log_msg("arquivos fechado (x)");
                            self.draw();
                            return;
                        }
                        if luz_max_hit(self.fb.x, self.fb.y, self.fb.w, mx3, my3) {
                            if self.fb.maximizada {
                                let (px, py, pw, ph) = self.fb.pre_max;
                                self.fb.x = px;
                                self.fb.y = py;
                                self.fb.w = pw;
                                self.fb.h = ph;
                                self.fb.maximizada = false;
                                log_msg("arquivos restaurado (verde)");
                            } else {
                                self.fb.pre_max = (self.fb.x, self.fb.y, self.fb.w, self.fb.h);
                                self.fb.x = 0;
                                self.fb.y = MENU_H;
                                self.fb.w = w_center(&self.window);
                                self.fb.h = h_center(&self.window) - MENU_H - DOCK_H;
                                self.fb.maximizada = true;
                                log_msg("arquivos maximizado (verde)");
                            }
                            self.draw();
                            return;
                        }
                        if luz_min_hit(self.fb.x, self.fb.y, self.fb.w, mx3, my3) {
                            self.fb.minimizada = true;
                            self.fb.arrastando = false;
                            log_msg("arquivos minimizado (amarelo)");
                            self.notif.disparar(self.frame, "SISTEMA", "ARQUIVOS MINIMIZADA - DOCK RESTAURA");
                            self.draw();
                            return;
                        }
                        if self.fb.hit_barra(mx3, my3) {
                            let (lx, ly, lf) = self.ultimo_clique_nat;
                            let duplo = self.frame.wrapping_sub(lf) < 30 && (mx3 - lx).abs() < 8 && (my3 - ly).abs() < 8;
                            self.ultimo_clique_nat = (mx3, my3, self.frame);
                            if duplo {
                                if self.fb.maximizada {
                                    let (px, py, pw, ph) = self.fb.pre_max;
                                    self.fb.x = px;
                                    self.fb.y = py;
                                    self.fb.w = pw;
                                    self.fb.h = ph;
                                    self.fb.maximizada = false;
                                    log_msg("arquivos restaurado (duplo clique)");
                                } else {
                                    self.fb.pre_max = (self.fb.x, self.fb.y, self.fb.w, self.fb.h);
                                    self.fb.x = 0;
                                    self.fb.y = MENU_H;
                                    self.fb.w = w_center(&self.window);
                                    self.fb.h = h_center(&self.window) - MENU_H - DOCK_H;
                                    self.fb.maximizada = true;
                                    log_msg("arquivos maximizado (duplo clique)");
                                }
                            } else {
                                self.fb.arrastando = true;
                                self.fb.dx = mx3 - self.fb.x;
                                self.fb.dy = my3 - self.fb.y;
                                log_msg("arquivos pego pela barra");
                            }
                            self.draw();
                            return;
                        }
                    }
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
                    for idx in (0..self.janelas.len()).rev() {
                        let j = &self.janelas[idx];
                        if j.hit_min(mx, my) {
                            let mut j = self.janelas.remove(idx);
                            j.minimizada = true;
                            self.janelas.insert(idx, j);
                            log_msg("janela minimizada (amarelo)");
                            self.notif.disparar(self.frame, "SISTEMA", "JANELA MINIMIZADA - DOCK RESTAURA");
                            self.draw();
                            return;
                        }
                        if j.hit_max(mx, my) {
                            let mut j = self.janelas.remove(idx);
                            if j.maximizada {
                                let (px, py, pw, ph) = j.pre_max;
                                j.x = px;
                                j.y = py;
                                j.w = pw;
                                j.h = ph;
                                j.maximizada = false;
                                log_msg("janela restaurada (verde)");
                            } else {
                                j.pre_max = (j.x, j.y, j.w, j.h);
                                j.x = 0;
                                j.y = MENU_H;
                                j.w = w_center(&self.window);
                                j.h = h_center(&self.window) - MENU_H - DOCK_H;
                                j.maximizada = true;
                                log_msg("janela maximizada (verde)");
                            }
                            self.janelas.push(j);
                            self.draw();
                            return;
                        }
                    }
                    if let Some(idx) = pegar {
                        // M26: double-click na barra (2 cliques < 500ms, < 8px) = maximizar/restaurar
                        let (lx, ly, lf) = self.ultimo_clique;
                        let duplo = self.frame.wrapping_sub(lf) < 30 && (mx - lx).abs() < 8 && (my - ly).abs() < 8;
                        self.ultimo_clique = (mx, my, self.frame);
                        let mut j = self.janelas.remove(idx);
                        if duplo {
                            if j.maximizada {
                                let (px, py, pw, ph) = j.pre_max;
                                j.x = px;
                                j.y = py;
                                j.w = pw;
                                j.h = ph;
                                j.maximizada = false;
                                log_msg("janela restaurada (duplo clique)");
                            } else {
                                j.pre_max = (j.x, j.y, j.w, j.h);
                                j.x = 0;
                                j.y = MENU_H;
                                j.w = w_center(&self.window);
                                j.h = h_center(&self.window) - MENU_H - DOCK_H;
                                j.maximizada = true;
                                log_msg("janela maximizada (duplo clique)");
                            }
                            j.arrastando = false;
                        } else {
                            j.arrastando = true;
                            j.dx = mx - j.x;
                            j.dy = my - j.y;
                            log_msg("janela pega pela barra (foco)");
                        }
                        self.janelas.push(j);
                        self.draw();
                        return;
                    }
                    for idx in (0..self.janelas.len()).rev() {
                        let j = &self.janelas[idx];
                        if j.hit_alca(mx, my) {
                            let mut j = self.janelas.remove(idx);
                            j.redimensionando = true;
                            self.janelas.push(j);
                            log_msg("janela redimensionando (alca)");
                            self.draw();
                            return;
                        }
                    }
                    for idx in (0..self.janelas.len()).rev() {
                        let j = &self.janelas[idx];
                        if mx >= j.x && mx <= j.x + j.w && my >= j.y && my <= j.y + j.h {
                            if idx + 1 != self.janelas.len() {
                                let mut j = self.janelas.remove(idx);
                                j.arrastando = false;
                                self.janelas.push(j);
                                log_msg("janela focada pelo corpo");
                                self.draw();
                                return;
                            }
                            break;
                        }
                    }
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
                        // M27: Aero Snap - soltar na borda encaixa (esq/dir = metade, topo = maximiza)
                        let (sx, sy) = self.mouse;
                        let wscr = w_center(&self.window);
                        let hscr = h_center(&self.window);
                        let metade_h = hscr - MENU_H - DOCK_H;
                        if sx <= 20 && sy < 150 {
                            j.x = 0;
                            j.y = MENU_H;
                            j.w = wscr / 2;
                            j.h = metade_h / 2;
                            j.maximizada = false;
                            log_msg("snap: quadrante SUP ESQ");
                            self.notif.disparar(self.frame, "SISTEMA", "QUADRANTE SUPERIOR ESQUERDO");
                        } else if sx <= 20 && sy >= hscr - 150 {
                            j.x = 0;
                            j.y = MENU_H + metade_h / 2;
                            j.w = wscr / 2;
                            j.h = metade_h / 2;
                            j.maximizada = false;
                            log_msg("snap: quadrante INF ESQ");
                            self.notif.disparar(self.frame, "SISTEMA", "QUADRANTE INFERIOR ESQUERDO");
                        } else if sx <= 20 {
                            j.x = 0;
                            j.y = MENU_H;
                            j.w = wscr / 2;
                            j.h = metade_h;
                            j.maximizada = false;
                            log_msg("snap: janela encaixou a ESQUERDA");
                            self.notif.disparar(self.frame, "SISTEMA", "ENCAIXADA A ESQUERDA");
                        } else if sx >= wscr - 20 && sy < 150 {
                            j.x = wscr / 2;
                            j.y = MENU_H;
                            j.w = wscr / 2;
                            j.h = metade_h / 2;
                            j.maximizada = false;
                            log_msg("snap: quadrante SUP DIR");
                            self.notif.disparar(self.frame, "SISTEMA", "QUADRANTE SUPERIOR DIREITO");
                        } else if sx >= wscr - 20 && sy >= hscr - 150 {
                            j.x = wscr / 2;
                            j.y = MENU_H + metade_h / 2;
                            j.w = wscr / 2;
                            j.h = metade_h / 2;
                            j.maximizada = false;
                            log_msg("snap: quadrante INF DIR");
                            self.notif.disparar(self.frame, "SISTEMA", "QUADRANTE INFERIOR DIREITO");
                        } else if sx >= wscr - 20 {
                            j.x = wscr / 2;
                            j.y = MENU_H;
                            j.w = wscr / 2;
                            j.h = metade_h;
                            j.maximizada = false;
                            log_msg("snap: janela encaixou a DIREITA");
                            self.notif.disparar(self.frame, "SISTEMA", "ENCAIXADA A DIREITA");
                        } else if sy <= MENU_H + 8 {
                            j.pre_max = (j.x, j.y, j.w, j.h);
                            j.x = 0;
                            j.y = MENU_H;
                            j.w = wscr;
                            j.h = metade_h;
                            j.maximizada = true;
                            log_msg("snap: janela maximizada (topo)");
                            self.notif.disparar(self.frame, "SISTEMA", "MAXIMIZADA (SNAP TOPO)");
                        } else {
                            log_msg("janela solta");
                        }
                    }
                    if let Some(mut j) = self.janelas.iter_mut().find(|j| j.redimensionando) {
                        j.redimensionando = false;
                        log_msg("janela redimensionada");
                    }
                    if self.fb.arrastando {
                        self.fb.arrastando = false;
                        // M33: Aero Snap nos apps nativos (esq/dir = metade, topo = maximiza)
                        let (sx, sy) = self.mouse;
                        let wscr = w_center(&self.window);
                        let hscr = h_center(&self.window);
                        let metade_h = hscr - MENU_H - DOCK_H;
                                                if sx <= 20 && sy < 150 {
                            self.fb.x = 0;
                            self.fb.y = MENU_H;
                            self.fb.w = wscr / 2;
                            self.fb.h = metade_h / 2;
                            self.fb.maximizada = false;
                            log_msg("snap arquivos: quadrante SUP ESQ");
                            self.notif.disparar(self.frame, "SISTEMA", "ARQUIVOS QUADRANTE SUP ESQ");
                        } else if sx <= 20 && sy >= hscr - 150 {
                            self.fb.x = 0;
                            self.fb.y = MENU_H + metade_h / 2;
                            self.fb.w = wscr / 2;
                            self.fb.h = metade_h / 2;
                            self.fb.maximizada = false;
                            log_msg("snap arquivos: quadrante INF ESQ");
                            self.notif.disparar(self.frame, "SISTEMA", "ARQUIVOS QUADRANTE INF ESQ");
                        } else if sx <= 20 {
                            self.fb.x = 0;
                            self.fb.y = MENU_H;
                            self.fb.w = wscr / 2;
                            self.fb.h = metade_h;
                            self.fb.maximizada = false;
                            log_msg("snap arquivos: ESQUERDA");
                            self.notif.disparar(self.frame, "SISTEMA", "ARQUIVOS METADE ESQUERDA");
                        } else if sx >= wscr - 20 && sy < 150 {
                            self.fb.x = wscr / 2;
                            self.fb.y = MENU_H;
                            self.fb.w = wscr / 2;
                            self.fb.h = metade_h / 2;
                            self.fb.maximizada = false;
                            log_msg("snap arquivos: quadrante SUP DIR");
                            self.notif.disparar(self.frame, "SISTEMA", "ARQUIVOS QUADRANTE SUP DIR");
                        } else if sx >= wscr - 20 && sy >= hscr - 150 {
                            self.fb.x = wscr / 2;
                            self.fb.y = MENU_H + metade_h / 2;
                            self.fb.w = wscr / 2;
                            self.fb.h = metade_h / 2;
                            self.fb.maximizada = false;
                            log_msg("snap arquivos: quadrante INF DIR");
                            self.notif.disparar(self.frame, "SISTEMA", "ARQUIVOS QUADRANTE INF DIR");
                        } else if sx >= wscr - 20 {
                            self.fb.x = wscr / 2;
                            self.fb.y = MENU_H;
                            self.fb.w = wscr / 2;
                            self.fb.h = metade_h;
                            self.fb.maximizada = false;
                            log_msg("snap arquivos: DIREITA");
                            self.notif.disparar(self.frame, "SISTEMA", "ARQUIVOS METADE DIREITA");
                        } else if sy <= MENU_H + 8 {
                            self.fb.pre_max = (self.fb.x, self.fb.y, self.fb.w, self.fb.h);
                            self.fb.x = 0;
                            self.fb.y = MENU_H;
                            self.fb.w = wscr;
                            self.fb.h = metade_h;
                            self.fb.maximizada = true;
                            log_msg("snap arquivos: TOPO (maximiza)");
                            self.notif.disparar(self.frame, "SISTEMA", "ARQUIVOS MAXIMIZADA (SNAP)");
                        } else {
                            log_msg("arquivos solto");
                        }
                    }
                    if self.term.arrastando {
                        self.term.arrastando = false;
                        // M33: Aero Snap nos apps nativos
                        let (sx, sy) = self.mouse;
                        let wscr = w_center(&self.window);
                        let hscr = h_center(&self.window);
                        let metade_h = hscr - MENU_H - DOCK_H;
                                                if sx <= 20 && sy < 150 {
                            self.term.x = 0;
                            self.term.y = MENU_H;
                            self.term.w = wscr / 2;
                            self.term.h = metade_h / 2;
                            self.term.maximizada = false;
                            log_msg("snap terminal: quadrante SUP ESQ");
                            self.notif.disparar(self.frame, "SISTEMA", "TERMINAL QUADRANTE SUP ESQ");
                        } else if sx <= 20 && sy >= hscr - 150 {
                            self.term.x = 0;
                            self.term.y = MENU_H + metade_h / 2;
                            self.term.w = wscr / 2;
                            self.term.h = metade_h / 2;
                            self.term.maximizada = false;
                            log_msg("snap terminal: quadrante INF ESQ");
                            self.notif.disparar(self.frame, "SISTEMA", "TERMINAL QUADRANTE INF ESQ");
                        } else if sx <= 20 {
                            self.term.x = 0;
                            self.term.y = MENU_H;
                            self.term.w = wscr / 2;
                            self.term.h = metade_h;
                            self.term.maximizada = false;
                            log_msg("snap terminal: ESQUERDA");
                            self.notif.disparar(self.frame, "SISTEMA", "TERMINAL METADE ESQUERDA");
                        } else if sx >= wscr - 20 && sy < 150 {
                            self.term.x = wscr / 2;
                            self.term.y = MENU_H;
                            self.term.w = wscr / 2;
                            self.term.h = metade_h / 2;
                            self.term.maximizada = false;
                            log_msg("snap terminal: quadrante SUP DIR");
                            self.notif.disparar(self.frame, "SISTEMA", "TERMINAL QUADRANTE SUP DIR");
                        } else if sx >= wscr - 20 && sy >= hscr - 150 {
                            self.term.x = wscr / 2;
                            self.term.y = MENU_H + metade_h / 2;
                            self.term.w = wscr / 2;
                            self.term.h = metade_h / 2;
                            self.term.maximizada = false;
                            log_msg("snap terminal: quadrante INF DIR");
                            self.notif.disparar(self.frame, "SISTEMA", "TERMINAL QUADRANTE INF DIR");
                        } else if sx >= wscr - 20 {
                            self.term.x = wscr / 2;
                            self.term.y = MENU_H;
                            self.term.w = wscr / 2;
                            self.term.h = metade_h;
                            self.term.maximizada = false;
                            log_msg("snap terminal: DIREITA");
                            self.notif.disparar(self.frame, "SISTEMA", "TERMINAL METADE DIREITA");
                        } else if sy <= MENU_H + 8 {
                            self.term.pre_max = (self.term.x, self.term.y, self.term.w, self.term.h);
                            self.term.x = 0;
                            self.term.y = MENU_H;
                            self.term.w = wscr;
                            self.term.h = metade_h;
                            self.term.maximizada = true;
                            log_msg("snap terminal: TOPO (maximiza)");
                            self.notif.disparar(self.frame, "SISTEMA", "TERMINAL MAXIMIZADA (SNAP)");
                        } else {
                            log_msg("terminal solto");
                        }
                    }
                    if self.cfg.arrastando {
                        self.cfg.arrastando = false;
                        // M33: Aero Snap nos apps nativos
                        let (sx, sy) = self.mouse;
                        let wscr = w_center(&self.window);
                        let hscr = h_center(&self.window);
                        let metade_h = hscr - MENU_H - DOCK_H;
                                                if sx <= 20 && sy < 150 {
                            self.cfg.x = 0;
                            self.cfg.y = MENU_H;
                            self.cfg.w = wscr / 2;
                            self.cfg.h = metade_h / 2;
                            self.cfg.maximizada = false;
                            log_msg("snap config: quadrante SUP ESQ");
                            self.notif.disparar(self.frame, "SISTEMA", "CONFIG QUADRANTE SUP ESQ");
                        } else if sx <= 20 && sy >= hscr - 150 {
                            self.cfg.x = 0;
                            self.cfg.y = MENU_H + metade_h / 2;
                            self.cfg.w = wscr / 2;
                            self.cfg.h = metade_h / 2;
                            self.cfg.maximizada = false;
                            log_msg("snap config: quadrante INF ESQ");
                            self.notif.disparar(self.frame, "SISTEMA", "CONFIG QUADRANTE INF ESQ");
                        } else if sx <= 20 {
                            self.cfg.x = 0;
                            self.cfg.y = MENU_H;
                            self.cfg.w = wscr / 2;
                            self.cfg.h = metade_h;
                            self.cfg.maximizada = false;
                            log_msg("snap config: ESQUERDA");
                            self.notif.disparar(self.frame, "SISTEMA", "CONFIG METADE ESQUERDA");
                        } else if sx >= wscr - 20 && sy < 150 {
                            self.cfg.x = wscr / 2;
                            self.cfg.y = MENU_H;
                            self.cfg.w = wscr / 2;
                            self.cfg.h = metade_h / 2;
                            self.cfg.maximizada = false;
                            log_msg("snap config: quadrante SUP DIR");
                            self.notif.disparar(self.frame, "SISTEMA", "CONFIG QUADRANTE SUP DIR");
                        } else if sx >= wscr - 20 && sy >= hscr - 150 {
                            self.cfg.x = wscr / 2;
                            self.cfg.y = MENU_H + metade_h / 2;
                            self.cfg.w = wscr / 2;
                            self.cfg.h = metade_h / 2;
                            self.cfg.maximizada = false;
                            log_msg("snap config: quadrante INF DIR");
                            self.notif.disparar(self.frame, "SISTEMA", "CONFIG QUADRANTE INF DIR");
                        } else if sx >= wscr - 20 {
                            self.cfg.x = wscr / 2;
                            self.cfg.y = MENU_H;
                            self.cfg.w = wscr / 2;
                            self.cfg.h = metade_h;
                            self.cfg.maximizada = false;
                            log_msg("snap config: DIREITA");
                            self.notif.disparar(self.frame, "SISTEMA", "CONFIG METADE DIREITA");
                        } else if sy <= MENU_H + 8 {
                            self.cfg.pre_max = (self.cfg.x, self.cfg.y, self.cfg.w, self.cfg.h);
                            self.cfg.x = 0;
                            self.cfg.y = MENU_H;
                            self.cfg.w = wscr;
                            self.cfg.h = metade_h;
                            self.cfg.maximizada = true;
                            log_msg("snap config: TOPO (maximiza)");
                            self.notif.disparar(self.frame, "SISTEMA", "CONFIG MAXIMIZADA (SNAP)");
                        } else {
                            log_msg("config solto");
                        }
                    }
                    if self.notas.arrastando {
                        self.notas.arrastando = false;
                        // M33: Aero Snap nos apps nativos
                        let (sx, sy) = self.mouse;
                        let wscr = w_center(&self.window);
                        let hscr = h_center(&self.window);
                        let metade_h = hscr - MENU_H - DOCK_H;
                                                if sx <= 20 && sy < 150 {
                            self.notas.x = 0;
                            self.notas.y = MENU_H;
                            self.notas.w = wscr / 2;
                            self.notas.h = metade_h / 2;
                            self.notas.maximizada = false;
                            log_msg("snap notas: quadrante SUP ESQ");
                            self.notif.disparar(self.frame, "SISTEMA", "NOTAS QUADRANTE SUP ESQ");
                        } else if sx <= 20 && sy >= hscr - 150 {
                            self.notas.x = 0;
                            self.notas.y = MENU_H + metade_h / 2;
                            self.notas.w = wscr / 2;
                            self.notas.h = metade_h / 2;
                            self.notas.maximizada = false;
                            log_msg("snap notas: quadrante INF ESQ");
                            self.notif.disparar(self.frame, "SISTEMA", "NOTAS QUADRANTE INF ESQ");
                        } else if sx <= 20 {
                            self.notas.x = 0;
                            self.notas.y = MENU_H;
                            self.notas.w = wscr / 2;
                            self.notas.h = metade_h;
                            self.notas.maximizada = false;
                            log_msg("snap notas: ESQUERDA");
                            self.notif.disparar(self.frame, "SISTEMA", "NOTAS METADE ESQUERDA");
                        } else if sx >= wscr - 20 && sy < 150 {
                            self.notas.x = wscr / 2;
                            self.notas.y = MENU_H;
                            self.notas.w = wscr / 2;
                            self.notas.h = metade_h / 2;
                            self.notas.maximizada = false;
                            log_msg("snap notas: quadrante SUP DIR");
                            self.notif.disparar(self.frame, "SISTEMA", "NOTAS QUADRANTE SUP DIR");
                        } else if sx >= wscr - 20 && sy >= hscr - 150 {
                            self.notas.x = wscr / 2;
                            self.notas.y = MENU_H + metade_h / 2;
                            self.notas.w = wscr / 2;
                            self.notas.h = metade_h / 2;
                            self.notas.maximizada = false;
                            log_msg("snap notas: quadrante INF DIR");
                            self.notif.disparar(self.frame, "SISTEMA", "NOTAS QUADRANTE INF DIR");
                        } else if sx >= wscr - 20 {
                            self.notas.x = wscr / 2;
                            self.notas.y = MENU_H;
                            self.notas.w = wscr / 2;
                            self.notas.h = metade_h;
                            self.notas.maximizada = false;
                            log_msg("snap notas: DIREITA");
                            self.notif.disparar(self.frame, "SISTEMA", "NOTAS METADE DIREITA");
                        } else if sy <= MENU_H + 8 {
                            self.notas.pre_max = (self.notas.x, self.notas.y, self.notas.w, self.notas.h);
                            self.notas.x = 0;
                            self.notas.y = MENU_H;
                            self.notas.w = wscr;
                            self.notas.h = metade_h;
                            self.notas.maximizada = true;
                            log_msg("snap notas: TOPO (maximiza)");
                            self.notif.disparar(self.frame, "SISTEMA", "NOTAS MAXIMIZADA (SNAP)");
                        } else {
                            log_msg("notas solto");
                        }
                    }
                    if self.calc.arrastando {
                        self.calc.arrastando = false;
                        // M33: Aero Snap nos apps nativos
                        let (sx, sy) = self.mouse;
                        let wscr = w_center(&self.window);
                        let hscr = h_center(&self.window);
                        let metade_h = hscr - MENU_H - DOCK_H;
                                                if sx <= 20 && sy < 150 {
                            self.calc.x = 0;
                            self.calc.y = MENU_H;
                            self.calc.w = wscr / 2;
                            self.calc.h = metade_h / 2;
                            self.calc.maximizada = false;
                            log_msg("snap calc: quadrante SUP ESQ");
                            self.notif.disparar(self.frame, "SISTEMA", "CALC QUADRANTE SUP ESQ");
                        } else if sx <= 20 && sy >= hscr - 150 {
                            self.calc.x = 0;
                            self.calc.y = MENU_H + metade_h / 2;
                            self.calc.w = wscr / 2;
                            self.calc.h = metade_h / 2;
                            self.calc.maximizada = false;
                            log_msg("snap calc: quadrante INF ESQ");
                            self.notif.disparar(self.frame, "SISTEMA", "CALC QUADRANTE INF ESQ");
                        } else if sx <= 20 {
                            self.calc.x = 0;
                            self.calc.y = MENU_H;
                            self.calc.w = wscr / 2;
                            self.calc.h = metade_h;
                            self.calc.maximizada = false;
                            log_msg("snap calc: ESQUERDA");
                            self.notif.disparar(self.frame, "SISTEMA", "CALC METADE ESQUERDA");
                        } else if sx >= wscr - 20 && sy < 150 {
                            self.calc.x = wscr / 2;
                            self.calc.y = MENU_H;
                            self.calc.w = wscr / 2;
                            self.calc.h = metade_h / 2;
                            self.calc.maximizada = false;
                            log_msg("snap calc: quadrante SUP DIR");
                            self.notif.disparar(self.frame, "SISTEMA", "CALC QUADRANTE SUP DIR");
                        } else if sx >= wscr - 20 && sy >= hscr - 150 {
                            self.calc.x = wscr / 2;
                            self.calc.y = MENU_H + metade_h / 2;
                            self.calc.w = wscr / 2;
                            self.calc.h = metade_h / 2;
                            self.calc.maximizada = false;
                            log_msg("snap calc: quadrante INF DIR");
                            self.notif.disparar(self.frame, "SISTEMA", "CALC QUADRANTE INF DIR");
                        } else if sx >= wscr - 20 {
                            self.calc.x = wscr / 2;
                            self.calc.y = MENU_H;
                            self.calc.w = wscr / 2;
                            self.calc.h = metade_h;
                            self.calc.maximizada = false;
                            log_msg("snap calc: DIREITA");
                            self.notif.disparar(self.frame, "SISTEMA", "CALC METADE DIREITA");
                        } else if sy <= MENU_H + 8 {
                            self.calc.pre_max = (self.calc.x, self.calc.y, self.calc.w, self.calc.h);
                            self.calc.x = 0;
                            self.calc.y = MENU_H;
                            self.calc.w = wscr;
                            self.calc.h = metade_h;
                            self.calc.maximizada = true;
                            log_msg("snap calc: TOPO (maximiza)");
                            self.notif.disparar(self.frame, "SISTEMA", "CALC MAXIMIZADA (SNAP)");
                        } else {
                            log_msg("calc solto");
                        }
                    }
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == winit::event::ElementState::Pressed {
                    if event.logical_key == winit::keyboard::Key::Named(winit::keyboard::NamedKey::Space) {
                        if self.frame >= self.hotkey_cooldown {
                            self.alternar_spot();
                        }
                        return;
                    }
                    // M29: Ctrl+Setas = snap da janela focada (estilo Win+Setas)
                    if tecla_pressionada(VK_CONTROL) {
                        let mut direcao: Option<char> = None;
                        if event.logical_key == winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowLeft) { direcao = Some('L'); }
                        if event.logical_key == winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowRight) { direcao = Some('R'); }
                        if event.logical_key == winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowUp) { direcao = Some('U'); }
                        if event.logical_key == winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowDown) { direcao = Some('D'); }
                        if let Some(d) = direcao {
                            if let Some(idx) = self.janelas.iter().rposition(|j| !j.minimizada) {
                                let mut j = self.janelas.remove(idx);
                                let wscr = w_center(&self.window);
                                let hscr = h_center(&self.window);
                                let metade_h = hscr - MENU_H - DOCK_H;
                                if d == 'L' {
                                    j.x = 0;
                                    j.y = MENU_H;
                                    j.w = wscr / 2;
                                    j.h = metade_h;
                                    j.maximizada = false;
                                    log_msg("snap teclado: ESQUERDA");
                                    self.notif.disparar(self.frame, "SISTEMA", "METADE ESQUERDA (CTRL+SETA)");
                                }
                                if d == 'R' {
                                    j.x = wscr / 2;
                                    j.y = MENU_H;
                                    j.w = wscr / 2;
                                    j.h = metade_h;
                                    j.maximizada = false;
                                    log_msg("snap teclado: DIREITA");
                                    self.notif.disparar(self.frame, "SISTEMA", "METADE DIREITA (CTRL+SETA)");
                                }
                                if d == 'U' {
                                    j.pre_max = (j.x, j.y, j.w, j.h);
                                    j.x = 0;
                                    j.y = MENU_H;
                                    j.w = wscr;
                                    j.h = metade_h;
                                    j.maximizada = true;
                                    log_msg("snap teclado: MAXIMIZA");
                                    self.notif.disparar(self.frame, "SISTEMA", "MAXIMIZADA (CTRL+CIMA)");
                                }
                                if d == 'D' {
                                    if j.maximizada {
                                        let (px, py, pw, ph) = j.pre_max;
                                        j.x = px;
                                        j.y = py;
                                        j.w = pw;
                                        j.h = ph;
                                        j.maximizada = false;
                                        log_msg("snap teclado: RESTAURA");
                                        self.notif.disparar(self.frame, "SISTEMA", "RESTAURADA (CTRL+BAIXO)");
                                    } else {
                                        j.minimizada = true;
                                        log_msg("snap teclado: MINIMIZA");
                                        self.notif.disparar(self.frame, "SISTEMA", "MINIMIZADA (CTRL+BAIXO)");
                                    }
                                }
                                self.janelas.push(j);
                                self.draw();
                                return;
                            }
                        }
                    }