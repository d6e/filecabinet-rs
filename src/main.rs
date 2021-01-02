#[macro_use]
extern crate lazy_static;
use crate::utils::{parse_date, to_document};
use chrono::{DateTime, Utc};
use iced::futures::{AsyncReadExt, AsyncWriteExt};
use iced::widget::pane_grid::{Content, Pane};
use iced::{
    button, pane_grid, scrollable, text_input, Align, Application, Button, Checkbox, Column,
    Command, Container, Element, Font, HorizontalAlignment, Image, Length, PaneGrid, Row,
    Scrollable, Settings, Text, TextInput,
};
use serde::export::Formatter;
use serde::{Deserialize, Serialize};
use std::borrow::Borrow;
use std::collections::linked_list::Iter;
use std::fmt::Debug;
use std::ops::Deref;
use std::path::Path;
use std::path::PathBuf;
use std::{env, fs};

mod utils;

pub fn main() -> iced::Result {
    FileCabinet::run(Settings::default())
}

enum FileCabinet {
    Loading,
    Loaded(State),
}

struct State {
    panes: pane_grid::State<Box<dyn PaneContent>>,
    doc_pane: Option<Pane>,
    preview_pane: Option<Pane>,
    preview_image: String,
    dirty: bool,
    saving: bool,
}

impl Default for State {
    fn default() -> Self {
        let (pane_state, pane) =
            pane_grid::State::new(Box::new(DocPane::default()) as Box<dyn PaneContent>);
        State {
            panes: pane_state,
            doc_pane: Some(pane),
            preview_pane: None,
            preview_image: "".to_string(),
            dirty: false,
            saving: false,
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    Loaded(Result<SavedState, LoadError>),
    Saved(Result<(), SaveError>),
    PathChanged(String),
    FilterChanged(Filter),
    TaskMessage(usize, TaskMessage),
    ClosePreviewPane(Pane),
}

#[derive(Debug, Default)]
struct DocPane {
    scroll: scrollable::State,
    path: text_input::State,
    path_value: String,
    filter: Filter,
    controls: Controls,
    docs: Vec<Document>,
}

#[derive(Debug, Default)]
struct ImagePane {
    preview_image: String,
    close_button: button::State,
}

trait PaneContent {
    fn update(&mut self, message: Message);
    fn view(&mut self, pane: Pane) -> Element<Message>;
}

impl PaneContent for ImagePane {
    fn update(&mut self, message: Message) {}
    fn view(&mut self, pane: Pane) -> Element<'_, Message> {
        println!(
            "event=preview_pane_opened image=\"{}\"",
            &self.preview_image
        );
        Column::new()
            .push(
                Row::new().push(
                    Button::new(&mut self.close_button, Text::new("X"))
                        .padding(10)
                        .style(style::Button::Destructive)
                        .on_press(Message::ClosePreviewPane(pane)),
                ),
            )
            .push(Text::new(&self.preview_image))
            .push(Image::new(&self.preview_image))
            .align_items(Align::End)
            .width(Length::Fill)
            .into()
    }
}

impl PaneContent for DocPane {
    fn update(&mut self, message: Message) {
        match message {
            Message::Loaded(_) => {}
            Message::Saved(_) => {}
            Message::PathChanged(value) => {
                self.path_value = value;
                let dir_path = Path::new(&self.path_value).to_path_buf();
                self.docs = utils::list_files(&dir_path)
                    .iter()
                    .map(|path| {
                        let mut full_path = dir_path.clone();
                        full_path.push(path);
                        // Document {
                        //     path: full_path
                        //         .to_str()
                        //         .expect(&format!("can't convert '{}' to a str", path))
                        //         .to_string(),
                        //     completed: false,
                        //     state: Default::default(),
                        // }
                        Document::new(full_path.to_str().unwrap().to_string())
                    })
                    .collect();
            }
            Message::FilterChanged(filter) => {
                self.filter = filter;
            }
            Message::TaskMessage(i, TaskMessage::Delete) => {
                self.docs.remove(i);
            }
            Message::TaskMessage(i, task_message) => {
                if let Some(doc) = self.docs.get_mut(i) {
                    doc.update(task_message);
                }
            }
            _ => {}
        }
    }

