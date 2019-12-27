extern crate pixel_canvas;
extern crate rand;

use std::time::{Duration, Instant};
use std::env;

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
use crate::rand::SeedableRng;

mod grid;
use grid::{Grid, XY};

const GRID_WIDTH: usize = 20;
const GRID_HEIGHT: usize = 20;
const PATH_POINT_COUNT: usize = 12;
const SCALE_IN_PX: usize = 25;
const CELL_FILL_MARGIN_IN_PX: usize  = 5;
const EDGE_THICKNESS_IN_PX: usize = 3;
const EDGE_ENABLED_CHANCE: f64 = 1.0;
const DRAW_OFFSET_IN_PX: usize = 20;

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
            image[pixel_canvas::XY(draw_x + DRAW_OFFSET_IN_PX, draw_y + DRAW_OFFSET_IN_PX)] = *color;
        }
    }
}

#[derive(Clone, Debug)]
enum GridCellKind {
    Empty,
    Path(usize),
    PathIntermediate,
    End,
}

#[derive(Clone, Debug)]
enum EdgeState {
    Unset,
    Off,
    ProvisionallyOn,
    On,
}

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
enum GenStage {
    Borders,
    Path,
    EnableEdgesRandomly,
    EraseInvalidEdges(usize),
    Rest,
    Done,
    TimedTransition(Duration, Box<GenStage>),
}

struct GenState {
    stage: GenStage,
    entry_time: Instant,
}

type CellGrid = Grid<GridCell>;
struct GridState {
    rng: rand::rngs::StdRng,
    mouse_state: MouseState,
    grid: CellGrid,
    path: Vec<XY>,
    next_command: Option<Command>,
    state: GenState,
    should_draw_path: bool,
    path_point_count: usize,
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

#[derive(Clone)]
struct CellEdge {
    point: XY,
    is_left_edge: bool,
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

impl GenState {
    fn new(stage: GenStage) -> GenState {
        GenState {
            stage,
            entry_time: Instant::now(),
        }
    }
}

impl GridState {
    fn new(seed: u64, width: usize, height: usize, path_point_count: usize) -> GridState {
        let mut gs = GridState {
            rng: rand::rngs::StdRng::seed_from_u64(seed),
            grid: CellGrid::new(width, height, &GridCell::new()),
            path: Vec::new(),
            mouse_state: MouseState::new(),
            next_command: None,
            state: GenState::new(GenStage::Borders),
            should_draw_path: true,
            path_point_count: path_point_count,
        };

        gs.start_generate_maze();

        gs
    }

    fn set_stage(&mut self, stage: GenStage) {
        println!("Setting stage to {:?}", stage);
        self.state = GenState::new(stage);
    }

    fn set_stage_delayed(&mut self, stage: GenStage, millis: u64) {
        self.state = GenState::new(GenStage::TimedTransition(Duration::from_millis(millis), Box::new(stage)));
    }

    fn start_generate_maze(&mut self) {
        self.grid = CellGrid::new(self.grid.width(), self.grid.height(), &GridCell::new());
        self.set_stage(GenStage::Borders);
        self.path.clear();
    }

    fn update(&mut self) {
        match self.state.stage {
            GenStage::Borders => self.fill_borders(),
            GenStage::Path => self.update_path(),
            GenStage::EnableEdgesRandomly => self.enable_edges_randomly(),
            GenStage::EraseInvalidEdges(starting_index) => self.erase_invalid_edges(starting_index),
            GenStage::Rest => self.fill_rest_of_maze(),
            GenStage::TimedTransition(ref duration, ref next_stage) => {
                if self.state.entry_time.elapsed() >= *duration {
                    let next = (**next_stage).clone();
                    self.set_stage(next);
                }
            },
            _ => {},
        };
    }

    fn fill_borders(&mut self) {
        let width = self.grid.width();
        let height = self.grid.height();

        // Turn on walls at the borders.
        for (y, row) in self.grid.chunks_mut(width).enumerate() {
            for (x, cell) in row.iter_mut().enumerate() {
                cell.bottom_edge = if (x != width - 1) && (y == 0 || y == height - 1) { EdgeState::On } else { EdgeState::Unset };
                cell.left_edge = if (y != height - 1) && (x == 0 || x == width - 1) { EdgeState::On } else { EdgeState::Unset };
            }
        }

        self.set_stage_delayed(GenStage::Path, 1000);
    }

