#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    Right,
    Left,
    Up,
    Down,
    A,
    B,
    Select,
    Start,
}

impl Button {
    pub fn from_index(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Right),
            1 => Some(Self::Left),
            2 => Some(Self::Up),
            3 => Some(Self::Down),
            4 => Some(Self::A),
            5 => Some(Self::B),
            6 => Some(Self::Select),
            7 => Some(Self::Start),
            _ => None,
        }
    }
}
