extern crate pixel_canvas;
extern crate rand;

use pixel_canvas::{
    Canvas,
    canvas::CanvasInfo,
    Color,
    image::Image,
    input::{
        Event,
        MouseState,
        WindowEvent,
        glutin::event::{
            KeyboardInput,
            ElementState,
            VirtualKeyCode,
        },
    },
    };
use rand::Rng;

mod grid;
use grid::{Grid, XY};

const SCALE_IN_PX: usize = 50;
const CELL_FILL_MARGIN_IN_PX: usize  = 10;
const EDGE_THICKNESS_IN_PX: usize = 5;

fn draw_box(
    image: &mut Image,
    x1: usize,
    y1: usize,
    x2: usize,
    y2: usize,
    color: &Color
    ) {
    for draw_x in x1 .. x2 {
        for draw_y in y1 .. y2 {
            image[pixel_canvas::XY(draw_x, draw_y)] = *color;
        }
    }
}

#[derive(Clone)]
enum GridCellKind {
    Empty,
    Path(usize),
    End,
}

#[derive(Clone, PartialEq)]
enum EdgeState {
    Unset,
    Off,
    ProvisionallyOn,
    On,
}

#[derive(Clone)]
struct GridCell {
    kind: GridCellKind,
    left_edge: EdgeState,
    bottom_edge: EdgeState,
}

#[derive(Clone)]
enum Command {
    Exit,
    Refresh,
}

type CellGrid = Grid<GridCell>;
struct GridState {
    mouse_state: MouseState,
    grid: CellGrid,
    path: Vec<XY>,
    next_command: Option<Command>,
}

impl GridCell {
    fn new() -> GridCell {
        GridCell {
            kind: GridCellKind::Empty,
            left_edge: EdgeState::Unset,
            bottom_edge: EdgeState::Unset,
        }
    }

    fn has_left_edge(&self) -> bool {
        match self.left_edge {
            EdgeState::On | EdgeState::ProvisionallyOn => true,
            _ => false,
        }
    }

    fn has_bottom_edge(&self) -> bool {
        match self.bottom_edge {
            EdgeState::On | EdgeState::ProvisionallyOn => true,
            _ => false,
        }
    }
}

impl GridState {
    fn new(width: usize, height: usize, path_point_count: usize) -> GridState {
        let grid = Self::create_grid(width, height, path_point_count);
        let path = Self::extract_path(&grid);

        GridState {
            grid: grid,
            path: path,
            mouse_state: MouseState::new(),
            next_command: None,
        }
    }

