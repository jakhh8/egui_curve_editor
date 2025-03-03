// From Godot

#[derive(PartialEq, Clone, Copy, Default, serde::Deserialize, serde::Serialize)]
pub enum TangentMode {
    Free,
    #[default]
    Linear,
}

#[derive(Clone, Copy, Default, serde::Deserialize, serde::Serialize)]
pub struct Point {
    pos: egui::Pos2,
    left_tan: f32,
    right_tan: f32,
    left_mode: TangentMode,
    right_mode: TangentMode,
}

impl Point {
    pub fn from_pos(pos: egui::Pos2) -> Self {
        Self {
            pos,
            ..Default::default()
        }
    }
}

#[derive(Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct Curve {
    points: Vec<Point>,
}

#[allow(unused)]
impl Curve {
    pub fn linear() -> Self {
        Self {
            points: vec![
                Point::from_pos(egui::pos2(0.0, 0.0)),
                Point::from_pos(egui::pos2(1.0, 1.0)),
            ],
        }
    }

    pub fn add_point(&mut self, mut point: Point) -> usize {
        point.pos = point.pos.clamp(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));

        let index = if self.points.len() == 0 {
            self.points.push(point);

            0
        } else if self.points.len() == 1 {
            let diff = point.pos.x - self.points[0].pos.x;

            if diff > 0.0 {
                self.points.push(point);

                1
            } else {
                self.points.insert(0, point);

                0
            }
        } else {
            let i = self.get_index(point.pos.x);

            if i == 0 && point.pos.x < self.points[0].pos.x {
                self.points.insert(0, point);

                0
            } else {
                self.points.insert(i + 1, point);

                i + 1
            }
        };

        self.update_auto_tangents(index);

        index
    }

    pub fn remove_point(&mut self, index: usize) {
        if index > self.points.len() - 1 {
            return;
        }

        self.points.remove(index);
    }

    pub fn clear_points(&mut self) {
        self.points.clear();
    }

    pub fn get_index(&self, offset: f32) -> usize {
        let mut min = 0;
        let mut max = self.points.len() - 1;

        while max - min > 1 {
            let m = (min + max) / 2;

            let a = self.points[m].pos.x;
            let b = self.points[m + 1].pos.x;

            if a < offset && b < offset {
                min = m;
            } else if a > offset {
                max = m;
            } else {
                return m;
            }
        }

        if offset > self.points[max].pos.x {
            return max;
        }

        min
    }

    pub fn sample(&self, offset: f32) -> f32 {
        if self.points.len() == 0 {
            return 0.0;
        }

        if self.points.len() == 1 {
            return self.points[0].pos.y;
        }

        let i = self.get_index(offset);

        if i == self.points.len() - 1 {
            return self.points[i].pos.y;
        }

        let local = offset - self.points[i].pos.x;

        if i == 0 && local <= 0.0 {
            return self.points[0].pos.y;
        }

        self.sample_local_nocheck(i, local)
    }

    pub fn point_positions(&self) -> Vec<egui::Pos2> {
        self.points.iter().map(|point| point.pos).collect()
    }

    pub fn get_position(&self, index: usize) -> Option<egui::Pos2> {
        if index >= self.points.len() {
            return None;
        }

        Some(self.points[index].pos)
    }

    pub fn set_position(&mut self, index: usize, mut pos: egui::Pos2) {
        pos = pos.clamp(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));

        if index >= self.points.len() {
            return;
        }

        if index > 0 && self.points[index - 1].pos.x > pos.x {
            return;
        }

        if index < self.points.len() - 1 && self.points[index + 1].pos.x < pos.x {
            return;
        }

        self.points[index].pos = pos;
        //TODO:
        self.update_auto_tangents(index);
    }

    pub fn get_left_tan(&self, index: usize) -> Option<f32> {
        if index >= self.points.len() {
            return None;
        }

        Some(self.points[index].left_tan)
    }

    pub fn set_left_tan(&mut self, index: usize, tangent: f32) {
        if index >= self.points.len() || tangent.is_nan() || tangent.is_infinite() {
            return;
        }

        self.points[index].left_tan = tangent;
        self.points[index].left_mode = TangentMode::Free;
    }

    pub fn get_right_tan(&self, index: usize) -> Option<f32> {
        if index >= self.points.len() {
            return None;
        }

        Some(self.points[index].right_tan)
    }

    pub fn set_right_tan(&mut self, index: usize, tangent: f32) {
        if index >= self.points.len() || tangent.is_nan() || tangent.is_infinite() {
            return;
        }

        self.points[index].right_tan = tangent;
        self.points[index].right_mode = TangentMode::Free;
    }

    pub fn index_is_first_or_last(&self, index: usize) -> bool {
        index == 0 || index == self.points.len() - 1
    }

    pub fn index_is_first(&self, index: usize) -> bool {
        index == 0
    }

    pub fn index_is_last(&self, index: usize) -> bool {
        index == self.points.len() - 1
    }

    fn sample_local_nocheck(&self, index: usize, mut local_offset: f32) -> f32 {
        let a = self.points[index];
        let b = self.points[index + 1];

        // Cubic bézier

        // Control points at equal distances
        let mut d = b.pos.x - a.pos.x;
        const EPSILON: f32 = 0.00001;
        if d.abs() < EPSILON {
            return b.pos.y;
        }
        local_offset /= d;
        d /= 3.0;
        let yac = a.pos.y + d * a.right_tan;
        let ybc = b.pos.y - d * b.left_tan;

        let y = bezier_interpolate(a.pos.y, yac, ybc, b.pos.y, local_offset);

        y.clamp(0.0, 1.0)
    }

    fn update_auto_tangents(&mut self, index: usize) {
        let mut p = self.points[index];

        if index > 0 {
            if p.left_mode == TangentMode::Linear {
                let v = (self.points[index - 1].pos - p.pos).normalized();
                p.left_tan = v.y / v.x;
            }
            if self.points[index - 1].right_mode == TangentMode::Linear {
                let v = (self.points[index - 1].pos - p.pos).normalized();
                self.points[index - 1].right_tan = v.y / v.x;
            }
        }

        if index + 1 < self.points.len() {
            if p.right_mode == TangentMode::Linear {
                let v = (self.points[index + 1].pos - p.pos).normalized();
                p.right_tan = v.y / v.x;
            }
            if self.points[index + 1].left_mode == TangentMode::Linear {
                let v = (self.points[index + 1].pos - p.pos).normalized();
                self.points[index + 1].left_tan = v.y / v.x;
            }
        }

        self.points[index] = p;
    }
}

fn bezier_interpolate(start: f32, control_1: f32, control_2: f32, end: f32, t: f32) -> f32 {
    // From Wikipedia
    let omt = 1.0 - t;
    let omt2 = omt * omt;
    let omt3 = omt2 * omt;
    let t2 = t * t;
    let t3 = t2 * t;

    start * omt3 + control_1 * omt2 * t * 3.0 + control_2 * omt * t2 * 3.0 + end * t3
}
