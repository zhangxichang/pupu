use slint::winit_030::winit::window::ResizeDirection;

use crate::WindowResizeDirection;

impl From<WindowResizeDirection> for ResizeDirection {
    fn from(value: WindowResizeDirection) -> Self {
        match value {
            WindowResizeDirection::NorthWest => Self::NorthWest,
            WindowResizeDirection::North => Self::North,
            WindowResizeDirection::NorthEast => Self::NorthEast,
            WindowResizeDirection::East => Self::East,
            WindowResizeDirection::SouthEast => Self::SouthEast,
            WindowResizeDirection::South => Self::South,
            WindowResizeDirection::SouthWest => Self::SouthWest,
            WindowResizeDirection::West => Self::West,
        }
    }
}
