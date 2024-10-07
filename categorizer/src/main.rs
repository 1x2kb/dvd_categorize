use database::{FullMovie, Movie};
use dotenvy::dotenv;
use iced::{
    widget::{column, container, image, row, text, Column, Image, Row},
    Application, Color, Command, Length, Settings, Theme,
};

#[derive(Debug, Clone)]
pub enum Message {}

#[derive(Debug, Clone)]
pub struct App {
    state: AppState,
}

#[derive(Debug, Clone)]
pub struct AppState {
    movies: Vec<FullMovie>,
    current_view: Page,
    desired_theme: Theme,
}

#[derive(Debug, Clone)]
pub enum Page {
    List,
}

impl Application for App {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, iced::Command<Self::Message>) {
        let movies = database::get_movies().expect("Failed to retrieve movies");

        (
            Self {
                state: AppState {
                    movies,
                    current_view: Page::List,
                    desired_theme: Theme::Dark,
                },
            },
            Command::none(),
        )
    }

    fn theme(&self) -> Self::Theme {
        match &self.state.desired_theme {
            Theme::Light => Theme::Light,
            Theme::Dark => Theme::Dark,
            _ => Theme::Dark,
        }
    }

    fn title(&self) -> String {
        "DVD Categorizer".to_string()
    }

    fn update(&mut self, message: Self::Message) -> iced::Command<Self::Message> {
        Command::none()
    }

    fn view(&self) -> iced::Element<'_, Self::Message, Self::Theme, iced::Renderer> {
        match &self.state.current_view {
            Page::List => {
                let movie_columns: Vec<
                    iced::widget::Column<'_, Message, iced::Theme, iced::Renderer>,
                > = self
                    .state
                    .movies
                    .iter()
                    .map(App::create_movie_column)
                    .collect();

                let main_column = Column::new()
                    .padding(20)
                    .spacing(20)
                    .push(text("Movie List").size(24))
                    .push(
                        movie_columns
                            .into_iter()
                            .fold(Column::new().spacing(40), |column, movie_column| {
                                column.push(movie_column)
                            }),
                    );

                let movie_container = container(main_column);

                movie_container
            }
            .into(),
        }
    }
}

impl App {
    fn create_movie_column<'a>(
        movie: &'a FullMovie,
    ) -> iced::widget::Column<'a, Message, iced::Theme, iced::Renderer> {
        column![
            text(&movie.name),
            text(movie.description.as_ref().unwrap_or(&String::new())),
            text(
                movie
                    .actors
                    .iter()
                    .map(|actor| actor.name.as_str())
                    .collect::<Vec<&str>>()
                    .join(", ")
            ),
            text(movie.genres.join(", ")),
        ]
    }
}

fn main() -> iced::Result {
    dotenv().ok().expect("Failed to run env reader");
    App::run(Settings::default())
}