    fn create_grid(width: usize, height: usize, path_point_count: usize) -> CellGrid {
        let mut grid = CellGrid::new(width, height, &GridCell::new());

        // Turn on walls at the borders.
        for (y, row) in grid.chunks_mut(width).enumerate() {
            for (x, cell) in row.iter_mut().enumerate() {
                cell.bottom_edge = if (x != width - 1) && (y == 0 || y == height - 1) { EdgeState::On } else { EdgeState::Unset };
                cell.left_edge = if (y != height - 1) && (x == 0 || x == width - 1) { EdgeState::On } else { EdgeState::Unset };
            }
        }

        #[derive(Clone, Debug)]
        enum Direction {
            Up,
            Down,
            Left,
            Right,
        }

        #[derive(Clone)]
        struct PathPoint {
            point: XY,
            dir: Direction,
        }

        // Create the correct path through the maze.
        let mut path = Vec::<PathPoint>::new();

        // For now choose the start point in a corner
        let start = {
            let start_bottom = rand::thread_rng().gen_bool(0.5);
            let start_left = rand::thread_rng().gen_bool(0.5);
            let start_points_vertical = rand::thread_rng().gen_bool(0.5);

            PathPoint {
                point: XY(if start_left { 0 } else { width - 2 }, if start_bottom { 0 } else { height - 2 }),
                dir: if start_points_vertical {
                         if start_bottom {
                             Direction::Up
                         }
                         else {
                             Direction::Down
                         }
                     }
                     else {
                         if start_left {
                             Direction::Right
                         }
                         else {
                             Direction::Left
                         }
                     },
            }
        };

        println!("start point ({}, {}, {:?})", start.point.0, start.point.1, start.dir);

        path.push(start.clone());

        let mut iter = 0;
        while path.len() < path_point_count {
            loop {
                let start_left = start.point.0 == 0;
                let start_bottom = start.point.1 == 0;
                let last = &path.last().unwrap();

                // Alternate directions. respect whether moving from left to right and up to down
                // or the other way.
                let dir =
                    match last.dir {
                        Direction::Up | Direction::Down => {
                            if start_left {
                                Direction::Right
                            }
                            else {
                                Direction::Left
                            }
                        },
                        Direction::Left | Direction::Right => {
                            if start_bottom {
                                Direction::Up
                            }
                            else {
                                Direction::Down
                            }
                        },
                    };

                let mut should_reset_path = false;

                match dir {
                    Direction::Up if last.point.1 == height - 2 => should_reset_path = true,
                    Direction::Down if last.point.1 == 0 => should_reset_path = true,
                    Direction::Left if last.point.0 == 0 => should_reset_path = true,
                    Direction::Right if last.point.0 == width - 2 => should_reset_path = true,
                    _ => ()
                };

                if !should_reset_path {
                    let point =
                        match last.dir {
                            Direction::Up => XY(last.point.0, rand::thread_rng().gen_range(last.point.1, height - 1)),
                            Direction::Down => XY(last.point.0, rand::thread_rng().gen_range(0, last.point.1)),
                            Direction::Left => XY(rand::thread_rng().gen_range(0, last.point.0), last.point.1),
                            Direction::Right => XY(rand::thread_rng().gen_range(last.point.0, width - 1), last.point.1),
                        };

                    println!("considering point ({}, {}, {:?})", point.0, point.1, dir);

                    let is_valid =
                        path.iter().find(|&item| { item.point == point }).is_none() &&
                        ((path.len() != path_point_count - 1) || Self::is_valid_start_or_end(&grid, &point));

                    if is_valid {
                        path.push(PathPoint { point, dir });
                        println!("added. len now {}", path.len());
                        break;
                    }
                    else if path.len() == path_point_count - 1 {
                        should_reset_path = true;
                    }
                }

                if should_reset_path {
                    println!("resetting iteration {}, path length {} out of {}. start point ({}, {}, {:?}). last point ({}, {}, {:?})", iter, path.len(), path_point_count, start.point.0, start.point.1, start.dir, last.point.0, last.point.1, last.dir);
                    iter += 1;
                    path.clear();
                    path.push(start.clone());
                }
            }
        }

        // if cornered (can't even travel one step in any direction), then restart from the start point
        // pick a random direction and random distance
        //    but if this is the last point, then the distance must extend all the way to a wall
        // check if point is valid on the path
        // add point to path

        // Now set the path value on every cell in the chosen path.
        let len = path.len();
        for (i, point) in path.iter_mut().enumerate() {
            match i {
                0 => {
                    grid[point.point.clone()].kind = GridCellKind::Path(i);
                    Self::erase_start_or_end_edge(&mut grid, &point.point);
                },
                _ if i == len - 1 => {
                    grid[point.point.clone()].kind = GridCellKind::End;
                    Self::erase_start_or_end_edge(&mut grid, &point.point);
                },
                _ => {
                    grid[point.point.clone()].kind = GridCellKind::Path(i);
                },
            };
        }

        // Walk the path and erase edges to ensure the path is open.
        path.iter().fold(None, |last_opt : Option<&PathPoint>, step| {
            if let Some(last) = last_opt {
                let XY(mut x, mut y) = last.point;
                while x != step.point.0 || y != step.point.1 {
                    match last.dir {
                        Direction::Up => {
                            grid[XY(x, y+1)].bottom_edge = EdgeState::Off;
                            y += 1;
                        },
                        Direction::Down => {
                            grid[XY(x, y)].bottom_edge = EdgeState::Off;
                            y -= 1;
                        },
                        Direction::Left => {
                            grid[XY(x, y)].left_edge = EdgeState::Off;
                            x -= 1;
                        },
                        Direction::Right => {
                            grid[XY(x+1, y)].left_edge = EdgeState::Off;
                            x += 1;
                        },
                    }
                }
            }

            Some(step)
        });

        iter = 0;
        while !Self::has_valid_edges(&grid) {
            // Reset all provisionally-on edges to unset to try again.
            for cell in grid.iter_mut() {
                if cell.bottom_edge == EdgeState::ProvisionallyOn {
                    cell.bottom_edge = EdgeState::Unset;
                }

                if cell.left_edge == EdgeState::ProvisionallyOn {
                    cell.left_edge = EdgeState::Unset;
                }
            }

            // Turn on edges randomly
            for cell in grid.iter_mut() {
                if cell.bottom_edge == EdgeState::Unset && rand::thread_rng().gen_bool(0.5) {
                    cell.bottom_edge = EdgeState::ProvisionallyOn;
                }

                if cell.left_edge == EdgeState::Unset && rand::thread_rng().gen_bool(0.5) {
                    cell.left_edge = EdgeState::ProvisionallyOn;
                }
            }

            iter += 1;
        }

        println!("took {} iterations to get valid borders", iter);

        // Make all the provisional edges real
        for cell in grid.iter_mut() {
            if cell.bottom_edge == EdgeState::ProvisionallyOn {
                cell.bottom_edge = EdgeState::On;
            }

            if cell.left_edge == EdgeState::ProvisionallyOn {
                cell.left_edge = EdgeState::On;
            }
        }

        grid
    }

