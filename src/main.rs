use futures::{channel::mpsc, StreamExt};
use iced::{
    window::{settings::PlatformSpecific, Settings},
    Element, Length, Size, Subscription, Task, Theme,
};
use physics::{Circle, GridFrame, GridMessage};

mod physics;

const TARGET_FPS: u64 = 120;

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
    SetGridFrame(physics::GridFrame),
    SetGridMessageSender(mpsc::Sender<physics::GridMessage>),
    AddCircle(Circle),
}

#[derive(Default)]
struct App {
    grid_message_sender: Option<mpsc::Sender<physics::GridMessage>>,
    current_grid_frame: Option<physics::GridFrame>,
}

impl App {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SetGridFrame(grid_frame) => {
                let frame_number = grid_frame.get_frame_number();

                self.current_grid_frame = Some(grid_frame);

                if frame_number % 20 == 0 {
                    return Task::done(Message::AddCircle(Circle {
                        x_pos: 25.0,
                        y_pos: 25.0,
                        radius: 25.0,
                        velocity: (10.0, 0.0),
                    }));
                }
            }
            Message::SetGridMessageSender(grid_message_sender) => {
                self.grid_message_sender = Some(grid_message_sender);
            }
            Message::AddCircle(circle) => {
                if let Some(grid_message_sender) = self.grid_message_sender.as_mut() {
                    if grid_message_sender
                        .try_send(GridMessage::AddCircle(circle))
                        .is_err()
                    {
                        println!("Failed to send AddCircle message to grid_message_sender.");
                    }
                } else {
                    println!("No grid_message_sender to send AddCircle message to.")
                }
            }
        }

        Task::none()
    }

    fn view(&self) -> Element<Message> {
        if let Some(current_grid_frame) = &self.current_grid_frame {
            current_grid_frame.view()
        } else {
            iced::widget::Space::new(Length::Fill, Length::Fill).into()
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::Subscription::run_with_id(
            std::any::TypeId::of::<GridFrame>(),
            // We're wrapping `stream` in a `stream!` macro to make it lazy (meaning `stream` isn't
            // created unless the outer `stream!` is actually used). This is necessary because the
            // outer `stream!` is created on every update, but will only be polled if the subscription
            // ID is new.
            async_stream::stream! {
                let (grid_message_sender, grid_frame_stream) =
                    physics::new_throttled_grid_frame_stream(APP_WIDTH, APP_HEIGHT, TARGET_FPS);

                yield Message::SetGridMessageSender(grid_message_sender);

                let mut grid_frame_stream = Box::pin(grid_frame_stream);

                while let Some(msg) = grid_frame_stream.next().await {
                    yield Message::SetGridFrame(msg);
                }
            },
        )
    }
}
