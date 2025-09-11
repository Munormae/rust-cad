use cad_render::AppState;

struct EguiApp {
    app: AppState,
}

impl eframe::App for EguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.app.ui(ctx);
    }
}

fn main() -> eframe::Result<()> {
    let viewport = egui::ViewportBuilder::default()
        .with_title("rust-cad")
        .with_inner_size([1280.0, 800.0]);

    let opts = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "rust-cad",
        opts,
        Box::new(|_cc| {
            Ok(Box::new(EguiApp {
                app: AppState::default(),
            }))
        }),
    )
}
