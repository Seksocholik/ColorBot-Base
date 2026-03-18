// You can disable cmd in easy way: (uncomment ts)
//#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter};
use libloading::{Library, Symbol};
use rand::Rng;
use opencv::{core, imgproc, Result as OcvResult};
use serde::{Deserialize, Serialize};
use eframe::egui;

#[macro_use] // for obfuscation
mod obfuscation;
mod screen_capture;

// config name
const CONFIG_FILENAME: &str = "config.json";

// serialize, deserialize macro
#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    pub lower_color_h: f64, 
    pub lower_color_s: f64, 
    pub lower_color_v: f64,
    pub upper_color_h: f64, 
    pub upper_color_s: f64, 
    pub upper_color_v: f64,
    pub use_bezier: bool,
    pub color_preset: usize,
}

// set default config.
impl Default for Config {
    fn default() -> Self {
        Self {
            lower_color_h: 140.0,
            lower_color_s: 110.0,
            lower_color_v: 150.0,
            upper_color_h: 150.0,
            upper_color_s: 195.0,
            upper_color_v: 255.0,
            use_bezier: false,
            color_preset: 0,
        }
    }
}

// loads config.
fn load_config() -> Config {
    match File::open(CONFIG_FILENAME) {
        Ok(file) => {
            let reader = BufReader::new(file);
            match serde_json::from_reader(reader) {
                Ok(config) => config,
                Err(_) => {
                    let cfg = Config::default();
                    let _ = save_config(&cfg);
                    cfg
                }
            }
        }
        // if there is no config, create it as default
        Err(_) => {
            let cfg = Config::default();
            let _ = save_config(&cfg);
            cfg
        }
    }
}

// save current config
fn save_config(config: &Config) -> std::io::Result<()> {
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(CONFIG_FILENAME)?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, config)?;
    Ok(())
}

fn apply_color_template(cfg: &mut Config, template: &str) {
    match template {
        "Purple" => {
            cfg.lower_color_h = 140.0; cfg.lower_color_s = 110.0; cfg.lower_color_v = 150.0;
            cfg.upper_color_h = 150.0; cfg.upper_color_s = 195.0; cfg.upper_color_v = 255.0;
        }
        "Yellow" => {
            cfg.lower_color_h = 30.0; cfg.lower_color_s = 170.0; cfg.lower_color_v = 170.0;
            cfg.upper_color_h = 30.0; cfg.upper_color_s = 255.0; cfg.upper_color_v = 255.0;
        }
        "Red" => {
            cfg.lower_color_h = 0.0; cfg.lower_color_s = 190.0; cfg.lower_color_v = 150.0;
            cfg.upper_color_h = 10.0; cfg.upper_color_s = 255.0; cfg.upper_color_v = 255.0;
        }
        // anti astra is shit, just make ur own system such as like "L2 Hue Filter"
        "Anti-Astra" => {
            cfg.lower_color_h = 140.0; cfg.lower_color_s = 130.0; cfg.lower_color_v = 180.0;
            cfg.upper_color_h = 150.0; cfg.upper_color_s = 255.0; cfg.upper_color_v = 255.0;
        }
        _ => {}
    }
}

// bezier helpers 

#[derive(Clone, Copy, Debug)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self { Self { x, y } }
}

#[derive(Clone, Copy, Debug)]
pub struct CubicBez {
    pub p0: Point,
    pub p1: Point,
    pub p2: Point,
    pub p3: Point,
}

impl CubicBez {
    pub fn new(p0: Point, p1: Point, p2: Point, p3: Point) -> Self {
        Self { p0, p1, p2, p3 }
    }
}