    fn update_path(&mut self) {
        let width = self.grid.width();
        let height = self.grid.height();

        let mut path = Vec::<PathPoint>::new();

        // For now choose the start point in a corner
        let start = {
            let start_bottom = self.rng.gen_bool(0.5);
            let start_left = self.rng.gen_bool(0.5);
            let start_points_vertical = self.rng.gen_bool(0.5);

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
        while path.len() < self.path_point_count {
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
                            Direction::Up => XY(last.point.0, self.rng.gen_range(last.point.1, height - 1)),
                            Direction::Down => XY(last.point.0, self.rng.gen_range(0, last.point.1)),
                            Direction::Left => XY(self.rng.gen_range(0, last.point.0), last.point.1),
                            Direction::Right => XY(self.rng.gen_range(last.point.0, width - 1), last.point.1),
                        };

                    println!("considering point ({}, {}, {:?})", point.0, point.1, dir);

                    let is_valid =
                        path.iter().find(|&item| { item.point == point }).is_none() &&
                        ((path.len() != self.path_point_count - 1) || Self::is_valid_start_or_end(&self.grid, &point));

                    if is_valid {
                        path.push(PathPoint { point, dir });
                        println!("added. len now {}", path.len());
                        break;
                    }
                    else if path.len() == self.path_point_count - 1 {
                        should_reset_path = true;
                    }
                }

                if should_reset_path {
                    println!("resetting iteration {}, path length {} out of {}. start point ({}, {}, {:?}). last point ({}, {}, {:?})", iter, path.len(), self.path_point_count, start.point.0, start.point.1, start.dir, last.point.0, last.point.1, last.dir);
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
        for (i, point) in path.iter().enumerate() {
            match i {
                0 => {
                    self.grid[&point.point].kind = GridCellKind::Path(i);
                    Self::erase_start_or_end_edge(&mut self.rng, &mut self.grid, &point.point);
                },
                _ if i == len - 1 => {
                    self.grid[&point.point].kind = GridCellKind::End;
                    Self::erase_start_or_end_edge(&mut self.rng, &mut self.grid, &point.point);
                },
                _ => {
                    self.grid[&point.point].kind = GridCellKind::Path(i);
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
                            let cell = &mut self.grid[XY(x, y+1)];
                            cell.bottom_edge = EdgeState::Off;
                            if let GridCellKind::Empty = cell.kind {
                                cell.kind = GridCellKind::PathIntermediate;
                            }
                            y += 1;
                        },
                        Direction::Down => {
                            let cell = &mut self.grid[XY(x, y)];
                            cell.bottom_edge = EdgeState::Off;
                            if let GridCellKind::Empty = cell.kind {
                                cell.kind = GridCellKind::PathIntermediate;
                            }
                            y -= 1;
                        },
                        Direction::Left => {
                            let cell = &mut self.grid[XY(x, y)];
                            cell.left_edge = EdgeState::Off;
                            if let GridCellKind::Empty = cell.kind {
                                cell.kind = GridCellKind::PathIntermediate;
                            }
                            x -= 1;
                        },
                        Direction::Right => {
                            let cell = &mut self.grid[XY(x+1, y)];
                            cell.left_edge = EdgeState::Off;
                            if let GridCellKind::Empty = cell.kind {
                                cell.kind = GridCellKind::PathIntermediate;
                            }
                            x += 1;
                        },
                    }
                }
            }

            Some(step)
        });

        self.path = Self::extract_path(&self.grid);
        println!("path: {:?}", self.path);

        self.set_stage_delayed(GenStage::EnableEdgesRandomly, 1000);
    }

    fn enable_edges_randomly(&mut self) {
        let width = self.grid.width();
        let height = self.grid.height();

        // Reset all provisionally-on edges to unset to try again.
        for cell in self.grid.iter_mut() {
            if let EdgeState::ProvisionallyOn = cell.bottom_edge {
                cell.bottom_edge = EdgeState::Unset;
            }

            if let EdgeState::ProvisionallyOn = cell.left_edge {
                cell.left_edge = EdgeState::Unset;
            }
        }

        // Turn on edges randomly
        for cell in self.grid.iter_mut() {
            if let EdgeState::Unset = cell.bottom_edge {
                if self.rng.gen_bool(EDGE_ENABLED_CHANCE) {
                    cell.bottom_edge = EdgeState::ProvisionallyOn;
                }
            }

            if let EdgeState::Unset = cell.left_edge {
                if self.rng.gen_bool(EDGE_ENABLED_CHANCE) {
                    cell.left_edge = EdgeState::ProvisionallyOn;
                }
            }
        }

        if Self::has_inner_grid_walls(&self.grid) {
            self.set_stage_delayed(GenStage::EraseInvalidEdges(0), 250);
        }
    }

