use futures::{channel::mpsc, stream::Stream};
use iced::{
    mouse::{self, Interaction},
    widget::canvas::{Frame, Geometry, Path, Program},
    Color, Point, Rectangle, Renderer, Size, Theme,
};

use std::collections::HashMap;
use std::time::Duration;

const SUBTICKS_PER_FRAME: u32 = 10;
const ELASTICITY_COEFFICIENT: f32 = 0.7;
const AIR_DENSITY: f32 = 0.005;
const SIZE_COEFFICIENT_PER_TICK: f32 = 0.998;
const MIN_RADIUS_SIZE: f32 = 0.5;
const GRAVITY: f32 = 0.2;
const CELL_SIZE: f32 = 50.0;
const BALL_COLOR: Color = Color::from_rgb(1.0, 0.6, 0.0);

use crate::Message;

pub fn new_throttled_grid_frame_stream(
    width: f32,
    height: f32,
    target_fps: u64,
) -> (mpsc::Sender<GridMessage>, impl Stream<Item = GridFrame>) {
    let (mut grid, grid_message_sender) = Grid::new(width, height);

    let grid_frame_stream = async_stream::stream! {

        let mut interval = tokio::time::interval_at(tokio::time::Instant::now(), Duration::from_millis(1000 / target_fps));

        // FPS counter variables.
        const FPS_MEASUREMENT_INTERVAL: Duration = Duration::from_secs(5);
        let mut frame_counter_count = 0;
        let mut frame_counter_start = tokio::time::Instant::now();

        loop {
            interval.tick().await;

            let mut messages = Vec::new();
            while let Ok(Some(message)) = grid.message_receiver.try_next() {
                messages.push(message);
            }

            frame_counter_count += 1;
            let elapsed = frame_counter_start.elapsed();
            if elapsed >= FPS_MEASUREMENT_INTERVAL {
                println!("FPS: {}", frame_counter_count as f32 / elapsed.as_secs_f32());
                frame_counter_count = 0;
                frame_counter_start = tokio::time::Instant::now();
            }

            yield grid.tick(SUBTICKS_PER_FRAME, messages);
        }
    };

    (grid_message_sender, grid_frame_stream)
}

pub enum GridMessage {
    AddCircle(Circle),
}

#[derive(Debug, Clone)]
pub struct GridFrame {
    frame_number: u32,
    width: f32,
    height: f32,
    circles: Vec<Circle>,
}

impl GridFrame {
    pub fn get_frame_number(&self) -> u32 {
        self.frame_number
    }

    pub fn view(&self) -> iced::Element<Message> {
        iced::widget::Canvas::new(self).into()
    }
}

struct Grid {
    frame_number: u32,
    width: f32,
    height: f32,
    circles: Vec<Circle>,
    message_receiver: mpsc::Receiver<GridMessage>,
}

impl Grid {
    fn new(width: f32, height: f32) -> (Self, mpsc::Sender<GridMessage>) {
        let (message_sender, message_receiver) = mpsc::channel(100);

        (
            Self {
                frame_number: 0,
                width,
                height,
                circles: Vec::new(),
                message_receiver,
            },
            message_sender,
        )
    }

    fn tick(&mut self, sub_ticks: u32, messages: Vec<GridMessage>) -> GridFrame {
        for message in messages {
            match message {
                GridMessage::AddCircle(circle) => self.circles.push(circle),
            }
        }

        // Apply subtick-independent forces first.
        for circle in &mut self.circles {
            // Apply air resistance to all circles.
            let velocity = (circle.velocity.0.powi(2) + circle.velocity.1.powi(2)).sqrt();
            let resistance = velocity * AIR_DENSITY;
            let angle = circle.velocity.1.atan2(circle.velocity.0);
            circle.velocity.0 -= resistance * angle.cos();
            circle.velocity.1 -= resistance * angle.sin();

            // Change circle sizes.
            circle.radius *= SIZE_COEFFICIENT_PER_TICK;
        }

        self.circles
            .retain(|circle| circle.radius >= MIN_RADIUS_SIZE);

        for _ in 0..sub_ticks {
            // Apply gravity to all circles.
            for circle in &mut self.circles {
                circle.velocity.1 += GRAVITY / sub_ticks as f32;
            }

            // Move circles based on current velocity.
            for circle in &mut self.circles {
                circle.x_pos += circle.velocity.0 / sub_ticks as f32;
                circle.y_pos += circle.velocity.1 / sub_ticks as f32;
            }

            // Bounce circles off the walls, applying friction.
            for circle in &mut self.circles {
                if circle.x_pos - circle.radius < 0.0 {
                    circle.x_pos = circle.radius;
                    circle.velocity.0 = -circle.velocity.0 * ELASTICITY_COEFFICIENT;
                }

                if circle.x_pos + circle.radius > self.width {
                    circle.x_pos = self.width - circle.radius;
                    circle.velocity.0 = -circle.velocity.0 * ELASTICITY_COEFFICIENT;
                }

                if circle.y_pos - circle.radius < 0.0 {
                    circle.y_pos = circle.radius;
                    circle.velocity.1 = -circle.velocity.1 * ELASTICITY_COEFFICIENT;
                }

                if circle.y_pos + circle.radius > self.height {
                    circle.y_pos = self.height - circle.radius;
                    circle.velocity.1 = -circle.velocity.1 * ELASTICITY_COEFFICIENT;
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

        self.frame_number += 1;

        GridFrame {
            frame_number: self.frame_number,
            width: self.width,
            height: self.height,
            circles: self.circles.clone(),
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

    fn avoid_collision(circle_a: &mut Circle, circle_b: &mut Circle) {
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

        // Masses, based on the circle areas
        let m1 = circle_a.radius * circle_a.radius;
        let m2 = circle_b.radius * circle_b.radius;

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
}

#[derive(Debug, Clone)]
pub struct Circle {
    pub x_pos: f32,
    pub y_pos: f32,
    pub radius: f32,
    pub velocity: (f32, f32),
}

impl Program<Message> for GridFrame {
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

        for circle in &self.circles {
            frame.fill(
                &Path::circle(Point::new(circle.x_pos, circle.y_pos), circle.radius),
                BALL_COLOR,
            );
        }

        vec![frame.into_geometry()]
    }
}
