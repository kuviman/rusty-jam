use super::*;

struct PlayerState {
    step_animation: f32,
}

impl PlayerState {
    pub fn new() -> Self {
        Self {
            step_animation: 0.0,
        }
    }
    pub fn update(&mut self, player: &Player, delta_time: f32) {}
}

impl Default for PlayerState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct GameState {
    geng: Rc<Geng>,
    assets: Rc<Assets>,
    opt: Rc<Opt>,
    camera: Camera,
    renderer: Renderer,
    model: Model,
    player: Player,
    players: HashMap<Id, PlayerState>,
    connection: Connection,
    transition: Option<geng::Transition>,
    to_send: Vec<ClientMessage>,
    framebuffer_size: Vec2<f32>,
}

impl Drop for GameState {
    fn drop(&mut self) {
        if let Connection::Remote(connection) = &mut self.connection {
            connection.send(ClientMessage::Event(Event::PlayerLeft(self.player.id)));
        }
    }
}

impl GameState {
    pub fn new(
        geng: &Rc<Geng>,
        assets: &Rc<Assets>,
        opt: &Rc<Opt>,
        player: Option<Player>,
        welcome: WelcomeMessage,
        connection: Connection,
    ) -> Self {
        let player = match player {
            Some(mut player) => {
                player.id = welcome.player_id;
                player
            }
            None => welcome.model.players[&welcome.player_id].clone(),
        };
        Self {
            geng: geng.clone(),
            assets: assets.clone(),
            opt: opt.clone(),
            camera: Camera::new(10.0),
            renderer: Renderer::new(geng),
            player,
            players: HashMap::new(),
            model: welcome.model,
            connection,
            transition: None,
            to_send: Vec::new(),
            framebuffer_size: vec2(1.0, 1.0),
        }
    }

    fn draw_player(&self, framebuffer: &mut ugli::Framebuffer, player: &Player) {
        self.renderer.draw(
            framebuffer,
            &self.camera,
            Mat4::translate(player.position.extend(0.0)) * Mat4::translate(vec3(-0.5, -0.5, 0.0)),
            Some(&self.assets.player),
            Color::WHITE,
        );
        self.renderer.draw(
            framebuffer,
            &self.camera,
            Mat4::translate(player.position.extend(0.0) + vec3(0.0, 0.7, 0.0))
                * Mat4::scale(vec3(1.5 * player.oxygen / Player::MAX_OXYGEN, 0.1, 1.0))
                * Mat4::translate(vec3(-0.5, -0.5, 0.0)),
            None,
            Color::WHITE,
        );
    }

    fn draw_item(&self, framebuffer: &mut ugli::Framebuffer, item: &Item) {
        self.renderer.draw(
            framebuffer,
            &self.camera,
            Mat4::translate(item.position.extend(0.0))
                * Mat4::scale_uniform(0.2)
                * Mat4::translate(vec3(-0.5, -0.5, 0.0)),
            Some(&self.assets.player),
            Color::GRAY,
        );
    }

    fn draw_impl(&mut self, framebuffer: &mut ugli::Framebuffer) {
        ugli::clear(framebuffer, Some(Color::rgb(0.05, 0.05, 0.2)), None);
        self.draw_player(framebuffer, &self.player);
        for player in self.model.players.values() {
            if player.id != self.player.id {
                self.draw_player(framebuffer, player);
            }
        }
        for item in self.model.items.values() {
            self.draw_item(framebuffer, item);
        }
    }
    fn update_player(&mut self, delta_time: f32) {
        self.player.target_velocity = vec2(0.0, 0.0);
        if self.geng.window().is_key_pressed(geng::Key::A)
            || self.geng.window().is_key_pressed(geng::Key::Left)
        {
            self.player.target_velocity.x -= 1.0;
        }
        if self.geng.window().is_key_pressed(geng::Key::D)
            || self.geng.window().is_key_pressed(geng::Key::Right)
        {
            self.player.target_velocity.x += 1.0;
        }
        if self.geng.window().is_key_pressed(geng::Key::W)
            || self.geng.window().is_key_pressed(geng::Key::Up)
        {
            self.player.target_velocity.y += 1.0;
        }
        if self.geng.window().is_key_pressed(geng::Key::S)
            || self.geng.window().is_key_pressed(geng::Key::Down)
        {
            self.player.target_velocity.y -= 1.0;
        }
        self.player.update(delta_time);
        for item in self.model.items.values() {
            if (self.player.position - item.position).len() < 0.5 {
                self.to_send.push(ClientMessage::Event(Event::PickUpItem {
                    item_id: item.id,
                    player_id: self.player.id,
                }));
            }
        }
    }
}

impl geng::State for GameState {
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        self.draw_impl(framebuffer);
    }
    fn update(&mut self, delta_time: f64) {
        self.camera.target_position = self.player.position;
        self.camera.update(delta_time as f32);
        let mut messages = Vec::new();
        match &mut self.connection {
            Connection::Remote(connection) => messages.extend(connection.new_messages()),
            Connection::Local { next_tick, model } => {
                *next_tick -= delta_time;
                while *next_tick <= 0.0 {
                    messages.push(ServerMessage::Update(model.tick()));
                    *next_tick += 1.0 / model.ticks_per_second;
                }
            }
        }
        let mut messages_to_send = mem::replace(&mut self.to_send, Vec::new());
        if !messages.is_empty() {
            messages_to_send.push(ClientMessage::Event(Event::PlayerUpdated(
                self.player.clone(),
            )));
        }
        for message in messages_to_send {
            match &mut self.connection {
                Connection::Remote(connection) => connection.send(message),
                Connection::Local {
                    next_tick: _,
                    model,
                } => {
                    messages.push(ServerMessage::Update(
                        model.handle_message(self.player.id, message),
                    ));
                }
            }
        }
        for message in messages {
            match message {
                ServerMessage::Update(events) => {
                    for event in events {
                        match event {
                            Event::PlayerDied(ref player) if player.id == self.player.id => {
                                self.player = player.clone();
                            }
                            _ => {}
                        }
                        self.model.handle(event);
                    }
                }
                _ => unreachable!(),
            }
        }
        let delta_time = delta_time as f32;
        self.update_player(delta_time);
        for player in self.model.players.values_mut() {
            player.update(delta_time);
        }

        for player in self.model.players.values() {
            if player.id == self.player.id {
                continue;
            }
            self.players
                .entry(player.id)
                .or_default()
                .update(player, delta_time);
        }
        self.players
            .entry(self.player.id)
            .or_default()
            .update(&self.player, delta_time);
    }
    fn handle_event(&mut self, event: geng::Event) {
        match event {
            geng::Event::Wheel { delta } => {
                self.camera.target_fov = clamp(
                    self.camera.target_fov * 2f32.powf(-delta as f32 * 0.01),
                    2.0..=100.0,
                );
            }
            _ => {}
        }
    }
    fn transition(&mut self) -> Option<geng::Transition> {
        self.transition.take()
    }
}
