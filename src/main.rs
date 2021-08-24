use geng::prelude::*;

pub mod camera;
pub mod game_state;
pub mod lobby;
pub mod model;
pub mod net;
pub mod renderer;
#[cfg(not(target_arch = "wasm32"))]
pub mod server;

pub use camera::*;
pub use game_state::GameState;
pub use lobby::*;
pub use model::*;
pub use net::*;
pub use renderer::*;
#[cfg(not(target_arch = "wasm32"))]
pub use server::Server;

pub fn hsv(h: f32, s: f32, v: f32) -> Color<f32> {
    hsva(h, s, v, 1.0)
}
pub fn hsva(mut h: f32, s: f32, v: f32, a: f32) -> Color<f32> {
    h -= h.floor();
    let r;
    let g;
    let b;
    let f = h * 6.0 - (h * 6.0).floor();
    let p = v * (1.0 - s);
    let q = v * (1.0 - f * s);
    let t = v * (1.0 - (1.0 - f) * s);
    if h * 6.0 < 1.0 {
        r = v;
        g = t;
        b = p;
    } else if h * 6.0 < 2.0 {
        r = q;
        g = v;
        b = p;
    } else if h * 6.0 < 3.0 {
        r = p;
        g = v;
        b = t;
    } else if h * 6.0 < 4.0 {
        r = p;
        g = q;
        b = v;
    } else if h * 6.0 < 5.0 {
        r = t;
        g = p;
        b = v;
    } else {
        r = v;
        g = p;
        b = q;
    }
    Color::rgba(r, g, b, a)
}

#[derive(Deref)]
pub struct Font {
    #[deref]
    inner: Rc<geng::Font>,
}

impl geng::LoadAsset for Font {
    fn load(geng: &Rc<Geng>, path: &str) -> geng::AssetFuture<Self> {
        let geng = geng.clone();
        <Vec<u8> as geng::LoadAsset>::load(&geng, path)
            .map(move |data| {
                Ok(Font {
                    inner: Rc::new(geng::Font::new(&geng, data?)?),
                })
            })
            .boxed_local()
    }
    const DEFAULT_EXT: Option<&'static str> = Some("ttf");
}

#[derive(geng::Assets)]
pub struct Assets {
    pub player: ugli::Texture,
    pub font: Rc<Font>,
}

#[derive(StructOpt)]
pub struct Opt {
    #[structopt(long)]
    addr: Option<String>,
    #[structopt(long)]
    server: bool,
    #[structopt(long)]
    with_server: bool,
}

impl Opt {
    pub fn addr(&self) -> &str {
        match &self.addr {
            Some(addr) => addr,
            None => option_env!("SERVER_ADDR").unwrap_or("127.0.0.1:1155"),
        }
    }
}

fn main() {
    logger::init().unwrap();
    geng::setup_panic_handler();
    if let Some(dir) = std::env::var_os("CARGO_MANIFEST_DIR") {
        std::env::set_current_dir(std::path::Path::new(&dir).join("static")).unwrap();
    } else {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(path) = std::env::current_exe().unwrap().parent() {
                std::env::set_current_dir(path).unwrap();
            }
        }
    }
    let opt: Opt = StructOpt::from_args();
    let opt = Rc::new(opt);
    if opt.server {
        #[cfg(not(target_arch = "wasm32"))]
        Server::new(opt.addr(), Model::new()).run();
    } else {
        #[cfg(not(target_arch = "wasm32"))]
        let server = if opt.with_server {
            let server = Server::new(opt.addr(), Model::new());
            let server_handle = server.handle();
            let server_thread = std::thread::spawn(move || {
                server.run();
            });
            Some((server_handle, server_thread))
        } else {
            None
        };
        let geng = Rc::new(Geng::new(geng::ContextOptions {
            title: "RustyJam - TODO by kuviman".to_owned(),
            ..default()
        }));
        let assets = <Assets as geng::LoadAsset>::load(&geng, ".");
        geng::run(
            geng.clone(),
            geng::LoadingScreen::new(&geng, geng::EmptyLoadingScreen, assets, {
                let geng = geng.clone();
                move |assets| {
                    let mut assets = assets.unwrap();
                    ConnectingState::new(&geng, &Rc::new(assets), &opt, None)
                    // let mut model = Model::new();
                    // let (welcome, _) = model.welcome();
                    // GameState::new(
                    //     &geng,
                    //     &Rc::new(assets),
                    //     &opt,
                    //     None,
                    //     welcome,
                    //     Connection::Local {
                    //         next_tick: 0.0,
                    //         model,
                    //     },
                    // )
                    // Lobby::new(&geng, Rc::new(assets), &opt)
                }
            }),
        );
        #[cfg(not(target_arch = "wasm32"))]
        if let Some((server_handle, server_thread)) = server {
            server_handle.shutdown();
            server_thread.join().unwrap();
        }
    }
}