// https://github.com/ivan-guerra/colorbot/blob/master/src/lib.rs
// line 100+
pub fn mouse_bez(init_pos: Point, fin_pos: Point, deviation: u32) -> CubicBez {
    let mut rng = rand::thread_rng();
    
    let dx = fin_pos.x - init_pos.x;
    let dy = fin_pos.y - init_pos.y;
    let dist = (dx * dx + dy * dy).sqrt();
    
    let max_dev = dist * (deviation as f64 / 50.0);
    let ctrl1_offset = rng.gen_range(max_dev * 0.5..=max_dev);
    let ctrl2_offset = rng.gen_range(max_dev * 0.3..=max_dev * 0.8);
    
    let angle = dy.atan2(dx);
    let ctrl1_angle = angle + rng.gen_range(-0.8..0.8);
    let ctrl2_angle = angle + rng.gen_range(-0.5..0.5);
    
    let control_1 = Point::new(
        init_pos.x + ctrl1_offset * ctrl1_angle.cos(),
        init_pos.y + ctrl1_offset * ctrl1_angle.sin()
    );
    
    let control_2 = Point::new(
        fin_pos.x - ctrl2_offset * ctrl2_angle.cos(),
        fin_pos.y - ctrl2_offset * ctrl2_angle.sin()
    );

    CubicBez::new(init_pos, control_1, control_2, fin_pos)
}


// UI 
struct UI {
    config: Arc<Mutex<Config>>,
}

impl UI {
    fn new(config: Arc<Mutex<Config>>) -> Self {
        Self { config }
    }
}
// simple helpers 
// you dont need a set hidden system attribute but if you want to hide anything from user, here is example 
fn set_hidden(path: &std::path::Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use std::ffi::OsStr;
    
    let wide_path: Vec<u16> = OsStr::new(path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    
    let result = unsafe {
        DYN_SET_FILE_ATTRIBUTES_W(wide_path.as_ptr(), 0x06)
    };
    
    if result == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn show_msg_box(text: &str, title: &str) {
    use std::os::windows::ffi::OsStrExt;
    use std::ffi::OsStr;
    
    let wide_text: Vec<u16> = OsStr::new(text).encode_wide().chain(std::iter::once(0)).collect();
    let wide_title: Vec<u16> = OsStr::new(title).encode_wide().chain(std::iter::once(0)).collect();
    
    unsafe {
        DYN_MESSAGE_BOX_W(std::ptr::null(), wide_text.as_ptr(), wide_title.as_ptr(), 0x00000040); 
    }
}

fn show_msg_popup_yesno(text: &str, title: &str) -> bool {
    use std::os::windows::ffi::OsStrExt;
    use std::ffi::OsStr;
    
    let wide_text: Vec<u16> = OsStr::new(text).encode_wide().chain(std::iter::once(0)).collect();
    let wide_title: Vec<u16> = OsStr::new(title).encode_wide().chain(std::iter::once(0)).collect();
    
    const MB_YESNO: u32 = 0x00000004;
    const MB_ICONQUESTION: u32 = 0x00000020;
    const IDYES: i32 = 6;
    
    let result = unsafe {
        DYN_MESSAGE_BOX_W(std::ptr::null(), wide_text.as_ptr(), wide_title.as_ptr(), MB_YESNO | MB_ICONQUESTION)
    };
    result == IDYES
}

type MessageBoxWFn = unsafe extern "system" fn(*const std::ffi::c_void, *const u16, *const u16, u32) -> i32;
type SetFileAttributesWFn = unsafe extern "system" fn(*const u16, u32) -> i32;
type BeepFn = unsafe extern "system" fn(u32, u32) -> i32;

lazy_static::lazy_static! {
    static ref DYN_USER32: Library = unsafe { Library::new(obf_string!("user32.dll")).expect("failed to load user32") };
    static ref DYN_KERNEL32: Library = unsafe { Library::new(obf_string!("kernel32.dll")).expect("failed to load kernel32") };

    pub static ref DYN_MESSAGE_BOX_W: Symbol<'static, MessageBoxWFn> = unsafe {
        DYN_USER32.get(b"MessageBoxW\0").unwrap()
    };
    
    pub static ref DYN_SET_FILE_ATTRIBUTES_W: Symbol<'static, SetFileAttributesWFn> = unsafe {
        DYN_KERNEL32.get(b"SetFileAttributesW\0").unwrap()
    };

    pub static ref DYN_BEEP: Symbol<'static, BeepFn> = unsafe {
        DYN_KERNEL32.get(b"Beep\0").unwrap()
    };

    // example with using obf string (xor)
    static ref APP_DATA_DIR: PathBuf = {
        obfuscation::junk_code_2();
        let local_app_data = std::env::var(obf_string!("LOCALAPPDATA")).unwrap_or(obf_string!("C:\\Windows\\Temp"));
        let mut path = PathBuf::from(local_app_data);
        path.push(obf_string!("example"));
        obfuscation::stack_noise();
        path
    };
    
    // define dll path to use it later
    static ref EXAMPLE_DLL: Result<PathBuf, std::io::Error> = {
        // here we put junk code near that
        obfuscation::junk_code_1();
        // fake api calls 
        let _fake = obfuscation::fake_operations(0xCAFEBABE);
        obfuscation::stack_noise();
        // u can change path from appdata to any in line 122
        Ok(APP_DATA_DIR.join(obf_string!("example.dll")))
    };
    
    // it will open a config.json and put default settings
    static ref GLOBAL_CONFIG: Arc<Mutex<Config>> = Arc::new(Mutex::new(load_config()));
}