    fn erase_start_or_end_edge(grid: &mut CellGrid, XY(x, y): &XY) {
        let x = *x;
        let y = *y;

        let has_vertical_edge = x == 0 || x == grid.width() - 2;
        let has_horizontal_edge = y == 0 || y == grid.height() - 2;

        // Decide whether to erase a vertical edge or horizontal edge. The chosen edge can only be
        // erased if the cell has such an edge available to erase, so keep looping until the intent
        // and available edge lines up.
        let mut erase_vertical_edge = rand::thread_rng().gen_bool(0.5);
        while erase_vertical_edge != has_vertical_edge && !erase_vertical_edge != has_horizontal_edge {
            erase_vertical_edge = rand::thread_rng().gen_bool(0.5);
        }

        // Erasing a vertical edge on the right border means going to the next cell over (just
        // outside the border) and erasing the left edge.
        if erase_vertical_edge {
            if x == grid.width() - 2 {
                grid[XY(x + 1, y)].left_edge = EdgeState::Off;
            } else {
                grid[XY(x, y)].left_edge = EdgeState::Off;
            }
        }

        // Erasing a horizontal edge on the top edge means going one cell up (just outside the
        // border) and erasing the bottom edge.
        else {
            if y == grid.height() - 2 {
                grid[XY(x, y + 1)].bottom_edge = EdgeState::Off;
            } else {
                grid[XY(x, y)].bottom_edge = EdgeState::Off;
            }
        }
    }

    fn is_valid_start_or_end(
        grid: &CellGrid,
        XY(x, y): &XY,
        ) -> bool
    {
        // Start and end points need to be somewhere on the border, within the confines of the maze.
        *x == 0 || *x == grid.width() - 2 ||
        *y == 0 || *y == grid.height() - 2
    }

    fn count_exits(&self, point: &XY) -> usize {
        let mut exit_count = 0;

        let cell = &self.grid[point.clone()];
        if point.0 > 0 && !cell.has_left_edge() {
            exit_count += 1;
        }

        if point.1 > 0 && !cell.has_bottom_edge() {
            exit_count += 1;
        }

        if self.grid[XY(point.0 + 1, point.1)].has_left_edge() {
            exit_count += 1;
        }

        if self.grid[XY(point.0, point.1 + 1)].has_bottom_edge() {
            exit_count += 1;
        }

        exit_count
    }

    fn has_valid_edges(grid: &CellGrid) -> bool {
        for y in 0 .. grid.height() - 1 {
            for x in 0 .. grid.width() - 1 {
                if grid[XY(x+1, y)].has_left_edge() {
                    continue;
                }

                if grid[XY(x, y+1)].has_bottom_edge() {
                    continue;
                }

                let cell = &grid[XY(x+1, y+1)];
                if cell.has_bottom_edge() || cell.has_left_edge() {
                    continue;
                }

                return false;
            }
        }

        true
    }

    fn update_grid(&mut self) {
        self.grid = Self::create_grid(self.grid.width(), self.grid.height(), self.path.len());
        self.path = Self::extract_path(&self.grid);
    }

    fn extract_path(grid: &CellGrid) -> Vec<XY> {
        grid.iter().enumerate().filter_map(|(i, cell)| {
            match cell.kind {
                GridCellKind::Path(_) | GridCellKind::End => Some(XY(i % grid.width(), i / grid.width())),
                _ => None
            }
        }).collect()
    }

