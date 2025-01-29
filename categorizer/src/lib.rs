use std::{
    env,
    error::Error,
    fmt::{Display, Formatter},
};

use iced::{
    futures::TryFutureExt,
    widget::{button, column, container, row, scrollable, text, text_input, Column, Container},
    Application, Command, Theme,
};
use log::{error, info};
use models::{Actor, Director, FullMovie};
use tracing::{instrument, Level};

pub trait ConnectOnce {
    fn connection_string(&self) -> String;
}

#[derive(Debug, Clone)]
pub enum UiError {
    Reqwest(String),
    Serde(String),
}

impl Display for UiError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            UiError::Reqwest(error) => write!(
                f,
                "{error}"
            ),
            UiError::Serde(error) => write!(
                f,
                "{error}"
            ),
        }
    }
}

impl Error for UiError {}

impl From<reqwest::Error> for UiError {
    fn from(value: reqwest::Error) -> Self {
        UiError::Reqwest(value.to_string())
    }
}

impl From<serde_json::Error> for UiError {
    fn from(value: serde_json::Error) -> Self {
        UiError::Serde(value.to_string())
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    MovieInputChanged(
        String,
        MovieInputChange,
    ),
    ChatInputChanged(String),
    LoadDvds,
    LoadedDvds(Result<Vec<FullMovie>, UiError>),
    SaveDvd(FullMovie),
    SavedDvd(Result<FullMovie, UiError>),
    Navigation(Page),
    SendChatMessage(String),
}

impl Display for Message {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Message::MovieInputChanged(value, movie_input_change) => write!(
                f,
                "Message::InputChanged event {movie_input_change} to {value}"
            ),
            Message::ChatInputChanged(value) => write!(
                f,
                "Message::ChatInputChanged event input to {value}"
            ),
            Message::LoadDvds => write!(
                f,
                "Message::LoadDvds"
            ),
            Message::LoadedDvds(_) => write!(
                f,
                "Message::LoadedDvds"
            ),
            Message::SaveDvd(_) => write!(
                f,
                "Message::SaveDvd"
            ),
            Message::SavedDvd(_) => write!(
                f,
                "Message::SavedDvd"
            ),
            Message::Navigation(_) => write!(
                f,
                "Message::Navigation"
            ),
            Message::SendChatMessage(_) => write!(
                f,
                "Message::SendChatMessage"
            ),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ApiProperties {
    host: String,
    port: String,
}

impl ConnectOnce for ApiProperties {
    fn connection_string(&self) -> String {
        format!(
            "http://{}:{}",
            self.host, self.port
        )
    }
}

impl ConnectOnce for &ApiProperties {
    fn connection_string(&self) -> String {
        format!(
            "http://{}:{}",
            self.host, self.port
        )
    }
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
    api_properties: ApiProperties,
    loading: Option<bool>,
}

#[derive(Debug, Clone, Default)]
pub struct NewMovieInput {
    name: String,
    description: String,
    director: String,
    genres: String,
    actors: String,
}

#[derive(Debug, Clone, Default)]
pub struct ChatInput {
    user_input: String,
}

impl From<&NewMovieInput> for FullMovie {
    fn from(val: &NewMovieInput) -> Self {
        let genres = val
            .genres
            .split("|")
            .map(|genre| genre.trim())
            .filter(|genre| !genre.is_empty())
            .map(|genre| genre.to_string())
            .collect();

        let actors = val
            .actors
            .split("|")
            .map(|actor| actor.trim())
            .filter(|actor| !actor.is_empty())
            .map(|actor| Actor::from(actor.to_string()))
            .collect();

        let director = match val
            .director
            .is_empty()
        {
            true => Some(
                Director::from(
                    val.director
                        .to_string(),
                ),
            ),
            false => None,
        };

        FullMovie {
            id: 0,
            name: val
                .name
                .to_string(),
            description: Some(
                val.description
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

impl Display for MovieInputChange {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            MovieInputChange::Name => write!(
                f,
                "Changing name input"
            ),
            MovieInputChange::Description => write!(
                f,
                "Changing Description input"
            ),
            MovieInputChange::Director => write!(
                f,
                "Changing Director input"
            ),
            MovieInputChange::Genres => write!(
                f,
                "Changing Genres input"
            ),
            MovieInputChange::Actors => write!(
                f,
                "Changing actors input"
            ),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Page {
    List,
    NewMovie(NewMovieInput),
    Chat(ChatInput),
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
        (
            Self {
                state: AppState {
                    movies: Default::default(),
                    current_view: Page::List,
                    desired_theme: Theme::Dark,
                    api_properties: get_host(),
                    loading: None,
                },
            },
            Command::perform(
                async {},
                |_| Message::LoadDvds,
            ),
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
        info!("Current message {message}");
        match message {
            Message::MovieInputChanged(value, movie_input_change) => {
                input_changed(
                    value,
                    movie_input_change,
                    self,
                );

                Command::none()
            }
            Message::ChatInputChanged(input) => {
                chat_input_changed(
                    input, self,
                );
                Command::none()
            }
            Message::SaveDvd(movie) => {
                info!(
                    "Saving dvd {:?}",
                    &movie
                );

                Command::perform(
                    insert_dvd(
                        movie,
                        self.state
                            .api_properties
                            .connection_string(),
                    ),
                    Message::SavedDvd,
                )
            }
            Message::SavedDvd(full_movie) => {
                info!(
                    "Created {:?}",
                    full_movie
                );
                Command::perform(
                    async {},
                    |_| Message::Navigation(Page::List),
                )
            }
            Message::Navigation(page) => {
                self.state
                    .current_view = page;

                Command::none()
            }
            Message::LoadDvds => {
                self.state
                    .loading = Some(true);
                info!("Loading dvds. Set loading true");

                Command::perform(
                    get_dvds(
                        self.state
                            .api_properties
                            .connection_string(),
                    ),
                    Message::LoadedDvds,
                )
            }
            Message::LoadedDvds(vec) => {
                self.state
                    .movies = vec.unwrap_or_else(
                    |e| {
                        error!("Encountered error when entering LoadedDvds");
                        error!("{e}");
                        Vec::new()
                    },
                );

                self.state
                    .loading = Some(false);

                Command::perform(
                    async {},
                    |_| Message::Navigation(Page::List),
                )
            }
            Message::SendChatMessage(input) => {
                info!("Sending message to bot {input}");

                Command::perform(
                    send_bot_message(input),
                    |_| Message::Navigation(Page::List), // TODO: Handle response
                )
            }
        }
    }

    fn view(&self) -> iced::Element<'_, Self::Message, Self::Theme, iced::Renderer> {
        let page_view = match &self
            .state
            .current_view
        {
            Page::List => create_list_ui(
                &self
                    .state
                    .movies,
            ),
            Page::NewMovie(new_movie_input) => create_movie_ui(new_movie_input),
            Page::Chat(chat_input) => create_chat_ui(chat_input),
        };

        column![
            row![
                button("List").on_press(Message::Navigation(Page::List)),
                button("New").on_press(Message::Navigation(Page::NewMovie(Default::default()))),
                button("Chat").on_press(Message::Navigation(Page::Chat(Default::default())))
            ]
            .padding(20)
            .spacing(10),
            page_view
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

    let movie_container = container(scrollable(main_column));

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

fn create_movie_ui(new_movie_input: &NewMovieInput) -> container::Container<'_, Message> {
    let width = 150;

    let view = column![
        row![
            Container::new(text("Movie Name")).width(width),
            text_input(
                "Movie Name",
                &new_movie_input.name
            )
            .on_input(
                |value| Message::MovieInputChanged(
                    value,
                    MovieInputChange::Name
                )
            )
        ],
        row![
            Container::new(text("Movie Descripton")).width(width),
            text_input(
                "Movie Description",
                &new_movie_input.description
            )
            .on_input(
                |value| Message::MovieInputChanged(
                    value,
                    MovieInputChange::Description
                )
            )
        ],
        row![
            Container::new(text("Director")).width(width),
            text_input(
                "Jackie Chan",
                &new_movie_input.director
            )
            .on_input(
                |value| Message::MovieInputChanged(
                    value,
                    MovieInputChange::Director
                )
            )
        ],
        row![
            Container::new(text("Movie Actors")).width(width),
            text_input(
                "Tom Cruise | Jackie Chan",
                &new_movie_input.actors
            )
            .on_input(
                |value| Message::MovieInputChanged(
                    value,
                    MovieInputChange::Actors
                )
            )
        ],
        row![
            Container::new(text("Movie Genres")).width(width),
            text_input(
                "Action | Comedy",
                &new_movie_input.genres
            )
            .on_input(
                |value| Message::MovieInputChanged(
                    value,
                    MovieInputChange::Genres
                )
            )
        ],
        row![button("Submit").on_press(Message::SaveDvd(new_movie_input.into()))]
    ];

    Container::new(view)
}

fn create_chat_ui(chat_input: &ChatInput) -> container::Container<'_, Message> {
    let column = column![
        text_input(
            "Bot Response",
            &chat_input.user_input
        ),
        text_input(
            "Ask a question",
            &chat_input.user_input
        )
        .on_input(Message::ChatInputChanged),
        button("Submit").on_press(
            Message::SendChatMessage(
                chat_input
                    .user_input
                    .clone()
            )
        )
    ];

    Container::new(column)
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

fn chat_input_changed(value: String, app: &mut App) {
    if let Page::Chat(ref mut chat_input) = app
        .state
        .current_view
    {
        chat_input.user_input = value;
    }
}

#[instrument(level = Level::DEBUG)]
async fn get_dvds(url: String) -> Result<Vec<FullMovie>, UiError> {
    reqwest::get(format!("{url}/dvd"))
        .await?
        .json::<Vec<FullMovie>>()
        .await
        .map_err(
            |e| {
                error!("Failed to send request or parse response {e}");
                e
            },
        )
        .map_err(UiError::from)
}

#[instrument(level = Level::DEBUG)]
async fn insert_dvd(dvd: FullMovie, url: String) -> Result<FullMovie, UiError> {
    reqwest::Client::new()
        .post(format!("{url}/dvd"))
        .json(&dvd)
        .send()
        .map_err(
            |e| {
                error!(
                    "Error while sending load dvd request, {}",
                    e
                );

                e
            },
        )
        .await?
        .json::<FullMovie>()
        .await
        .map_err(
            |e| {
                error!(
                    "Error while parsing the dvd resposne {}",
                    e
                );
                UiError::from(e)
            },
        )
}

#[instrument(level = Level::DEBUG)]
async fn send_bot_message(message: String) {
    let result = ai_chat::bot_message(message).await;
    info!(
        "{:#?}",
        result
    );
}

fn get_host() -> ApiProperties {
    let host = env::var("server_host").unwrap_or("0.0.0.0".to_string());
    let port = env::var("server_port").unwrap_or("3000".to_string());

    ApiProperties { host, port }
}
