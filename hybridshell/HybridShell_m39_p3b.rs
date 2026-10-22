// M34: Ctrl+Setas = snap do app nativo aberto (quando nao ha janela SOBRE)
                    if tecla_pressionada(VK_CONTROL) {
                        let mut direcao: Option<char> = None;
                        if event.logical_key == winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowLeft) { direcao = Some('L'); }
                        if event.logical_key == winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowRight) { direcao = Some('R'); }
                        if event.logical_key == winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowUp) { direcao = Some('U'); }
                        if event.logical_key == winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowDown) { direcao = Some('D'); }
                        if let Some(d) = direcao {
                            let wscr = w_center(&self.window);
                            let hscr = h_center(&self.window);
                            let metade_h = hscr - MENU_H - DOCK_H;
                            if self.term.aberto && !self.term.minimizada {
                                if d == 'L' {
                                    self.term.x = 0;
                                    self.term.y = MENU_H;
                                    self.term.w = wscr / 2;
                                    self.term.h = metade_h;
                                    self.term.maximizada = false;
                                    log_msg("snap teclado TERMINAL: ESQUERDA");
                                    self.notif.disparar(self.frame, "SISTEMA", "TERMINAL METADE ESQUERDA (CTRL+SETA)");
                                }
                                if d == 'R' {
                                    self.term.x = wscr / 2;
                                    self.term.y = MENU_H;
                                    self.term.w = wscr / 2;
                                    self.term.h = metade_h;
                                    self.term.maximizada = false;
                                    log_msg("snap teclado TERMINAL: DIREITA");
                                    self.notif.disparar(self.frame, "SISTEMA", "TERMINAL METADE DIREITA (CTRL+SETA)");
                                }
                                if d == 'U' {
                                    self.term.pre_max = (self.term.x, self.term.y, self.term.w, self.term.h);
                                    self.term.x = 0;
                                    self.term.y = MENU_H;
                                    self.term.w = wscr;
                                    self.term.h = metade_h;
                                    self.term.maximizada = true;
                                    log_msg("snap teclado TERMINAL: MAXIMIZA");
                                    self.notif.disparar(self.frame, "SISTEMA", "TERMINAL MAXIMIZADA (CTRL+CIMA)");
                                }
                                if d == 'D' {
                                    if self.term.maximizada {
                                        let (px, py, pw, ph) = self.term.pre_max;
                                        self.term.x = px;
                                        self.term.y = py;
                                        self.term.w = pw;
                                        self.term.h = ph;
                                        self.term.maximizada = false;
                                        log_msg("snap teclado TERMINAL: RESTAURA");
                                        self.notif.disparar(self.frame, "SISTEMA", "TERMINAL RESTAURADA (CTRL+BAIXO)");
                                    } else {
                                        self.term.minimizada = true;
                                        log_msg("snap teclado TERMINAL: MINIMIZA");
                                        self.notif.disparar(self.frame, "SISTEMA", "TERMINAL MINIMIZADA (CTRL+BAIXO)");
                                    }
                                }
                                self.draw();
                                return;
                            }
                            if self.fb.aberto && !self.fb.minimizada {
                                if d == 'L' {
                                    self.fb.x = 0;
                                    self.fb.y = MENU_H;
                                    self.fb.w = wscr / 2;
                                    self.fb.h = metade_h;
                                    self.fb.maximizada = false;
                                    log_msg("snap teclado ARQUIVOS: ESQUERDA");
                                    self.notif.disparar(self.frame, "SISTEMA", "ARQUIVOS METADE ESQUERDA (CTRL+SETA)");
                                }
                                if d == 'R' {
                                    self.fb.x = wscr / 2;
                                    self.fb.y = MENU_H;
                                    self.fb.w = wscr / 2;
                                    self.fb.h = metade_h;
                                    self.fb.maximizada = false;
                                    log_msg("snap teclado ARQUIVOS: DIREITA");
                                    self.notif.disparar(self.frame, "SISTEMA", "ARQUIVOS METADE DIREITA (CTRL+SETA)");
                                }
                                if d == 'U' {
                                    self.fb.pre_max = (self.fb.x, self.fb.y, self.fb.w, self.fb.h);
                                    self.fb.x = 0;
                                    self.fb.y = MENU_H;
                                    self.fb.w = wscr;
                                    self.fb.h = metade_h;
                                    self.fb.maximizada = true;
                                    log_msg("snap teclado ARQUIVOS: MAXIMIZA");
                                    self.notif.disparar(self.frame, "SISTEMA", "ARQUIVOS MAXIMIZADA (CTRL+CIMA)");
                                }
                                if d == 'D' {
                                    if self.fb.maximizada {
                                        let (px, py, pw, ph) = self.fb.pre_max;
                                        self.fb.x = px;
                                        self.fb.y = py;
                                        self.fb.w = pw;
                                        self.fb.h = ph;
                                        self.fb.maximizada = false;
                                        log_msg("snap teclado ARQUIVOS: RESTAURA");
                                        self.notif.disparar(self.frame, "SISTEMA", "ARQUIVOS RESTAURADA (CTRL+BAIXO)");
                                    } else {
                                        self.fb.minimizada = true;
                                        log_msg("snap teclado ARQUIVOS: MINIMIZA");
                                        self.notif.disparar(self.frame, "SISTEMA", "ARQUIVOS MINIMIZADA (CTRL+BAIXO)");
                                    }
                                }
                                self.draw();
                                return;
                            }
                            if self.notas.aberto && !self.notas.minimizada {
                                if d == 'L' {
                                    self.notas.x = 0;
                                    self.notas.y = MENU_H;
                                    self.notas.w = wscr / 2;
                                    self.notas.h = metade_h;
                                    self.notas.maximizada = false;
                                    log_msg("snap teclado NOTAS: ESQUERDA");
                                    self.notif.disparar(self.frame, "SISTEMA", "NOTAS METADE ESQUERDA (CTRL+SETA)");
                                }
                                if d == 'R' {
                                    self.notas.x = wscr / 2;
                                    self.notas.y = MENU_H;
                                    self.notas.w = wscr / 2;
                                    self.notas.h = metade_h;
                                    self.notas.maximizada = false;
                                    log_msg("snap teclado NOTAS: DIREITA");
                                    self.notif.disparar(self.frame, "SISTEMA", "NOTAS METADE DIREITA (CTRL+SETA)");
                                }
                                if d == 'U' {
                                    self.notas.pre_max = (self.notas.x, self.notas.y, self.notas.w, self.notas.h);
                                    self.notas.x = 0;
                                    self.notas.y = MENU_H;
                                    self.notas.w = wscr;
                                    self.notas.h = metade_h;
                                    self.notas.maximizada = true;
                                    log_msg("snap teclado NOTAS: MAXIMIZA");
                                    self.notif.disparar(self.frame, "SISTEMA", "NOTAS MAXIMIZADA (CTRL+CIMA)");
                                }
                                if d == 'D' {
                                    if self.notas.maximizada {
                                        let (px, py, pw, ph) = self.notas.pre_max;
                                        self.notas.x = px;
                                        self.notas.y = py;
                                        self.notas.w = pw;
                                        self.notas.h = ph;
                                        self.notas.maximizada = false;
                                        log_msg("snap teclado NOTAS: RESTAURA");
                                        self.notif.disparar(self.frame, "SISTEMA", "NOTAS RESTAURADA (CTRL+BAIXO)");
                                    } else {
                                        self.notas.minimizada = true;
                                        log_msg("snap teclado NOTAS: MINIMIZA");
                                        self.notif.disparar(self.frame, "SISTEMA", "NOTAS MINIMIZADA (CTRL+BAIXO)");
                                    }
                                }
                                self.draw();
                                return;
                            }
                            if self.calc.aberto && !self.calc.minimizada {
                                if d == 'L' {
                                    self.calc.x = 0;
                                    self.calc.y = MENU_H;
                                    self.calc.w = wscr / 2;
                                    self.calc.h = metade_h;
                                    self.calc.maximizada = false;
                                    log_msg("snap teclado CALC: ESQUERDA");
                                    self.notif.disparar(self.frame, "SISTEMA", "CALC METADE ESQUERDA (CTRL+SETA)");
                                }
                                if d == 'R' {
                                    self.calc.x = wscr / 2;
                                    self.calc.y = MENU_H;
                                    self.calc.w = wscr / 2;
                                    self.calc.h = metade_h;
                                    self.calc.maximizada = false;
                                    log_msg("snap teclado CALC: DIREITA");
                                    self.notif.disparar(self.frame, "SISTEMA", "CALC METADE DIREITA (CTRL+SETA)");
                                }
                                if d == 'U' {
                                    self.calc.pre_max = (self.calc.x, self.calc.y, self.calc.w, self.calc.h);
                                    self.calc.x = 0;
                                    self.calc.y = MENU_H;
                                    self.calc.w = wscr;
                                    self.calc.h = metade_h;
                                    self.calc.maximizada = true;
                                    log_msg("snap teclado CALC: MAXIMIZA");
                                    self.notif.disparar(self.frame, "SISTEMA", "CALC MAXIMIZADA (CTRL+CIMA)");
                                }
                                if d == 'D' {
                                    if self.calc.maximizada {
                                        let (px, py, pw, ph) = self.calc.pre_max;
                                        self.calc.x = px;
                                        self.calc.y = py;
                                        self.calc.w = pw;
                                        self.calc.h = ph;
                                        self.calc.maximizada = false;
                                        log_msg("snap teclado CALC: RESTAURA");
                                        self.notif.disparar(self.frame, "SISTEMA", "CALC RESTAURADA (CTRL+BAIXO)");
                                    } else {
                                        self.calc.minimizada = true;
                                        log_msg("snap teclado CALC: MINIMIZA");
                                        self.notif.disparar(self.frame, "SISTEMA", "CALC MINIMIZADA (CTRL+BAIXO)");
                                    }
                                }
                                self.draw();
                                return;
                            }
                            if self.cfg.aberto && !self.cfg.minimizada {
                                if d == 'L' {
                                    self.cfg.x = 0;
                                    self.cfg.y = MENU_H;
                                    self.cfg.w = wscr / 2;
                                    self.cfg.h = metade_h;
                                    self.cfg.maximizada = false;
                                    log_msg("snap teclado CONFIG: ESQUERDA");
                                    self.notif.disparar(self.frame, "SISTEMA", "CONFIG METADE ESQUERDA (CTRL+SETA)");
                                }
                                if d == 'R' {
                                    self.cfg.x = wscr / 2;
                                    self.cfg.y = MENU_H;
                                    self.cfg.w = wscr / 2;
                                    self.cfg.h = metade_h;
                                    self.cfg.maximizada = false;
                                    log_msg("snap teclado CONFIG: DIREITA");
                                    self.notif.disparar(self.frame, "SISTEMA", "CONFIG METADE DIREITA (CTRL+SETA)");
                                }
                                if d == 'U' {
                                    self.cfg.pre_max = (self.cfg.x, self.cfg.y, self.cfg.w, self.cfg.h);
                                    self.cfg.x = 0;
                                    self.cfg.y = MENU_H;
                                    self.cfg.w = wscr;
                                    self.cfg.h = metade_h;
                                    self.cfg.maximizada = true;
                                    log_msg("snap teclado CONFIG: MAXIMIZA");
                                    self.notif.disparar(self.frame, "SISTEMA", "CONFIG MAXIMIZADA (CTRL+CIMA)");
                                }
                                if d == 'D' {
                                    if self.cfg.maximizada {
                                        let (px, py, pw, ph) = self.cfg.pre_max;
                                        self.cfg.x = px;
                                        self.cfg.y = py;
                                        self.cfg.w = pw;
                                        self.cfg.h = ph;
                                        self.cfg.maximizada = false;
                                        log_msg("snap teclado CONFIG: RESTAURA");
                                        self.notif.disparar(self.frame, "SISTEMA", "CONFIG RESTAURADA (CTRL+BAIXO)");
                                    } else {
                                        self.cfg.minimizada = true;
                                        log_msg("snap teclado CONFIG: MINIMIZA");
                                        self.notif.disparar(self.frame, "SISTEMA", "CONFIG MINIMIZADA (CTRL+BAIXO)");
                                    }
                                }
                                self.draw();
                                return;
                            }
                        }
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
                                        self.sessao_salvar();
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
                    if !self.spot.aberto && self.term.aberto && !self.term.minimizada {
                        // M39: Ctrl+C copia o input, Ctrl+V cola no input
                        if tecla_pressionada(VK_CONTROL) {
                            if let winit::keyboard::Key::Character(c) = &event.logical_key {
                                if c.as_str().eq_ignore_ascii_case("c") {
                                    if clipboard_colar_texto(&self.term.input) {
                                        self.notif.disparar(self.frame, "TERMINAL", "INPUT COPIADO");
                                        log_msg("M39: input copiado do terminal");
                                    }
                                    self.draw();
                                    return;
                                }
                                if c.as_str().eq_ignore_ascii_case("v") {
                                    if let Some(txt) = clipboard_pegar() {
                                        for ch in txt.chars().take(200) {
                                            if ch.is_ascii_graphic() || ch == ' ' { self.term.input.push(ch); }
                                        }
                                        self.notif.disparar(self.frame, "TERMINAL", "COLADO NO INPUT");
                                        log_msg("M39: colado no terminal");
                                    }
                                    self.draw();
                                    return;
                                }
                            }
                        }
                        match &event.logical_key {
                            winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowUp) => {
                                self.term.historico_navegar(-1);
                                self.draw();
                                return;
                            }
                            winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowDown) => {
                                self.term.historico_navegar(1);
                                self.draw();
                                return;
                            }
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
                                layout_salvar("terminal", self.term.x, self.term.y, self.term.w, self.term.h);
                                self.term.aberto = false;
                                log_msg("terminal fechado (esc)");
                                self.draw();
                                return;
                            }
                            _ => {}
                        }
                    }
                    if !self.spot.aberto && self.cfg.aberto && !self.cfg.minimizada {
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
                            layout_salvar("config", self.cfg.x, self.cfg.y, self.cfg.w, self.cfg.h);
                            self.cfg.aberto = false;
                            log_msg("config fechado (esc)");
                        }
                        if consumiu_cfg || event.logical_key == winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape) {
                            self.draw();
                            return;
                        }
                    }
                    if !self.spot.aberto && self.notas.aberto && !self.notas.minimizada {
                        let ctrl = tecla_pressionada(VK_CONTROL);
                        if ctrl {
                            if let winit::keyboard::Key::Character(c) = &event.logical_key {
                                if c.as_str().eq_ignore_ascii_case("s") {
                                    self.notas.salvar();
                                    self.notif.disparar(self.frame, "NOTAS", "TEXTO SALVO NO DISCO");
                                    self.draw();
                                    return;
                                }
                                // M39: Ctrl+C copia a linha atual, Ctrl+V cola no fim dela
                                if c.as_str().eq_ignore_ascii_case("c") {
                                    let n = self.notas.linhas.len();
                                    if clipboard_colar_texto(&self.notas.linhas[n - 1]) {
                                        self.notif.disparar(self.frame, "NOTAS", "LINHA COPIADA");
                                        log_msg("M39: linha copiada das notas");
                                    }
                                    self.draw();
                                    return;
                                }
                                if c.as_str().eq_ignore_ascii_case("v") {
                                    if let Some(txt) = clipboard_pegar() {
                                        let limpo: String = txt.chars().filter(|ch| ch.is_ascii_graphic() || *ch == ' ').take(400).collect();
                                        let n = self.notas.linhas.len();
                                        self.notas.linhas[n - 1].push_str(&limpo);
                                        self.notas.sujo = true;
                                        self.notif.disparar(self.frame, "NOTAS", "TEXTO COLADO");
                                        log_msg("M39: colado nas notas");
                                    }
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
                                layout_salvar("notas", self.notas.x, self.notas.y, self.notas.w, self.notas.h);
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
                    if !self.spot.aberto && self.calc.aberto && !self.calc.minimizada {
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
                                layout_salvar("calc", self.calc.x, self.calc.y, self.calc.w, self.calc.h);
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
                    if !self.spot.aberto && self.fb.aberto && !self.fb.minimizada {
                        let mut consumiu = true;
                        match &event.logical_key {
                            winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowUp) => self.fb.mover(-1),
                            winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowDown) => self.fb.mover(1),
                            winit::keyboard::Key::Named(winit::keyboard::NamedKey::Enter) => self.fb.entrar_sel(),
                            winit::keyboard::Key::Named(winit::keyboard::NamedKey::Backspace) => self.fb.subir(),
                            winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape) => {
                                layout_salvar("arquivos", self.fb.x, self.fb.y, self.fb.w, self.fb.h);
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
                    if !self.spot.aberto {
                        if let winit::keyboard::Key::Character(c) = &event.logical_key {
                            if c.as_str().eq_ignore_ascii_case("i") {
                                self.abrir_janela();
                                self.draw();
                                return;
                            }
                            if c.as_str().eq_ignore_ascii_case("m") {
                                self.mission = !self.mission;
                                log_msg("mission control alternado");
                                self.draw();
                                return;
                            }
                            if c.as_str().eq_ignore_ascii_case("c") {
                                self.cfg.abrir();
                                log_msg("configuracoes aberta (tecla c)");
                                self.draw();
                                return;
                            }
                            if c.as_str().eq_ignore_ascii_case("n") {
                                self.notas.abrir();
                                log_msg("notas aberta (tecla n)");
                                self.draw();
                                return;
                            }
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
                            if let Some(idx) = self.janelas.iter().rposition(|j| !j.minimizada) {
                                self.janelas.remove(idx);
                                log_msg("janela do topo fechada (esc)");
                                self.draw();
                            }
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
        let ctrl = tecla_pressionada(VK_CONTROL);
        let espaco = tecla_pressionada(VK_SPACE);
        let agora = ctrl && espaco;
        if agora && !self.hotkey_prev && self.frame >= self.hotkey_cooldown {
            self.alternar_spot();
            log_msg("spotlight global alternado (ctrl+espaco)");
        }
        self.hotkey_prev = agora;
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
