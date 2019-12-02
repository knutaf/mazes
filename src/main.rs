use pixel_canvas::{Canvas, canvas::CanvasInfo, Color, image::Image, input::Event, input::MouseState};
mod grid;
use grid::{Grid, XY};

struct GridState {
    mouse_state: MouseState,
    grid: Grid<bool>,
    scale: usize,
}

impl GridState {
    fn new(width: usize, height: usize, scale: usize) -> GridState {
        GridState {
            grid: Grid::new(width, height, &false),
            mouse_state: MouseState::new(),
            scale: scale,
        }
    }

    fn handle_input(
        info: &CanvasInfo,
        state: &mut GridState,
        event: &Event<()>
        ) -> bool {
        MouseState::handle_input(info, &mut state.mouse_state, event)
    }
}

fn draw_vertical_line(
    image: &mut Image,
    scale: usize,
    x: usize,
    y1: usize,
    y2: usize,
    ) {
    for draw_y in (y1 * scale .. y2 * scale) {
        image[pixel_canvas::XY(x * scale, draw_y)] = Color {
            r: 255,
            g: 0,
            b: 0,
        };
    }
}

fn draw_horizontal_line(
    image: &mut Image,
    scale: usize,
    x1: usize,
    x2: usize,
    y: usize,
    ) {
    for draw_x in (x1 * scale .. x2 * scale) {
        image[pixel_canvas::XY(draw_x, y * scale)] = Color {
            r: 0,
            g: 0,
            b: 255,
        };
    }
}

fn main() {
    let scale = 50;
    let mut grid_state = GridState::new(10, 10, scale);
    let grid = &mut grid_state.grid;

    let width = grid.width();
    let height = grid.height();
    for (y, row) in grid.chunks_mut(width).enumerate() {
        for (x, cell) in row.iter_mut().enumerate() {
            *cell = (y == 0 || y == height - 1 ||
                     x == 0 || x == height - 1)
        }
    }

    // Configure the window that you want to draw in. You can add an event
    // handler to build interactive art. Input handlers for common use are
    // provided.
    let canvas = Canvas::new(grid.width() * scale, grid.height() * scale)
        .title("Tile")
        .state(grid_state)
        .input(GridState::handle_input)
        ;

    // The canvas will render for you at up to 60fps.
    canvas.render(|grid_state, image| {
        image.fill(Color { r: 255, g: 255, b: 255 });

        let grid = &grid_state.grid;
        for (y, row) in grid.chunks(grid.width()).enumerate() {
            for (x, cell) in row.iter().enumerate() {
                if y < grid.height() - 1 {
                    if *cell && grid[XY(x, y+1)] {
                        draw_vertical_line(image, grid_state.scale, x, y, y+1);
                    }
                }

                if x < grid.width() - 1 {
                    if *cell && grid[XY(x+1, y)] {
                        draw_horizontal_line(image, grid_state.scale, x, x+1, y);
                    }
                }
            }
        }
    });
}