    fn process_command(&mut self) {
        match self.next_command {
            Some(Command::Exit) => std::process::exit(0),
            Some(Command::Refresh) => self.update_grid(),
            _ => (),
        };

        self.next_command = None;
    }

    fn handle_input(
        info: &CanvasInfo,
        state: &mut GridState,
        event: &Event<()>
        ) -> bool {
        let handled_mouse = MouseState::handle_input(info, &mut state.mouse_state, event);

        let handled_key = if state.next_command.is_none() {
            match event {
                Event::WindowEvent {
                    event: WindowEvent::KeyboardInput {
                        input: KeyboardInput {
                            state: ElementState::Released,
                            virtual_keycode: Some(vk),
                            ..
                        },
                        ..
                    },
                    ..
                } => {
                    state.next_command = match vk {
                        VirtualKeyCode::Escape => Some(Command::Exit),
                        VirtualKeyCode::F5 => Some(Command::Refresh),
                        _ => None
                    };

                    state.next_command.is_some()
                }
                _ => false,
            }
        } else {
            false
        };

        handled_mouse || handled_key
    }

    fn draw_vertical_edge(
        &self,
        image: &mut Image,
        x: usize,
        y1: usize,
        y2: usize
        )
    {
        draw_box(
            image,
            x * SCALE_IN_PX,
            y1 * SCALE_IN_PX,
            (x * SCALE_IN_PX) + EDGE_THICKNESS_IN_PX,
            y2 * SCALE_IN_PX,
            &Color { r: 0, g: 0, b: 255});
    }

    fn draw_horizontal_edge(
        &self,
        image: &mut Image,
        x1: usize,
        x2: usize,
        y: usize
        )
    {
        draw_box(
            image,
            x1 * SCALE_IN_PX,
            y * SCALE_IN_PX,
            x2 * SCALE_IN_PX,
            (y * SCALE_IN_PX) + EDGE_THICKNESS_IN_PX,
            &Color { r: 255, g: 0, b: 0});
    }

    fn draw_cell(
        &self,
        image: &mut Image,
        x: usize,
        y: usize,
        ) {
        let cell = &self.grid[XY(x, y)];

        let color = match cell.kind {
            GridCellKind::End => Color { r: 255, g: 50, b: 50 },
            GridCellKind::Path(n) => Color { r: 50, g: 50, b: 255 - (((n as f32 / self.path.len() as f32) * 255f32) as u8) },
            _ => Color { r: 255, g: 255, b: 255 },
        };

        draw_box(
            image,
            (x * SCALE_IN_PX) + CELL_FILL_MARGIN_IN_PX,
            (y * SCALE_IN_PX) + CELL_FILL_MARGIN_IN_PX,
            ((x+1) * SCALE_IN_PX) - CELL_FILL_MARGIN_IN_PX,
            ((y+1) * SCALE_IN_PX) - CELL_FILL_MARGIN_IN_PX,
            &color,
        );
    }

    fn draw(
        &self,
        image: &mut Image,
        )
    {
        image.fill(Color { r: 255, g: 255, b: 255 });

        let grid = &self.grid;
        for (y, row) in grid.chunks(grid.width()).enumerate() {
            for (x, cell) in row.iter().enumerate() {
                // Draw left edge
                if y < grid.height() - 1 {
                    if cell.left_edge == EdgeState::On {
                        self.draw_vertical_edge(image, x, y, y+1);
                    }
                }

                // Draw bottom edge
                if x < grid.width() - 1 {
                    if cell.bottom_edge == EdgeState::On {
                        self.draw_horizontal_edge(image, x, x+1, y);
                    }
                }

                self.draw_cell(image, x, y);
            }
        }
    }
}

fn main() {
    let mut grid_state = GridState::new(10, 10, 6);
    let grid = &mut grid_state.grid;

    // Configure the window that you want to draw in. You can add an event
    // handler to build interactive art. Input handlers for common use are
    // provided.
    let canvas = Canvas::new(grid.width() * SCALE_IN_PX, grid.height() * SCALE_IN_PX)
        .title("Mazes")
        .state(grid_state)
        .input(GridState::handle_input)
        ;

    // The canvas will render for you at up to 60fps.
    canvas.render(|grid_state, image| {
        grid_state.process_command();
        grid_state.draw(image);
    });
}
