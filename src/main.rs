use pixel_canvas::{Canvas, Color, input::MouseState};

fn main() {
    // Configure the window that you want to draw in. You can add an event
    // handler to build interactive art. Input handlers for common use are
    // provided.
    let canvas = Canvas::new(512, 512)
        .title("Tile")
        .state(MouseState::new())
        .input(MouseState::handle_input)
        ;

    // The canvas will render for you at up to 60fps.
    canvas.render(|mouse, image| {
        // Modify the `image` based on your state.
        let width = image.width();
        for (y, row) in image.chunks_mut(width).enumerate() {
            for (x, pixel) in row.iter_mut().enumerate() {
                let is_black = x % 2 == 0 && y % 2 == 0;
                *pixel = Color {
                    r: if is_black { 0 } else { 255 },
                    g: if is_black { 0 } else { 255 },
                    b: if is_black { 0 } else { 255 },
                }
            }
        }
    });
}
