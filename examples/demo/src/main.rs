use eframe::egui;
use egui_curve_editor::{Curve, CurveEditor};

// TODO: Remove main file
fn main() {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 280.0]),
        ..Default::default()
    };

    let mut curve = Curve::linear();

    let start = std::time::Instant::now();
    let mut now = start;

    eframe::run_simple_native("Curve Editor Demo", options, move |ctx, _frame| {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Curve Editor");
            let elapsed = start.elapsed().as_secs_f32() % 10.0;
            ui.label(format!(
                "Sample at {:.3}: {:.3}",
                elapsed / 10.0,
                curve.sample(elapsed / 10.0)
            ));
            ui.label(format!("frame time: {}ms", now.elapsed().as_millis()));
            now = std::time::Instant::now();

            ui.add(CurveEditor::new(&mut curve).with_max_size(egui::vec2(400.0, 100.0)));

            ui.label("sample text to test height of widget");

            ctx.request_repaint();
        });
    })
    .expect("Failed!");
}
