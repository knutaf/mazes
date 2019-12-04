use std::{
    ops::{Deref, DerefMut, Index, IndexMut},
};

/// A simple grid of user-defined objects.
///
/// It dereferences to a slice of [`CellType`], so you can directly manipulate
/// it via regular (mutable) slice methods. In addition, you can index
/// into it by `(row, column)` pairs.
pub struct Grid<CellType>
    where CellType : Clone {
    width: usize,
    height: usize,
    cells: Vec<CellType>,
}

/// A row/column pair for indexing into the grid.
/// Distinct from an x/y pair.
pub struct RC(pub usize, pub usize);

/// An x/y pair for indexing into the grid.
/// Distinct from a row/column pair.
#[derive(PartialEq, Clone)]
pub struct XY(pub usize, pub usize);

impl<CellType> Grid<CellType>
    where CellType : Clone {
    /// The width of the grid in cells.
    pub fn width(&self) -> usize {
        self.width
    }

    /// The height of the grid in cells.
    pub fn height(&self) -> usize {
        self.height
    }

    /// Create a blank grid with the given dimensions.
    pub fn new(width: usize, height: usize, template: &CellType) -> Grid<CellType> {
        Grid {
            width,
            height,
            cells: vec![template.clone(); (width * height) as usize],
        }
    }
}

impl<CellType> Index<RC> for Grid<CellType>
    where CellType : Clone {
    type Output = CellType;
    fn index(&self, RC(row, col): RC) -> &Self::Output {
        &self.cells[(row * self.width + col) as usize]
    }
}

impl<CellType> IndexMut<RC> for Grid<CellType>
    where CellType : Clone {
    fn index_mut(&mut self, RC(row, col): RC) -> &mut Self::Output {
        &mut self.cells[(row * self.width + col) as usize]
    }
}

impl<CellType> Index<XY> for Grid<CellType>
    where CellType : Clone {
    type Output = CellType;
    fn index(&self, XY(x, y): XY) -> &Self::Output {
        &self.cells[(y * self.width + x) as usize]
    }
}

impl<CellType> IndexMut<XY> for Grid<CellType>
    where CellType : Clone {
    fn index_mut(&mut self, XY(x, y): XY) -> &mut Self::Output {
        &mut self.cells[(y * self.width + x) as usize]
    }
}

impl<CellType> Deref for Grid<CellType>
    where CellType : Clone {
    type Target = [CellType];
    fn deref(&self) -> &Self::Target {
        &self.cells
    }
}

impl<CellType> DerefMut for Grid<CellType>
    where CellType : Clone {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.cells
    }
}
