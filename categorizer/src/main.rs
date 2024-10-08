use categorizer::App;
use dotenvy::dotenv;
use iced::{Application, Settings};

fn main() -> iced::Result {
    dotenv().ok().expect("Failed to run env reader");
    App::run(Settings::default())
}