    fn view(&mut self, pane: Pane) -> Element<Message> {
        let DocPane {
            path,
            path_value,
            docs,
            filter,
            controls,
            ..
        } = self;
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
        .size(16);

        let controls = controls.view(&docs, *filter);
        let filtered_tasks = docs.iter().filter(|doc| filter.matches(doc));

        let docs: Element<_> = if filtered_tasks.count() > 0 {
            docs.iter_mut()
                .enumerate()
                .filter(|(_, doc)| filter.matches(doc))
                .fold(Column::new().spacing(20), |column, (i, doc)| {
                    column.push(
                        doc.view(&pane)
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

        Scrollable::new(&mut self.scroll)
            .padding(40)
            .push(Container::new(content).width(Length::Fill).center_x())
            .into()
    }
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
                    Message::Loaded(Ok(_state)) => {
                        *self = FileCabinet::Loaded(State::default());
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
                    Message::PathChanged(ref value) => {
                        for (pane, boxed_content) in state.panes.iter_mut() {
                            boxed_content.update(message.clone());
                        }
                    }
                    // Message::CreateTask => {
                    //     if !state.input_value.is_empty() {
                    //         state.docs.push(Document::new(state.input_value.clone()));
                    //         state.input_value.clear();
                    //     }
                    // }
                    Message::FilterChanged(filter) => {
                        for (pane, boxed_content) in state.panes.iter_mut() {
                            boxed_content.update(message.clone());
                        }
                    }
                    Message::ClosePreviewPane(pane) => {
                        state.panes.close(&pane);
                        state.preview_pane = Default::default();
                    }
                    Message::TaskMessage(_, TaskMessage::OpenPreviewPane(path, _)) => {
                        if let Some(doc_pane) = &state.doc_pane {
                            match state.preview_pane {
                                None => {
                                    println!("Preview pane closed, opening for the first time");
                                    // If the preview pane isn't open, open it,
                                    if let Some((preview_pane, split)) = state.panes.split(
                                        pane_grid::Axis::Vertical,
                                        doc_pane,
                                        Box::new(ImagePane {
                                            preview_image: path.clone(),
                                            close_button: Default::default(),
                                        }),
                                    ) {
                                        // then save the preview pane.
                                        state.preview_pane = Some(preview_pane);
                                        state.preview_image = path;
                                    }
                                }
                                Some(preview_pane) => {
                                    println!("Preview pane open, closing and reopening new one...");
                                    if state.preview_image != path {
                                        println!("Preview pane image is the same path, refusing to open.");
                                        // If the preview pane is open, close it,
                                        state.panes.close(&preview_pane);
                                        // then open the new one.
                                        if let Some((pane, _)) = state.panes.split(
                                            pane_grid::Axis::Vertical,
                                            doc_pane,
                                            Box::new(ImagePane {
                                                preview_image: path.clone(),
                                                close_button: Default::default(),
                                            }),
                                        ) {
                                            // Update the preview pane with state.
                                            state.preview_pane = Some(pane);
                                            state.preview_image = path;
                                        } else {
                                            // If fails, unset the preview pane.
                                            state.preview_pane = None;
                                            state.preview_image = String::new();
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Message::TaskMessage(_, TaskMessage::Delete) => {
                        for (pane, boxed_content) in state.panes.iter_mut() {
                            boxed_content.update(message.clone());
                        }
                    }
                    Message::TaskMessage(i, ref task_message) => {
                        for (pane, boxed_content) in state.panes.iter_mut() {
                            boxed_content.update(message.clone());
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

                    // TODO: migrate
                    // Command::perform(
                    //     SavedState {
                    //         path: state.path_value.clone(),
                    //         filter: state.filter,
                    //         docs: state.docs.clone(),
                    //     }
                    //     .save(),
                    //     Message::Saved,
                    // )
                    Command::none()
                } else {
                    Command::none()
                }
            }
        }
    }

    fn view(&mut self) -> Element<Message> {
        match self {
            FileCabinet::Loading => loading_message(),
            FileCabinet::Loaded(state) => {
                // let grid: PaneGrid<Message> = PaneGrid::new(&mut pane_state.0, |pane, state| {
                //     pane_grid::Content::new(match state {
                //         ImagePaneState::DocPane => Container::new(
                //             Scrollable::new(scroll)
                //                 .padding(40)
                //                 .push(Container::new(content).width(Length::Fill).center_x()),
                //         ),
                //         ImagePaneState::ImagePane => Container::new(Text::new("image pane")),
                //     })
                // });

                let pane_grid = PaneGrid::new(&mut state.panes, |pane, content| {
                    // let is_focused = focus == Some(pane);

                    // .title_bar(title_bar)
                    // .style(style::Pane { is_focused })
                    let c: Element<Message> = Container::new(content.view(pane)).into();
                    pane_grid::Content::new(c)
                })
                .width(Length::Fill)
                .height(Length::Fill)
                .spacing(10);
                // .on_click(Message::Clicked)
                // .on_drag(Message::Dragged)
                // .on_resize(10, Message::Resized);

                Container::new(pane_grid)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding(10)
                    .into()
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Document {
    path: String,
    date: String,
    institution: String,
    title: String,
    page: String,
    completed: bool,  // TODO remove
    encrypt_it: bool, // TODO remove

    #[serde(skip)]
    state: TaskState,
}

#[derive(Debug, Clone)]
pub enum TaskState {
    Idle {
        edit_button: button::State,
        preview_button: button::State,
    },
    Editing {
        date_input: text_input::State,
        institution_input: text_input::State,
        title_input: text_input::State,
        page_input: text_input::State,
        delete_button: button::State,
        cancel_button: button::State,
        submit_button: button::State,
    },
}

impl Default for TaskState {
    fn default() -> Self {
        TaskState::Idle {
            edit_button: button::State::new(),
            preview_button: button::State::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum TaskMessage {
    Completed(bool),
    Edit,
    DateEdited(String),
    InstitutionEdited(String),
    TitleEdited(String),
    PageEdited(String),
    FinishEdition,
    Delete,
    Cancel,
    OpenPreviewPane(String, Pane),
}

impl Document {
    fn new(path: String) -> Self {
        let options = to_document(&path);
        let now: DateTime<Utc> = Utc::now();
        Document {
            path,
            date: options.date.unwrap_or(now.format("%Y-%m-%d").to_string()),
            institution: options.institution.unwrap_or(String::new()),
            title: options.name.unwrap_or(String::new()),
            page: options.page.unwrap_or(String::from("1")).parse().unwrap(),
            completed: false,
            encrypt_it: false,
            state: TaskState::default(),
        }
    }

    fn update(&mut self, message: TaskMessage) {
        match message {
            TaskMessage::Completed(completed) => {
                self.completed = completed;
            }
            TaskMessage::Edit => {
                self.state = TaskState::Editing {
                    date_input: Default::default(),
                    institution_input: Default::default(),
                    title_input: Default::default(),
                    page_input: Default::default(),
                    delete_button: button::State::new(),
                    cancel_button: button::State::new(),
                    submit_button: button::State::new(),
                };
            }
            TaskMessage::Cancel => {
                self.state = TaskState::Idle {
                    edit_button: button::State::new(),
                    preview_button: button::State::new(),
                }
            }
            TaskMessage::FinishEdition => {
                let basename = Path::new(&self.path).parent();
                let extension = utils::extension(&self.path);
                let filename = format!(
                    "{}_{}_{}_{}.{}",
                    &self.date, &self.institution, &self.title, &self.page, extension
                );
                let new_path: String = basename
                    .and_then(|p| {
                        // basename is a valid directory, add it and return.
                        let mut pb = p.to_path_buf();
                        pb.push(&filename);
                        pb.to_str().map(|s| s.to_string())
                    })
                    .unwrap_or(filename);
                fs::rename(&self.path, &new_path).unwrap(); // Rename file
                println!(
                    "event=\"Rename\" old=\"{}\" new=\"{}\"",
                    &self.path, &new_path
                );
                self.path = new_path.to_string(); // Update UI doc path.
                self.state = TaskState::Idle {
                    edit_button: button::State::new(),
                    preview_button: button::State::new(),
                }
            }
            TaskMessage::Delete => {}
            TaskMessage::DateEdited(s) => {
                self.date = s;
            }
            TaskMessage::InstitutionEdited(s) => {
                self.institution = s;
            }
            TaskMessage::PageEdited(s) => {
                self.page = s;
            }
            TaskMessage::TitleEdited(s) => {
                self.title = s;
            }
            _ => {}
        }
    }

    fn view(&mut self, pane: &Pane) -> Element<TaskMessage> {
        match &mut self.state {
            TaskState::Idle {
                preview_button,
                edit_button,
            } => {
                let checkbox = Checkbox::new(self.completed, "", TaskMessage::Completed);
                let preview = Button::new(preview_button, Text::new(&self.path))
                    .on_press(TaskMessage::OpenPreviewPane(self.path.clone(), *pane))
                    .width(Length::Fill);
                Row::new()
                    .spacing(20)
                    .align_items(Align::Center)
                    .push(checkbox)
                    .push(preview)
                    .push(
                        Button::new(edit_button, edit_icon())
                            .on_press(TaskMessage::Edit)
                            .padding(10)
                            .style(style::Button::Icon),
                    )
                    .into()
            }
            TaskState::Editing {
                date_input,
                institution_input,
                title_input,
                page_input,
                delete_button,
                cancel_button,
                submit_button,
            } => {
                Column::new()
                    .spacing(10)
                    .push(Text::new(&self.path))
                    .push(
                        TextInput::new(date_input, "Date", &self.date, TaskMessage::DateEdited)
                            .on_submit(TaskMessage::FinishEdition)
                            .padding(10),
                    )
                    .push(
                        TextInput::new(
                            institution_input,
                            "Institution",
                            &self.institution,
                            TaskMessage::InstitutionEdited,
                        )
                        .on_submit(TaskMessage::FinishEdition)
                        .padding(10),
                    )
                    .push(
                        TextInput::new(title_input, "Title", &self.title, TaskMessage::TitleEdited)
                            .on_submit(TaskMessage::FinishEdition)
                            .padding(10),
                    )
                    .push(
                        TextInput::new(page_input, "Page", &self.page, TaskMessage::PageEdited)
                            .on_submit(TaskMessage::FinishEdition)
                            .padding(10),
                    )
                    .push(
                        Row::new()
                            .spacing(10)
                            .push(
                                Button::new(
                                    submit_button,
                                    Row::new().spacing(10).push(Text::new("Submit")),
                                )
                                .on_press(TaskMessage::FinishEdition)
                                .padding(10)
                                .style(style::Button::Update),
                            )
                            // Delete Button
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
                            // Cancel Button
                            .push(
                                Button::new(
                                    cancel_button,
                                    Row::new().spacing(10).push(Text::new("Cancel")),
                                )
                                .on_press(TaskMessage::Cancel)
                                .padding(10)
                                .style(style::Button::Cancel),
                            ),
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
        Update,
        Cancel,
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
                Button::Update => button::Style {
                    background: Some(Background::Color(Color::from_rgb(0.467, 0.867, 0.467))),
                    border_radius: 5.0,
                    text_color: Color::WHITE,
                    shadow_offset: Vector::new(1.0, 1.0),
                    ..button::Style::default()
                },
                Button::Cancel => button::Style {
                    background: Some(Background::Color(Color::from_rgb(0.2, 0.2, 0.2))),
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
