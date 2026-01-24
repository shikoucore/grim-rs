use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Box {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

impl Box {
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn x(&self) -> i32 {
        self.x
    }

    pub fn y(&self) -> i32 {
        self.y
    }

    pub fn width(&self) -> i32 {
        self.width
    }

    pub fn height(&self) -> i32 {
        self.height
    }

    pub fn is_empty(&self) -> bool {
        self.width <= 0 || self.height <= 0
    }

    pub fn intersects(&self, other: &Box) -> bool {
        if self.is_empty() || other.is_empty() {
            return false;
        }

        let x1 = self.x.max(other.x);
        let y1 = self.y.max(other.y);
        let x2 = (self.x + self.width).min(other.x + other.width);
        let y2 = (self.y + self.height).min(other.y + other.height);

        x2 > x1 && y2 > y1
    }

    pub fn intersection(&self, other: &Box) -> Option<Box> {
        if !self.intersects(other) {
            return None;
        }

        let x1 = self.x.max(other.x);
        let y1 = self.y.max(other.y);
        let x2 = (self.x + self.width).min(other.x + other.width);
        let y2 = (self.y + self.height).min(other.y + other.height);

        Some(Box::new(x1, y1, x2 - x1, y2 - y1))
    }
}

impl fmt::Display for Box {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{},{} {}x{}", self.x, self.y, self.width, self.height)
    }
}

impl std::str::FromStr for Box {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(' ').collect();
        if parts.len() != 2 {
            return Err(crate::Error::InvalidGeometry(s.to_string()));
        }

        let xy: Vec<&str> = parts[0].split(',').collect();
        let wh: Vec<&str> = parts[1].split('x').collect();

        if xy.len() != 2 || wh.len() != 2 {
            return Err(crate::Error::InvalidGeometry(s.to_string()));
        }

        let x = xy[0]
            .parse()
            .map_err(|_| crate::Error::InvalidGeometry(s.to_string()))?;
        let y = xy[1]
            .parse()
            .map_err(|_| crate::Error::InvalidGeometry(s.to_string()))?;
        let width = wh[0]
            .parse()
            .map_err(|_| crate::Error::InvalidGeometry(s.to_string()))?;
        let height = wh[1]
            .parse()
            .map_err(|_| crate::Error::InvalidGeometry(s.to_string()))?;

        Ok(Box::new(x, y, width, height))
    }
}
