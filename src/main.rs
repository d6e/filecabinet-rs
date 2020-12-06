use iced::futures::{AsyncReadExt, AsyncWriteExt};
use iced::{
    button, scrollable, text_input, Align, Application, Button, Checkbox, Column, Command,
    Container, Element, Font, HorizontalAlignment, Length, Row, Scrollable, Settings, Text,
    TextInput,
};
use serde::{Deserialize, Serialize};
use std::collections::linked_list::Iter;
use std::env;
use std::path::Path;

mod utils;

pub fn main() -> iced::Result {
    FileCabinet::run(Settings::default())
}

#[derive(Debug)]
enum FileCabinet {
    Loading,
    Loaded(State),
}

#[derive(Debug, Default)]
struct State {
    scroll: scrollable::State,
    path: text_input::State,
    path_value: String,
    filter: Filter,
    docs: Vec<Document>,
    controls: Controls,
    dirty: bool,
    saving: bool,
}

#[derive(Debug, Clone)]
enum Message {
    Loaded(Result<SavedState, LoadError>),
    Saved(Result<(), SaveError>),
    PathChanged(String),
    CreateTask,
    FilterChanged(Filter),
    TaskMessage(usize, TaskMessage),
}

impl Application for FileCabinet {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_flags: ()) -> (FileCabinet, Command<Message>) {
        (
            FileCabinet::Loading,
            Command::perform(SavedState::load(), Message::Loaded),
        )
    }

    fn title(&self) -> String {
        let dirty = match self {
            FileCabinet::Loading => false,
            FileCabinet::Loaded(state) => state.dirty,
        };

        format!("Filecabinet {}", if dirty { "*" } else { "" })
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match self {
            FileCabinet::Loading => {
                match message {
                    Message::Loaded(Ok(state)) => {
                        *self = FileCabinet::Loaded(State {
                            path_value: state.path,
                            filter: state.filter,
                            docs: state.docs,
                            ..State::default()
                        });
                    }
                    Message::Loaded(Err(_)) => {
                        *self = FileCabinet::Loaded(State::default());
                    }
                    _ => {}
                }

                Command::none()
            }
            FileCabinet::Loaded(state) => {
                let mut saved = false;

                match message {
                    Message::PathChanged(value) => {
                        state.path_value = value;
                        state.docs = utils::list_files(&Path::new(&state.path_value).to_path_buf())
                            .iter()
                            .map(|path| Document {
                                path: path.to_owned(),
                                completed: false,
                                state: Default::default(),
                            })
                            .collect();
                    }
                    // Message::CreateTask => {
                    //     if !state.input_value.is_empty() {
                    //         state.docs.push(Document::new(state.input_value.clone()));
                    //         state.input_value.clear();
                    //     }
                    // }
                    Message::FilterChanged(filter) => {
                        state.filter = filter;
                    }
                    Message::TaskMessage(i, TaskMessage::Delete) => {
                        state.docs.remove(i);
                    }
                    Message::TaskMessage(i, task_message) => {
                        if let Some(doc) = state.docs.get_mut(i) {
                            doc.update(task_message);
                        }
                    }
                    Message::Saved(_) => {
                        state.saving = false;
                        saved = true;
                    }
                    _ => {}
                }

                if !saved {
                    state.dirty = true;
                }

                if state.dirty && !state.saving {
                    state.dirty = false;
                    state.saving = true;

                    Command::perform(
                        SavedState {
                            path: state.path_value.clone(),
                            filter: state.filter,
                            docs: state.docs.clone(),
                        }
                        .save(),
                        Message::Saved,
                    )
                } else {
                    Command::none()
                }
            }
        }
    }

    fn view(&mut self) -> Element<Message> {
        match self {
            FileCabinet::Loading => loading_message(),
            FileCabinet::Loaded(State {
                scroll,
                path,
                path_value,
                filter,
                docs,
                controls,
                ..
            }) => {
                let title = Text::new("filecabinet")
                    .width(Length::Fill)
                    .size(100)
                    .color([0.5, 0.5, 0.5])
                    .horizontal_alignment(HorizontalAlignment::Center);

                let path_input = TextInput::new(
                    path,
                    "Specify path to documents",
                    path_value,
                    Message::PathChanged,
                )
                .padding(10)
                .size(16)
                .on_submit(Message::CreateTask);

                let controls = controls.view(&docs, *filter);
                let filtered_tasks = docs.iter().filter(|doc| filter.matches(doc));

                let docs: Element<_> = if filtered_tasks.count() > 0 {
                    docs.iter_mut()
                        .enumerate()
                        .filter(|(_, doc)| filter.matches(doc))
                        .fold(Column::new().spacing(20), |column, (i, doc)| {
                            column.push(
                                doc.view()
                                    .map(move |message| Message::TaskMessage(i, message)),
                            )
                        })
                        .into()
                } else {
                    empty_message(match filter {
                        Filter::All => "No files found...",
                        Filter::Normalized => "",
                        Filter::Unnormalized => "",
                    })
                };

                let content = Column::new()
                    .max_width(800)
                    .spacing(20)
                    .push(title)
                    .push(path_input)
                    .push(controls)
                    .push(docs);

                Scrollable::new(scroll)
                    .padding(40)
                    .push(Container::new(content).width(Length::Fill).center_x())
                    .into()
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Document {
    path: String,
    completed: bool,

    #[serde(skip)]
    state: TaskState,
}

#[derive(Debug, Clone)]
pub enum TaskState {
    Idle {
        edit_button: button::State,
    },
    Editing {
        text_input: text_input::State,
        delete_button: button::State,
    },
}

impl Default for TaskState {
    fn default() -> Self {
        TaskState::Idle {
            edit_button: button::State::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum TaskMessage {
    Completed(bool),
    Edit,
    PathEdited(String),
    FinishEdition,
    Delete,
}

impl Document {
    fn new(path: String) -> Self {
        Document {
            path,
            completed: false,
            state: TaskState::Idle {
                edit_button: button::State::new(),
            },
        }
    }

    fn update(&mut self, message: TaskMessage) {
        match message {
            TaskMessage::Completed(completed) => {
                self.completed = completed;
            }
            TaskMessage::Edit => {
                self.state = TaskState::Editing {
                    text_input: text_input::State::focused(),
                    delete_button: button::State::new(),
                };
            }
            TaskMessage::PathEdited(new_path) => {
                self.path = new_path;
            }
            TaskMessage::FinishEdition => {
                if !self.path.is_empty() {
                    self.state = TaskState::Idle {
                        edit_button: button::State::new(),
                    }
                }
            }
            TaskMessage::Delete => {}
        }
    }

    fn view(&mut self) -> Element<TaskMessage> {
        match &mut self.state {
            TaskState::Idle { edit_button } => {
                let checkbox = Checkbox::new(self.completed, &self.path, TaskMessage::Completed)
                    .width(Length::Fill);

                Row::new()
                    .spacing(20)
                    .align_items(Align::Center)
                    .push(checkbox)
                    .push(
                        Button::new(edit_button, edit_icon())
                            .on_press(TaskMessage::Edit)
                            .padding(10)
                            .style(style::Button::Icon),
                    )
                    .into()
            }
            TaskState::Editing {
                text_input,
                delete_button,
            } => {
                let text_input = TextInput::new(
                    text_input,
                    "Document Name",
                    &self.path,
                    TaskMessage::PathEdited,
                )
                .on_submit(TaskMessage::FinishEdition)
                .padding(10);

                Row::new()
                    .spacing(20)
                    .align_items(Align::Center)
                    .push(text_input)
                    .push(
                        Button::new(
                            delete_button,
                            Row::new()
                                .spacing(10)
                                .push(delete_icon())
                                .push(Text::new("Delete")),
                        )
                        .on_press(TaskMessage::Delete)
                        .padding(10)
                        .style(style::Button::Destructive),
                    )
                    .into()
            }
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Controls {
    all_button: button::State,
    active_button: button::State,
    completed_button: button::State,
}

impl Controls {
    fn view(&mut self, tasks: &[Document], current_filter: Filter) -> Row<Message> {
        let Controls {
            all_button,
            active_button,
            completed_button,
        } = self;

        let tasks_left = tasks.iter().filter(|task| !task.completed).count();

        let filter_button = |state, label, filter, current_filter| {
            let label = Text::new(label).size(16);
            let button = Button::new(state, label).style(style::Button::Filter {
                selected: filter == current_filter,
            });

            button.on_press(Message::FilterChanged(filter)).padding(8)
        };

        Row::new()
            .spacing(20)
            .align_items(Align::Center)
            .push(
                Text::new(&format!(
                    "{} {} found",
                    tasks_left,
                    if tasks_left == 1 { "doc" } else { "docs" }
                ))
                .width(Length::Fill)
                .size(16),
            )
            .push(
                Row::new()
                    .width(Length::Shrink)
                    .spacing(10)
                    .push(filter_button(
                        all_button,
                        "All",
                        Filter::All,
                        current_filter,
                    ))
                    .push(filter_button(
                        active_button,
                        "Normalized",
                        Filter::Normalized,
                        current_filter,
                    ))
                    .push(filter_button(
                        completed_button,
                        "Unnormalized",
                        Filter::Unnormalized,
                        current_filter,
                    )),
            )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Filter {
    All,
    Normalized,
    Unnormalized,
}

impl Default for Filter {
    fn default() -> Self {
        Filter::All
    }
}

impl Filter {
    fn matches(&self, doc: &Document) -> bool {
        match self {
            Filter::All => true,
            Filter::Normalized => !doc.completed,
            Filter::Unnormalized => doc.completed,
        }
    }
}

fn loading_message<'a>() -> Element<'a, Message> {
    Container::new(
        Text::new("Loading...")
            .horizontal_alignment(HorizontalAlignment::Center)
            .size(50),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .center_y()
    .into()
}

fn empty_message<'a>(message: &str) -> Element<'a, Message> {
    Container::new(
        Text::new(message)
            .width(Length::Fill)
            .size(25)
            .horizontal_alignment(HorizontalAlignment::Center)
            .color([0.7, 0.7, 0.7]),
    )
    .width(Length::Fill)
    .height(Length::Units(200))
    .center_y()
    .into()
}

// Fonts
const ICONS: Font = Font::External {
    name: "Icons",
    bytes: include_bytes!("../fonts/icons.ttf"),
};

fn icon(unicode: char) -> Text {
    Text::new(&unicode.to_string())
        .font(ICONS)
        .width(Length::Units(20))
        .horizontal_alignment(HorizontalAlignment::Center)
        .size(20)
}

fn edit_icon() -> Text {
    icon('\u{F303}')
}

fn delete_icon() -> Text {
    icon('\u{F1F8}')
}

// Persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SavedState {
    path: String,
    filter: Filter,
    docs: Vec<Document>,
}

#[derive(Debug, Clone)]
enum LoadError {
    FileError,
    FormatError,
}

#[derive(Debug, Clone)]
enum SaveError {
    DirectoryError,
    FileError,
    WriteError,
    FormatError,
}

#[cfg(not(target_arch = "wasm32"))]
impl SavedState {
    fn path() -> std::path::PathBuf {
        let mut path = if let Some(project_dirs) =
            directories_next::ProjectDirs::from("rs", "d6e", "filecabinet")
        {
            project_dirs.data_dir().into()
        } else {
            std::env::current_dir().unwrap_or(std::path::PathBuf::new())
        };

        path.push("filecabinet.json");

        path
    }

    async fn load() -> Result<SavedState, LoadError> {
        use async_std::prelude::*;

        let mut contents = String::new();

        let mut file = async_std::fs::File::open(Self::path())
            .await
            .map_err(|_| LoadError::FileError)?;

        AsyncReadExt::read_to_string(&mut file, &mut contents)
            .await
            .map_err(|_| LoadError::FileError)?;

        serde_json::from_str(&contents).map_err(|_| LoadError::FormatError)
    }

    async fn save(self) -> Result<(), SaveError> {
        use async_std::prelude::*;

        let json = serde_json::to_string_pretty(&self).map_err(|_| SaveError::FormatError)?;

        let path = Self::path();

        if let Some(dir) = path.parent() {
            async_std::fs::create_dir_all(dir)
                .await
                .map_err(|_| SaveError::DirectoryError)?;
        }

        {
            let mut file = async_std::fs::File::create(path)
                .await
                .map_err(|_| SaveError::FileError)?;

            AsyncWriteExt::write_all(&mut file, json.as_bytes())
                .await
                .map_err(|_| SaveError::WriteError)?;
        }

        // This is a simple way to save at most once every couple seconds
        async_std::task::sleep(std::time::Duration::from_secs(2)).await;

        Ok(())
    }
}

#[cfg(target_arch = "wasm32")]
impl SavedState {
    fn storage() -> Option<web_sys::Storage> {
        let window = web_sys::window()?;

        window.local_storage().ok()?
    }

    async fn load() -> Result<SavedState, LoadError> {
        let storage = Self::storage().ok_or(LoadError::FileError)?;

        let contents = storage
            .get_item("state")
            .map_err(|_| LoadError::FileError)?
            .ok_or(LoadError::FileError)?;

        serde_json::from_str(&contents).map_err(|_| LoadError::FormatError)
    }

    async fn save(self) -> Result<(), SaveError> {
        let storage = Self::storage().ok_or(SaveError::FileError)?;

        let json = serde_json::to_string_pretty(&self).map_err(|_| SaveError::FormatError)?;

        storage
            .set_item("state", &json)
            .map_err(|_| SaveError::WriteError)?;

        let _ = wasm_timer::Delay::new(std::time::Duration::from_secs(2)).await;

        Ok(())
    }
}

mod style {
    use iced::{button, Background, Color, Vector};

    pub enum Button {
        Filter { selected: bool },
        Icon,
        Destructive,
    }

    impl button::StyleSheet for Button {
        fn active(&self) -> button::Style {
            match self {
                Button::Filter { selected } => {
                    if *selected {
                        button::Style {
                            background: Some(Background::Color(Color::from_rgb(0.2, 0.2, 0.7))),
                            border_radius: 10.0,
                            text_color: Color::WHITE,
                            ..button::Style::default()
                        }
                    } else {
                        button::Style::default()
                    }
                }
                Button::Icon => button::Style {
                    text_color: Color::from_rgb(0.5, 0.5, 0.5),
                    ..button::Style::default()
                },
                Button::Destructive => button::Style {
                    background: Some(Background::Color(Color::from_rgb(0.8, 0.2, 0.2))),
                    border_radius: 5.0,
                    text_color: Color::WHITE,
                    shadow_offset: Vector::new(1.0, 1.0),
                    ..button::Style::default()
                },
            }
        }

        fn hovered(&self) -> button::Style {
            let active = self.active();

            button::Style {
                text_color: match self {
                    Button::Icon => Color::from_rgb(0.2, 0.2, 0.7),
                    Button::Filter { selected } if !selected => Color::from_rgb(0.2, 0.2, 0.7),
                    _ => active.text_color,
                },
                shadow_offset: active.shadow_offset + Vector::new(0.0, 1.0),
                ..active
            }
        }
    }
}