    fn erase_edge_in_enclosure(&mut self, point: &XY) -> bool {
        if let Some(enclosed_cells) = Self::find_enclosed_section(&self.grid, &point) {
            let mut tries = 4;
            while let Some(edge_to_erase) = Self::pick_random_non_border_edge(&mut self.rng, &self.grid, &point) {
                if tries == 0 {
                    return true;
                }

                let cell_orig = self.grid[edge_to_erase.point.clone()].clone();

                Self::erase_cell_edge(&mut self.grid, &edge_to_erase);

                if !Self::has_inner_grid_walls(&self.grid) {
                    tries -= 1;
                    self.grid[edge_to_erase.point.clone()] = cell_orig;
                }
                else {
                    return true;
                }
            }
        }

        // Nothing more to do with this point: no enclosure found.
        false
    }

    fn erase_invalid_edges(&mut self, starting_index: usize) {
        for i in starting_index .. self.grid.len() {
            let point = self.grid.index_to_xy(i);
            if point.0 < self.grid.width() - 1 && point.1 < self.grid.height() - 1 {
                if !self.erase_edge_in_enclosure(&point) {
                    self.set_stage(GenStage::EraseInvalidEdges(i + 1));
                }

                return;
            }
        }

        self.set_stage_delayed(GenStage::Rest, 500);
    }

    fn fill_rest_of_maze(&mut self) {
        let width = self.grid.width();
        let height = self.grid.height();

        // Make all the provisional edges real
        for cell in self.grid.iter_mut() {
            if let EdgeState::ProvisionallyOn = cell.bottom_edge {
                cell.bottom_edge = EdgeState::On;
            }

            if let EdgeState::ProvisionallyOn = cell.left_edge {
                cell.left_edge = EdgeState::On;
            }
        }

        self.set_stage(GenStage::Done);
    }

