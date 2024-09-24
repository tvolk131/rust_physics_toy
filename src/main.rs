use futures::{channel::mpsc, StreamExt};
use iced::{
    window::{settings::PlatformSpecific, Settings},
    Element, Length, Size, Subscription, Task, Theme,
};
use physics::{Circle, GridFrame, GridMessage, StaticCircle, StaticRectangle};

mod physics;

const TARGET_FPS: u64 = 120;

const APP_WIDTH: f32 = 800.0;
const APP_HEIGHT: f32 = 480.0;

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
            min_size: None,
            max_size: None,
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
    ResizeWindow(Size),
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

                if frame_number % 10 == 0 {
                    return Task::done(Message::AddCircle(Circle {
                        x_pos: 10.0,
                        y_pos: 10.0,
                        radius: 10.0,
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
            Message::ResizeWindow(size) => {
                if let Some(grid_message_sender) = self.grid_message_sender.as_mut() {
                    if grid_message_sender
                        .try_send(GridMessage::Resize(size))
                        .is_err()
                    {
                        println!("Failed to resize grid window.");
                    }
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
        let mut subscriptions = Vec::new();

        subscriptions.push(iced::Subscription::run_with_id(
            std::any::TypeId::of::<GridFrame>(),
            // We're wrapping `stream` in a `stream!` macro to make it lazy (meaning `stream` isn't
            // created unless the outer `stream!` is actually used). This is necessary because the
            // outer `stream!` is created on every update, but will only be polled if the subscription
            // ID is new.
            async_stream::stream! {
                let (mut grid_message_sender, grid_frame_stream) =
                    physics::new_throttled_grid_frame_stream(APP_WIDTH, APP_HEIGHT, TARGET_FPS);

                let square_size = 200.0;
                for message in create_rounded_rectangle(APP_WIDTH / 2.0 - square_size / 2.0, APP_HEIGHT / 2.0 - square_size / 2.0, square_size, square_size, 20.0) {
                    grid_message_sender.try_send(message).unwrap();
                }

                yield Message::SetGridMessageSender(grid_message_sender);

                let mut grid_frame_stream = Box::pin(grid_frame_stream);

                while let Some(msg) = grid_frame_stream.next().await {
                    yield Message::SetGridFrame(msg);
                }
            },
        ));

        subscriptions
            .push(iced::window::resize_events().map(|(_, size)| Message::ResizeWindow(size)));

        iced::Subscription::batch(subscriptions)
    }
}

fn create_rounded_rectangle(
    x_pos: f32,
    y_pos: f32,
    width: f32,
    height: f32,
    border_radius: f32,
) -> Vec<GridMessage> {
    let mut messages = Vec::new();

    // Horizontal rectangle in the middle
    messages.push(GridMessage::AddStaticRectangle(StaticRectangle {
        x_pos: x_pos + border_radius,
        y_pos,
        width: width - 2.0 * border_radius,
        height,
    }));

    // Vertical rectangle in the middle
    messages.push(GridMessage::AddStaticRectangle(StaticRectangle {
        x_pos,
        y_pos: y_pos + border_radius,
        width,
        height: height - 2.0 * border_radius,
    }));

    // Top-left corner
    messages.push(GridMessage::AddStaticCircle(StaticCircle {
        x_pos: x_pos + border_radius,
        y_pos: y_pos + border_radius,
        radius: border_radius,
    }));

    // Top-right corner
    messages.push(GridMessage::AddStaticCircle(StaticCircle {
        x_pos: x_pos + width - border_radius,
        y_pos: y_pos + border_radius,
        radius: border_radius,
    }));

    // Bottom-left corner
    messages.push(GridMessage::AddStaticCircle(StaticCircle {
        x_pos: x_pos + border_radius,
        y_pos: y_pos + height - border_radius,
        radius: border_radius,
    }));

    // Bottom-right corner
    messages.push(GridMessage::AddStaticCircle(StaticCircle {
        x_pos: x_pos + width - border_radius,
        y_pos: y_pos + height - border_radius,
        radius: border_radius,
    }));

    messages
}
