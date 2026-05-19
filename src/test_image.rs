use eframe::egui;

fn test(ui: &mut egui::Ui, texture: &egui::TextureHandle) {
    ui.add(
        egui::Image::new(texture).fit_to_exact_size(ui.available_size())
    );
}
