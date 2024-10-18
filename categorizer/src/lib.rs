use database::{Actor, Director, FullMovie};
use iced::{
    widget::{
        button, column, container, pane_grid::state::Action, row, text, text_input, Column,
        Container,
    },
    Application, Command, Theme,
};

#[derive(Debug, Clone)]
pub enum Message {
    InputChanged(
        String,
        MovieInputChange,
    ),
    SaveMovie(FullMovie),
    Navigation(Page),
}

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

#[derive(Debug, Clone, Default)]
pub struct NewMovieInput {
    name: String,
    description: String,
    director: String,
    genres: String,
    actors: String,
}

impl<'a> Into<FullMovie> for &'a NewMovieInput {
    fn into(self) -> FullMovie {
        let genres = self
            .genres
            .split("|")
            .map(|genre| genre.trim())
            .filter(|genre| !genre.is_empty())
            .map(|genre| genre.to_string())
            .collect();

        let actors = self
            .actors
            .split("|")
            .map(|actor| actor.trim())
            .filter(|actor| !actor.is_empty())
            .map(|actor| Actor::from(actor.to_string()))
            .collect();

        let director = match self
            .director
            .is_empty()
        {
            true => Some(
                Director::from(
                    self.director
                        .to_string(),
                ),
            ),
            false => None,
        };

        FullMovie {
            id: 0,
            name: self
                .name
                .to_string(),
            description: Some(
                self.description
                    .to_string(),
            ),
            director,
            genres,
            actors,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MovieInputChange {
    Name = 1,
    Description,
    Director,
    Genres,
    Actors,
}

#[derive(Debug, Clone)]
pub enum Page {
    List,
    NewMovie(NewMovieInput),
}

impl Application for App {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(
        _flags: Self::Flags,
    ) -> (
        Self,
        iced::Command<Self::Message>,
    ) {
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
        match &self
            .state
            .desired_theme
        {
            Theme::Light => Theme::Light,
            Theme::Dark => Theme::Dark,
            _ => Theme::Dark,
        }
    }

    fn title(&self) -> String {
        "DVD Categorizer".to_string()
    }

    fn update(&mut self, message: Self::Message) -> iced::Command<Self::Message> {
        match message {
            Message::InputChanged(value, movie_input_change) => {
                input_changed(
                    value,
                    movie_input_change,
                    self,
                );

                Command::none()
            }
            Message::SaveMovie(movie) => {
                let result = database::insert_full_movie(movie);

                if let Ok(movie) = result {
                    self.state
                        .movies
                        .push(movie);
                    self.state
                        .current_view = Page::List;
                }

                Command::none()
            }
            Message::Navigation(page) => {
                self.state
                    .current_view = page;

                Command::none()
            }
        }
    }

    fn view(&self) -> iced::Element<'_, Self::Message, Self::Theme, iced::Renderer> {
        let view = match &self
            .state
            .current_view
        {
            Page::List => create_list_ui(
                &self
                    .state
                    .movies,
            ),
            Page::NewMovie(new_movie_input) => create_movie(new_movie_input),
        };

        column![
            row![
                button("List").on_press(Message::Navigation(Page::List)),
                button("New")
                    .on_press(Message::Navigation(Page::NewMovie(NewMovieInput::default()))),
            ],
            view
        ]
        .into()
    }
}

fn create_list_ui(movies: &[FullMovie]) -> container::Container<'_, Message> {
    let movie_columns: Vec<iced::widget::Column<'_, Message, iced::Theme, iced::Renderer>> = movies
        .iter()
        .map(create_movie_column)
        .collect();

    let main_column = Column::new()
        .padding(20)
        .spacing(20)
        .push(text("Movie List").size(24))
        .push(
            movie_columns
                .into_iter()
                .fold(
                    Column::new().spacing(40),
                    |column, movie_column| column.push(movie_column),
                ),
        );

    let movie_container = container(main_column);

    movie_container
}

fn create_movie_column(
    movie: &FullMovie,
) -> iced::widget::Column<'_, Message, iced::Theme, iced::Renderer> {
    column![
        text(
            movie
                .name
                .as_str()
        ),
        text(
            movie
                .description
                .as_deref()
                .unwrap_or("")
        ),
        text(
            movie
                .actors
                .iter()
                .map(
                    |actor| actor
                        .name
                        .as_str()
                )
                .collect::<Vec<&str>>()
                .join(", ")
        ),
        text(
            movie
                .genres
                .join(", ")
        ),
    ]
}

fn create_movie<'a>(new_movie_input: &NewMovieInput) -> container::Container<'_, Message> {
    let view = column![
        row![
            text("Movie Name"),
            text_input(
                "Movie Name",
                &new_movie_input.name
            )
            .on_input(
                |value| Message::InputChanged(
                    value,
                    MovieInputChange::Name
                )
            )
        ],
        row![
            text("Movie Descripton"),
            text_input(
                "Movie Description",
                &new_movie_input.description
            )
            .on_input(
                |value| Message::InputChanged(
                    value,
                    MovieInputChange::Description
                )
            )
        ],
        row![
            text("Director"),
            text_input(
                "Jackie Chan",
                &new_movie_input.director
            )
            .on_input(
                |value| Message::InputChanged(
                    value,
                    MovieInputChange::Director
                )
            )
        ],
        row![
            text("Movie Actors"),
            text_input(
                "Tom Cruise | Jackie Chan",
                &new_movie_input.actors
            )
            .on_input(
                |value| Message::InputChanged(
                    value,
                    MovieInputChange::Actors
                )
            )
        ],
        row![
            text("Movie Genres"),
            text_input(
                "Action | Comedy",
                &new_movie_input.genres
            )
            .on_input(
                |value| Message::InputChanged(
                    value,
                    MovieInputChange::Genres
                )
            )
        ],
        row![button("Submit").on_press(Message::SaveMovie(new_movie_input.into()))]
    ];

    Container::new(view)
}

fn input_changed(value: String, change: MovieInputChange, app: &mut App) {
    if let Page::NewMovie(ref mut new_movie_input) = app
        .state
        .current_view
    {
        match change {
            MovieInputChange::Name => new_movie_input.name = value,
            MovieInputChange::Description => new_movie_input.description = value,
            MovieInputChange::Genres => new_movie_input.genres = value,
            MovieInputChange::Actors => new_movie_input.actors = value,
            MovieInputChange::Director => new_movie_input.director = value,
        }
    }
}
