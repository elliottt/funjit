#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Pos {
    pub x: isize,
    pub y: isize,
}

impl Pos {
    pub fn new(x: isize, y: isize) -> Self {
        Pos { x, y }
    }

    pub fn north() -> Self {
        Self::new(0, -1)
    }

    pub fn east() -> Self {
        Self::new(1, 0)
    }

    pub fn south() -> Self {
        Self::new(0, 1)
    }

    pub fn west() -> Self {
        Self::new(-1, 0)
    }
}

impl std::ops::AddAssign<&Pos> for Pos {
    fn add_assign(&mut self, other: &Self) {
        self.x += other.x;
        self.y += other.y;
        self.x = self.x.rem_euclid(Funge93::WIDTH as isize);
        self.y = self.y.rem_euclid(Funge93::HEIGHT as isize);
    }
}

#[test]
fn test_pos_move() {
    {
        let mut pos = Pos::new(0, 0);
        pos += &Pos::new(-1, 0);
        assert_eq!(Funge93::WIDTH as isize - 1, pos.x);
    }

    {
        let mut pos = Pos::new(0, 0);
        pos += &Pos::new(0, -1);
        assert_eq!(Funge93::HEIGHT as isize - 1, pos.y);
    }
}

pub struct Funge93 {
    rows: [[char; Self::WIDTH]; Self::HEIGHT],
}

impl Funge93 {
    const WIDTH: usize = 80;
    const HEIGHT: usize = 24;

    pub fn new() -> Self {
        Funge93 {
            rows: [[' '; Self::WIDTH]; Self::HEIGHT],
        }
    }

    pub fn from_string(prog: &str) -> Self {
        let mut space = Self::new();

        for (y, line) in prog.lines().enumerate().take(Self::HEIGHT) {
            for (x, c) in line.chars().enumerate().take(Self::WIDTH) {
                let cell = space.get_mut(Pos { x: x as isize, y: y as isize });
                *cell = c;
            }
        }

        space
    }

    pub fn get(&self, pos: Pos) -> char {
        self.rows[pos.y as usize][pos.x as usize]
    }

    pub fn get_mut(&mut self, pos: Pos) -> &mut char {
        &mut self.rows[pos.y as usize][pos.x as usize]
    }
}