    fn erase_start_or_end_edge(rng: &mut rand::rngs::StdRng, grid: &mut CellGrid, XY(x, y): &XY) {
        let x = *x;
        let y = *y;

        let has_vertical_edge = x == 0 || x == grid.width() - 2;
        let has_horizontal_edge = y == 0 || y == grid.height() - 2;

        // Decide whether to erase a vertical edge or horizontal edge. The chosen edge can only be
        // erased if the cell has such an edge available to erase, so keep looping until the intent
        // and available edge lines up.
        let mut erase_vertical_edge = rng.gen_bool(0.5);
        while erase_vertical_edge != has_vertical_edge && !erase_vertical_edge != has_horizontal_edge {
            erase_vertical_edge = rng.gen_bool(0.5);
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

    fn erase_cell_edge(grid: &mut CellGrid, edge_to_erase: &CellEdge) {
        if edge_to_erase.is_left_edge {
            grid[&edge_to_erase.point].left_edge = EdgeState::Unset;
        }
        else {
            grid[&edge_to_erase.point].bottom_edge = EdgeState::Unset;
        }
    }

    fn pick_random_non_border_edge(rng: &mut rand::rngs::StdRng, grid: &CellGrid, point: &XY) -> Option<CellEdge> {
        let mut edges = [None, None, None, None];
        let mut edge_count = 0;

        let cell = &grid[point];
        let adjacent_right_point = XY(point.0 + 1, point.1);
        let adjacent_above_point = XY(point.0, point.1 + 1);

        if point.0 > 0 && cell.has_left_edge() {
            edges[edge_count] = Some(CellEdge { point: point.clone(), is_left_edge: true });
            edge_count += 1;
        }

        if point.1 > 0 && cell.has_bottom_edge() {
            edges[edge_count] = Some(CellEdge { point: point.clone(), is_left_edge: false });
            edge_count += 1;
        }

        if point.0 < grid.width() - 2 && grid[&adjacent_right_point].has_left_edge() {
            edges[edge_count] = Some(CellEdge { point: adjacent_right_point.clone(), is_left_edge: true });
            edge_count += 1;
        }

        if point.1 < grid.height() - 2 && grid[&adjacent_above_point].has_bottom_edge() {
            edges[edge_count] = Some(CellEdge { point: adjacent_above_point.clone(), is_left_edge: false });
            edge_count += 1;
        }

        if edge_count > 0 {
            Some(edges[rng.gen_range(0, edge_count)].as_ref().unwrap().clone())
        }
        else {
            None
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

    fn count_exits(grid: &CellGrid, point: &XY) -> usize {
        let mut exit_count = 0;

        let cell = &grid[point];
        if !cell.has_left_edge() {
            exit_count += 1;
        }

        if !cell.has_bottom_edge() {
            exit_count += 1;
        }

        if !grid[XY(point.0 + 1, point.1)].has_left_edge() {
            exit_count += 1;
        }

        if !grid[XY(point.0, point.1 + 1)].has_bottom_edge() {
            exit_count += 1;
        }

        exit_count
    }

    fn has_inner_grid_walls(grid: &CellGrid) -> bool {
        // For every 2x2 sub-grid, there must be at least one inner wall
        for y in 0 .. grid.height() - 1 {
            for x in 0 .. grid.width() - 1 {
                if grid[XY(x+1, y)].has_left_edge() {
                    continue;
                }

                if grid[XY(x, y+1)].has_bottom_edge() {
                    continue;
                }

                let cell = &grid[XY(x+1, y+1)];
                if cell.has_left_edge() || cell.has_bottom_edge() {
                    continue;
                }

                //println!("no inner edges");
                return false;
            }
        }

        true
    }

    fn has_valid_edges(grid: &CellGrid) -> bool {
        fn are_all_cells_open(grid: &CellGrid) -> bool {
            // Every cell must have at least one exit
            for y in 0 .. grid.height() - 1 {
                for x in 0 .. grid.width() - 1 {
                    if GridState::count_exits(grid, &XY(x, y)) == 0 {
                        //println!("zero-exit cell");
                        return false;
                    }
                }
            }

            true
        }

        fn are_all_cells_reachable(grid: &CellGrid) -> bool {
            // Every cell must be reachable
            let traversal = GridState::visit_all(&grid, &XY(0, 0));
            traversal.iter().find(|&item| { !item }).is_none()
        }

        true
        && Self::has_inner_grid_walls(grid)
        //&& are_all_cells_open(grid)
        //&& are_all_cells_reachable(grid)
    }

    fn visit_all(grid: &CellGrid, start: &XY) -> Grid<bool> {
        fn visit(grid: &CellGrid, traversal: &mut Grid<bool>, x: usize, y: usize) {
            let traversal_cell = &mut traversal[XY(x, y)];
            if *traversal_cell {
                // already visited
                return;
            }

            *traversal_cell = true;

            if x > 0 && !grid[XY(x, y)].has_left_edge() {
                visit(grid, traversal, x - 1, y);
            }

            if y > 0 && !grid[XY(x, y)].has_bottom_edge() {
                visit(grid, traversal, x, y - 1);
            }

            if x < traversal.width() - 1 && !grid[XY(x + 1, y)].has_left_edge() {
                visit(grid, traversal, x + 1, y);
            }

            if y < traversal.height() - 1 && !grid[XY(x, y + 1)].has_bottom_edge() {
                visit(grid, traversal, x, y + 1);
            }
        }

        let mut traversal = Grid::new(grid.width() - 1, grid.height() - 1, &false);
        visit(grid, &mut traversal, start.0, start.1);

        traversal
    }

    fn find_enclosed_section(grid: &CellGrid, point: &XY) -> Option<Vec<XY>> {
        let traversal = Self::visit_all(grid, point);

        // Filter to only cells that were touched by the traversal
        traversal.iter().enumerate().filter(|(i, _)| {
            traversal[traversal.index_to_xy(*i)]
        }).fold(Some(Vec::<XY>::new()), |section_opt, (i, _)| {
            if let Some(mut section) = section_opt {
                let p = traversal.index_to_xy(i);
                match grid[&p].kind {
                    // Throw away the enclosed section if it ever touched the path, because that
                    // means it had a way out of the maze.
                    GridCellKind::Path(_) | GridCellKind::PathIntermediate | GridCellKind::End => None,

                    // Otherwise, it was non-path cell that might have been self-enclosed, so add it
                    // to the path.
                    _ => {
                        section.push(p);
                        Some(section)
                    },
                }
            }
            else {
                section_opt
            }
        })
    }

    fn extract_path(grid: &CellGrid) -> Vec<XY> {
        grid.iter().enumerate().filter_map(|(i, cell)| {
            match cell.kind {
                GridCellKind::Path(_) | GridCellKind::End => Some(grid.index_to_xy(i)),
                _ => None
            }
        }).collect()
    }

    fn process_command(&mut self) {
        match self.next_command {
            Some(Command::Exit) => std::process::exit(0),
            Some(Command::Refresh) => self.start_generate_maze(),
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
                        VirtualKeyCode::P => {
                            state.should_draw_path = !state.should_draw_path;
                            None
                        },
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
        y2: usize,
        color: &Color,
        )
    {
        draw_box(
            image,
            x * SCALE_IN_PX,
            y1 * SCALE_IN_PX,
            (x * SCALE_IN_PX) + EDGE_THICKNESS_IN_PX,
            (y2 * SCALE_IN_PX) + EDGE_THICKNESS_IN_PX,
            color);
    }

    fn draw_horizontal_edge(
        &self,
        image: &mut Image,
        x1: usize,
        x2: usize,
        y: usize,
        color: &Color,
        )
    {
        draw_box(
            image,
            x1 * SCALE_IN_PX,
            y * SCALE_IN_PX,
            (x2 * SCALE_IN_PX) + EDGE_THICKNESS_IN_PX,
            (y * SCALE_IN_PX) + EDGE_THICKNESS_IN_PX,
            color);
    }

    fn draw_cell(
        &self,
        image: &mut Image,
        x: usize,
        y: usize,
        ) {
        let cell = &self.grid[XY(x, y)];

        let color =
            if self.should_draw_path {
                match cell.kind {
                    GridCellKind::End => Color { r: 255, g: 50, b: 50 },
                    GridCellKind::Path(n) => Color { r: 50, g: 50, b: 255 - (((n as f32 / self.path.len() as f32) * 255f32) as u8) },
                    GridCellKind::PathIntermediate => Color { r: 100, g: 100, b: 100 },
                    _ => Color { r: 255, g: 255, b: 255 },
                }
            }
            else {
                Color { r: 255, g: 255, b: 255 }
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
        for i in 0 .. grid.len() {
            let XY(x, y) = grid.index_to_xy(i);
            let cell = &grid[XY(x, y)];

            // Draw left edge
            if y < grid.height() - 1 {
                match cell.left_edge {
                    EdgeState::On => self.draw_vertical_edge(image, x, y, y+1, &Color { r: 0, g: 0, b: 0 }),
                    EdgeState::ProvisionallyOn => self.draw_vertical_edge(image, x, y, y+1, &Color { r: 200, g: 200, b: 200 }),
                    _ => ()
                };
            }

            // Draw bottom edge
            if x < grid.width() - 1 {
                match cell.bottom_edge {
                    EdgeState::On => self.draw_horizontal_edge(image, x, x+1, y, &Color { r: 0, g: 0, b: 0 }),
                    EdgeState::ProvisionallyOn => self.draw_horizontal_edge(image, x, x+1, y, &Color { r: 200, g: 200, b: 200 }),
                    _ => ()
                };
            }

            self.draw_cell(image, x, y);
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut seed = rand::thread_rng().gen_range(0, u64::max_value());

    for i in 0 .. args.len() {
        if args[i] == "-r" {
            if i < args.len() - 1 {
                seed = args[i + 1].parse::<u64>().unwrap();
            }
        }
    }

    println!("Using seed {}", seed);

    let mut grid_state = GridState::new(seed, GRID_WIDTH, GRID_HEIGHT, PATH_POINT_COUNT);
    let grid = &mut grid_state.grid;

    let canvas = Canvas::new(600, 600)
        .title("Mazes")
        .state(grid_state)
        .input(GridState::handle_input)
        ;

    canvas.render(|grid_state, image| {
        grid_state.process_command();
        grid_state.update();
        grid_state.draw(image);
    });
}