fn example_opencv() -> OcvResult<()> {
    //
    // THIS IS NOT A EXAMPLE FOR COLORBOT CALCULATIONS, IMMA MAKE IT IN NEXT UPDATES ON GITHUB.
    //
    let src_mat = core::Mat::new_rows_cols_with_default(
        100, 
        100, 
        core::CV_8UC3, 
        core::Scalar::all(0.0)
    )?;
    
    let mut hsv_mat = core::Mat::default();
    
    imgproc::cvt_color(
        &src_mat,
        &mut hsv_mat,
        imgproc::COLOR_BGR2HSV,
        0,
        core::AlgorithmHint::ALGO_HINT_DEFAULT,
    )?;

    println!("{}", obf_string!("all good"));
    Ok(())
}

impl eframe::App for UI {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut cfg = self.config.lock().unwrap();
        let mut changed = false;

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(obf_string!("header"));
            ui.separator();
            /*
            ui.horizontal(|ui| {
                ui.label("slider");
                if ui.add(egui::Slider::new(&mut cfg.fov, 10.0..=500.0)).changed() {
                    changed = true;
                }
                // numeric input near slider to enter values not only sliders
                if ui.add(egui::DragValue::new(&mut cfg.fov).speed(1.0)).changed() {
                    changed = true;
                }
            });
                */
            ui.add_space(10.0);

            // checkbox
            if ui.checkbox(&mut cfg.use_bezier, "Use Bezier").changed() {
                changed = true;
            }

            ui.add_space(10.0);

            // radio
            ui.label("Color Preset:");
            ui.horizontal(|ui| {
                if ui.radio_value(&mut cfg.color_preset, 0, "Purple").changed() { changed = true; }
                if ui.radio_value(&mut cfg.color_preset, 1, "Yellow").changed() { changed = true; }
                if ui.radio_value(&mut cfg.color_preset, 2, "Red").changed() { changed = true; }
            });

            ui.add_space(20.0);
            
            // optional save cfg
            if ui.button("Save Config").clicked() {
                let _ = save_config(&cfg);
                println!("{}", obf_string!("config saved."));
            }
        });

        // auto save
        if changed {
            let _ = save_config(&cfg);
        }
    }
}


fn main() {
    println!("{}", obf_string!("Hello World!"));
    
    // initialization config, it will create a config.json
    let _cfg = GLOBAL_CONFIG.lock().unwrap();
    drop(_cfg); 

    // example box
    // show_msg_box("popup", "hey");
    
    // example
    if let Err(e) = example_opencv() {
        println!("error opencv: {}", e);
    }

    // example
    // let (w, h) = screen_capture::get_screen_size();
    // println!("Rozdzielczość ekranu: {}x{}", w, h);
    
    // ui run
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_title("base ui"),
        ..Default::default()
    };

    eframe::run_native(
        "ColorBotBase",
        options,
        Box::new(|_cc| Box::new(UI::new(GLOBAL_CONFIG.clone())) as Box<dyn eframe::App>),
    ).unwrap();
}
