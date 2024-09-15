use std::time::Duration;

use iced::{
    window::{settings::PlatformSpecific, Settings},
    Element, Size, Subscription, Task, Theme,
};
use physics::Circle;

mod physics;

const TICK_FPS: u64 = 120;
const TICK_SPEED: Duration = Duration::from_millis(1000 / TICK_FPS);

const APP_WIDTH: f32 = 1024.0;
const APP_HEIGHT: f32 = 768.0;

fn main() -> iced::Result {
    iced::application("Physics", App::update, App::view)
        .subscription(App::subscription)
        .theme(|_| Theme::Dark)
        .window(Settings {
            size: iced::Size {
                width: APP_WIDTH,
                height: APP_HEIGHT,
            },
            position: iced::window::Position::Default,
            min_size: Some(Size {
                width: APP_WIDTH,
                height: APP_HEIGHT,
            }),
            max_size: Some(Size {
                width: APP_WIDTH,
                height: APP_HEIGHT,
            }),
            visible: true,
            resizable: true,
            decorations: true,
            transparent: false,
            level: iced::window::Level::Normal,
            icon: None,                                     // TODO: Set icon.
            platform_specific: PlatformSpecific::default(), // TODO: Set platform specific settings for each platform.
            exit_on_close_request: true,
        })
        .run()
}

#[derive(Debug, Clone)]
pub enum Message {
    // Perform one tick/step of the physics simulation.
    Tick,
    AddCircle(Circle),
}

struct App {
    is_running: bool,
    grid: physics::Grid,
}

impl Default for App {
    fn default() -> Self {
        Self {
            is_running: true,
            grid: physics::Grid::new(APP_WIDTH, APP_HEIGHT),
        }
    }
}

impl App {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick => {
                let now = std::time::Instant::now();

                self.grid.tick(10);

                let circle_count = self.grid.get_circle_count();
                if circle_count > 0 {
                    println!(
                        "Tick took: {:?} ({:?} per circle)",
                        now.elapsed(),
                        now.elapsed() / circle_count as u32
                    );
                } else {
                    println!("Tick took: {:?}", now.elapsed());
                }
            }
            Message::AddCircle(circle) => {
                self.grid.add_circle(circle);
            }
        }

        Task::none()
    }

    fn view(&self) -> Element<Message> {
        self.grid.view()
    }

    fn subscription(&self) -> Subscription<Message> {
        let mut subscriptions = Vec::new();

        if self.is_running {
            subscriptions.push(iced::time::every(TICK_SPEED).map(|_| Message::Tick));
            subscriptions.push(iced::time::every(Duration::from_millis(200)).map(|_| {
                Message::AddCircle(Circle {
                    x_pos: 2.5,
                    y_pos: 2.5,
                    radius: 2.5,
                    velocity: (10.0, 0.0),
                })
            }));
        }

        Subscription::batch(subscriptions)
    }
}
