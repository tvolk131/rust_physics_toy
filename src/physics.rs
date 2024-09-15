use iced::{
    mouse::{self, Interaction},
    widget::canvas::{Frame, Geometry, Path, Program},
    Color, Point, Rectangle, Renderer, Size, Theme,
};

use std::collections::HashMap;

const ELASTICITY_COEFFICIENT: f32 = 0.7;
const AIR_RESISTANCE_COEFFICIENT: f32 = 0.998;
const SIZE_COEFFICIENT_PER_TICK: f32 = 0.998;
const MIN_RADIUS_SIZE: f32 = 0.5;
const GRAVITY: f32 = 0.2;
const CELL_SIZE: f32 = 50.0;

use crate::Message;

pub struct Grid {
    width: f32,
    height: f32,
    circles: Vec<Circle>,
}

impl Grid {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            width,
            height,
            circles: Vec::new(),
        }
    }

    pub fn add_circle(&mut self, circle: Circle) {
        self.circles.push(circle);
    }

    pub fn get_circle_count(&self) -> usize {
        self.circles.len()
    }

    pub fn tick(&mut self, sub_ticks: u32) {
        // Apply subtick-independent forces first.
        for circle in &mut self.circles {
            // Apply air resistance to all circles.
            circle.velocity.0 *= AIR_RESISTANCE_COEFFICIENT;
            circle.velocity.1 *= AIR_RESISTANCE_COEFFICIENT;

            // Change circle sizes.
            circle.radius *= SIZE_COEFFICIENT_PER_TICK;
        }

        self.circles
            .retain(|circle| circle.radius >= MIN_RADIUS_SIZE);

        for _ in 0..sub_ticks {
            // Apply gravity to all circles.
            for cell in &mut self.circles {
                cell.velocity.1 += GRAVITY / sub_ticks as f32;
            }

            // Move circles based on current velocity.
            for cell in &mut self.circles {
                cell.x_pos += cell.velocity.0 / sub_ticks as f32;
                cell.y_pos += cell.velocity.1 / sub_ticks as f32;
            }

            // Bounce circles off the walls, applying friction.
            for cell in &mut self.circles {
                if cell.x_pos - cell.radius < 0.0 {
                    cell.x_pos = cell.radius;
                    cell.velocity.0 = -cell.velocity.0 * ELASTICITY_COEFFICIENT;
                }

                if cell.x_pos + cell.radius > self.width {
                    cell.x_pos = self.width - cell.radius;
                    cell.velocity.0 = -cell.velocity.0 * ELASTICITY_COEFFICIENT;
                }

                if cell.y_pos - cell.radius < 0.0 {
                    cell.y_pos = cell.radius;
                    cell.velocity.1 = -cell.velocity.1 * ELASTICITY_COEFFICIENT;
                }

                if cell.y_pos + cell.radius > self.height {
                    cell.y_pos = self.height - cell.radius;
                    cell.velocity.1 = -cell.velocity.1 * ELASTICITY_COEFFICIENT;
                }
            }

            // Build the spatial grid for collision detection.
            let mut grid: HashMap<(i32, i32), Vec<usize>> = HashMap::new();

            for (i, circle) in self.circles.iter().enumerate() {
                let min_cell_x = ((circle.x_pos - circle.radius) / CELL_SIZE).floor() as i32;
                let max_cell_x = ((circle.x_pos + circle.radius) / CELL_SIZE).floor() as i32;
                let min_cell_y = ((circle.y_pos - circle.radius) / CELL_SIZE).floor() as i32;
                let max_cell_y = ((circle.y_pos + circle.radius) / CELL_SIZE).floor() as i32;

                for cell_x in min_cell_x..=max_cell_x {
                    for cell_y in min_cell_y..=max_cell_y {
                        grid.entry((cell_x, cell_y)).or_default().push(i);
                    }
                }
            }

            // Bounce circles off each other within the grid cells.
            for circle_indices in grid.values() {
                let len = circle_indices.len();
                for idx1 in 0..len {
                    let i = circle_indices[idx1];
                    for idx2 in (idx1 + 1)..len {
                        let j = circle_indices[idx2];

                        let (circle_a, circle_b) = self.get_two_mut(i, j);
                        Self::avoid_collision(circle_a, circle_b);
                    }
                }
            }
        }
    }

    fn get_two_mut(&mut self, i: usize, j: usize) -> (&mut Circle, &mut Circle) {
        assert!(i != j);
        let (first, second) = if i < j {
            let (left, right) = self.circles.split_at_mut(j);
            (&mut left[i], &mut right[0])
        } else {
            let (left, right) = self.circles.split_at_mut(i);
            (&mut right[0], &mut left[j])
        };
        (first, second)
    }

    pub fn avoid_collision(circle_a: &mut Circle, circle_b: &mut Circle) {
        let mut dx = circle_b.x_pos - circle_a.x_pos;
        let mut dy = circle_b.y_pos - circle_a.y_pos;
        let distance = ((dx * dx) + (dy * dy)).sqrt();
        let min_distance = circle_a.radius + circle_b.radius;

        if min_distance <= distance {
            return;
        }

        // Avoid division by zero
        let (nx, ny) = if distance > 1e-8 {
            // Normal vector (collision axis)
            (dx / distance, dy / distance)
        } else {
            // Circles are at the same position; choose an arbitrary normal vector
            // Also, slightly separate the circles to avoid overlap
            let separation = min_distance - distance + 1e-8;
            circle_a.x_pos -= separation / 2.0;
            circle_b.x_pos += separation / 2.0;
            dx = circle_b.x_pos - circle_a.x_pos;
            dy = circle_b.y_pos - circle_a.y_pos;
            (dx / separation, dy / separation)
        };

        // Tangent vector (perpendicular to normal)
        let tx = -ny;
        let ty = nx;

        // Decompose velocities into normal and tangential components
        let v_an = nx * circle_a.velocity.0 + ny * circle_a.velocity.1;
        let v_at = tx * circle_a.velocity.0 + ty * circle_a.velocity.1;

        let v_bn = nx * circle_b.velocity.0 + ny * circle_b.velocity.1;
        let v_bt = tx * circle_b.velocity.0 + ty * circle_b.velocity.1;

        // Masses (you might want to define mass based on area or keep it uniform)
        let m1 = 1.0;
        let m2 = 1.0;

        // Compute new normal velocities using 1D elastic collision equations
        let v_an_new = (v_an * (m1 - m2) + 2.0 * m2 * v_bn) / (m1 + m2);
        let v_bn_new = (v_bn * (m2 - m1) + 2.0 * m1 * v_an) / (m1 + m2);

        // Final velocities by recombining normal and tangential components
        circle_a.velocity.0 = v_an_new * nx + v_at * tx;
        circle_a.velocity.1 = v_an_new * ny + v_at * ty;

        circle_b.velocity.0 = v_bn_new * nx + v_bt * tx;
        circle_b.velocity.1 = v_bn_new * ny + v_bt * ty;

        // Resolve overlap by moving circles apart
        let overlap = 0.5 * (min_distance - distance);
        circle_a.x_pos -= overlap * nx;
        circle_a.y_pos -= overlap * ny;
        circle_b.x_pos += overlap * nx;
        circle_b.y_pos += overlap * ny;
    }

    pub fn view(&self) -> iced::Element<Message> {
        iced::widget::Canvas::new(self).into()
    }
}

#[derive(Debug, Clone)]
pub struct Circle {
    pub x_pos: f32,
    pub y_pos: f32,
    pub radius: f32,
    pub velocity: (f32, f32),
}

impl Program<Message> for Grid {
    type State = Interaction;

    fn draw(
        &self,
        _interaction: &Interaction,
        renderer: &Renderer,
        _theme: &Theme,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, Size::new(self.width, self.height));

        for cell in &self.circles {
            let cell = Path::circle(Point::new(cell.x_pos, cell.y_pos), cell.radius);
            frame.fill(&cell, Color::WHITE);
        }

        vec![frame.into_geometry()]
    }
}
