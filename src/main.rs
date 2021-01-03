#[macro_use]
extern crate lazy_static;
use crate::utils::OptDoc;
use chrono::{DateTime, Utc};
use iced::futures::{AsyncReadExt, AsyncWriteExt};
use iced::widget::pane_grid::Pane;
use iced::{
    button, pane_grid, scrollable, text_input, Align, Application, Button, Checkbox, Column,
    Command, Container, Element, Font, HorizontalAlignment, Image, Length, PaneGrid, Row,
    Scrollable, Settings, Text, TextInput,
};

use serde::{Deserialize, Serialize};

use std::fmt::Debug;

use std::path::Path;

use std::fs;

mod utils;

pub fn main() -> iced::Result {
    FileCabinet::run(Settings::default())
}

enum FileCabinet {
    Loading,
    Loaded(State),
}

struct State {
    refresh_state: button::State,
    target_dir_state: text_input::State,
    target_dir: String,
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
            refresh_state: Default::default(),
            target_dir_state: Default::default(),
            target_dir: "".to_string(),
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
    RefreshTargetDir(String),
    Loaded(Result<SavedState, LoadError>),
    Saved(Result<(), SaveError>),
    PathChanged(String),
    FilterChanged(Filter),
    DocMessage(usize, DocMessage),
    ClosePreviewPane(Pane),
    Dragged(pane_grid::DragEvent),
    Resized(pane_grid::ResizeEvent),
}

#[derive(Debug, Default)]
struct DocPane {
    scroll: scrollable::State,
    filter: Filter,
    controls: Controls,
    docs: Vec<Document>,
}

#[derive(Debug, Default)]
struct PreviewPane {
    preview_image_path: String,
    close_button: button::State,
    scroll_state: scrollable::State,
}

trait PaneContent {
    fn update(&mut self, message: Message);
    fn view(&mut self, pane: Pane) -> Element<Message>;
}

impl PaneContent for PreviewPane {
    fn update(&mut self, _message: Message) {}
    fn view(&mut self, pane: Pane) -> Element<'_, Message> {
        println!(
            "event=preview_pane_opened image=\"{}\"",
            &self.preview_image_path
        );
        Column::new()
            .push(
                Button::new(&mut self.close_button, Text::new("X").size(10))
                    .padding(10)
                    .style(style::Button::Destructive)
                    .on_press(Message::ClosePreviewPane(pane)),
            )
            .push(Text::new(&self.preview_image_path))
            .push(
                Scrollable::new(&mut self.scroll_state)
                    .push(
                        Row::new()
                            .push(Image::new(&self.preview_image_path))
                            .align_items(Align::Center)
                            .width(Length::Fill),
                    )
                    .width(Length::Fill),
            )
            .padding(10)
            .into()
    }
}

impl PaneContent for DocPane {
    fn update(&mut self, message: Message) {
        match message {
            Message::Loaded(_) => {}
            Message::Saved(_) => {}
            Message::RefreshTargetDir(path) => self.docs = utils::read_docs(&path),
            Message::PathChanged(path) => self.docs = utils::read_docs(&path),
            Message::FilterChanged(filter) => {
                self.filter = filter;
            }
            Message::DocMessage(i, DocMessage::ConfirmDelete) => {
                if let Some(doc) = self.docs.get_mut(i) {
                    doc.update(DocMessage::ConfirmDelete);
                    fs::remove_file(doc.clone().path).unwrap();
                }
                self.docs.remove(i);
            }
            Message::DocMessage(i, doc_message) => {
                if let Some(doc) = self.docs.get_mut(i) {
                    doc.update(doc_message);
                }
            }
            _ => {}
        }
    }

    fn view(&mut self, pane: Pane) -> Element<Message> {
        let DocPane {
            docs,
            filter,
            controls,
            ..
        } = self;

        let controls = controls.view(&docs, *filter);
        let filtered_docs = docs.iter().filter(|doc| filter.matches(doc));

        let docs: Element<_> = if filtered_docs.count() > 0 {
            docs.iter_mut()
                .enumerate()
                .filter(|(_, doc)| filter.matches(doc))
                .fold(Column::new().spacing(20), |column, (i, doc)| {
                    column.push(
                        doc.view(&pane)
                            .map(move |message| Message::DocMessage(i, message)),
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
                    Message::Loaded(Ok(saved_state)) => {
                        // Create the panes so that the documents are loaded on launch.
                        let (mut pane_state, pane) = pane_grid::State::new(Box::new(
                            DocPane::default(),
                        )
                            as Box<dyn PaneContent>);
                        // Pass the path to each doc_pane doc so it can render.
                        for (_pane, boxed_content) in pane_state.iter_mut() {
                            boxed_content
                                .update(Message::PathChanged(saved_state.target_dir.clone()));
                        }
                        *self = FileCabinet::Loaded(State {
                            target_dir: saved_state.target_dir,
                            panes: pane_state,
                            doc_pane: Some(pane),
                            ..Default::default()
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
                    Message::RefreshTargetDir(_) => {
                        for (_pane, boxed_content) in state.panes.iter_mut() {
                            boxed_content.update(message.clone());
                        }
                    }
                    Message::PathChanged(ref value) => {
                        state.target_dir = value.clone();
                        for (_pane, boxed_content) in state.panes.iter_mut() {
                            boxed_content.update(message.clone());
                        }
                    }
                    Message::FilterChanged(_filter) => {
                        for (_pane, boxed_content) in state.panes.iter_mut() {
                            boxed_content.update(message.clone());
                        }
                    }
                    Message::ClosePreviewPane(pane) => {
                        state.panes.close(&pane);
                        state.preview_pane = Default::default();
                    }
                    Message::DocMessage(_, DocMessage::OpenPreviewPane(path, _)) => {
                        if let Some(doc_pane) = &state.doc_pane {
                            match state.preview_pane {
                                None => {
                                    println!("Preview pane closed, opening for the first time");
                                    // If the preview pane isn't open, open it,
                                    if let Some((preview_pane, _split)) = state.panes.split(
                                        pane_grid::Axis::Vertical,
                                        doc_pane,
                                        Box::new(PreviewPane {
                                            preview_image_path: path.clone(),
                                            ..Default::default()
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
                                            Box::new(PreviewPane {
                                                preview_image_path: path.clone(),
                                                ..Default::default()
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
                    Message::DocMessage(_, DocMessage::Delete) => {
                        for (_pane, boxed_content) in state.panes.iter_mut() {
                            boxed_content.update(message.clone());
                        }
                    }
                    Message::DocMessage(_, ref _doc_message) => {
                        for (_pane, boxed_content) in state.panes.iter_mut() {
                            boxed_content.update(message.clone());
                        }
                    }
                    Message::Resized(pane_grid::ResizeEvent { split, ratio }) => {
                        state.panes.resize(&split, ratio);
                    }
                    Message::Dragged(pane_grid::DragEvent::Dropped { pane, target }) => {
                        state.panes.swap(&pane, &target);
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
                            target_dir: state.target_dir.clone(),
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
            FileCabinet::Loaded(state) => {
                let pane_grid = PaneGrid::new(&mut state.panes, |pane, content| {
                    pane_grid::Content::new(content.view(pane))
                        .style(style::Pane {})
                        .into()
                })
                .on_drag(Message::Dragged)
                .on_resize(10, Message::Resized)
                .spacing(10);

                let title = Text::new("filecabinet")
                    .width(Length::Fill)
                    .size(80)
                    .color([0.5, 0.5, 0.5])
                    .horizontal_alignment(HorizontalAlignment::Center);

                let target_dir_input = TextInput::new(
                    &mut state.target_dir_state,
                    "Specify path to documents",
                    &*state.target_dir,
                    Message::PathChanged,
                )
                .padding(10)
                .size(16);

                Container::new(
                    Column::new()
                        .push(title)
                        .push(
                            Row::new().spacing(10).push(target_dir_input).push(
                                Button::new(
                                    &mut state.refresh_state,
                                    Text::new("refresh").size(16),
                                )
                                .style(style::Button::Refresh)
                                .padding(10)
                                .on_press(Message::RefreshTargetDir(state.target_dir.clone())),
                            ),
                        )
                        .push(pane_grid)
                        .spacing(10),
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(10)
                .into()
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    path: String,
    filename: String,
    date: String,
    institution: String,
    title: String,
    page: String,
    extension: String,
    selected: bool,
    encrypted: bool,
    show_delete_confirmation: bool,
    #[serde(skip)]
    state: DocState,
}

#[derive(Debug, Clone)]
pub enum DocState {
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
        confirm_yes_button: button::State,
        confirm_no_button: button::State,
    },
}

impl Default for DocState {
    fn default() -> Self {
        DocState::Idle {
            edit_button: button::State::new(),
            preview_button: button::State::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum DocMessage {
    Selected(bool),
    Edit,
    DateEdited(String),
    InstitutionEdited(String),
    TitleEdited(String),
    PageEdited(String),
    FinishEdition,
    Delete,
    ConfirmDelete,
    ConfirmNo,
    Cancel,
    OpenPreviewPane(String, Pane),
}

impl Document {
    fn new(path: String) -> Self {
        let options = OptDoc::new(&path);
        let now: DateTime<Utc> = Utc::now();
        let tmp = &path.clone();
        let _path = Path::new(tmp);
        let file_stem = _path.file_stem().unwrap().to_str().unwrap();
        let extension = utils::extension(_path);
        Document {
            path,
            filename: format!("{}.{}", file_stem, extension),
            date: options.date.unwrap_or(now.format("%Y-%m-%d").to_string()),
            institution: options.institution.unwrap_or(String::new()),
            title: options.name.unwrap_or(String::new()),
            page: options.page.unwrap_or(String::from("1")).parse().unwrap(),
            extension: extension.to_string(),
            selected: false,
            encrypted: false,
            show_delete_confirmation: false,
            state: DocState::default(),
        }
    }

    fn update(&mut self, message: DocMessage) {
        match message {
            DocMessage::Selected(selected) => {
                self.selected = selected;
            }
            DocMessage::Edit => {
                self.state = DocState::Editing {
                    date_input: Default::default(),
                    institution_input: Default::default(),
                    title_input: Default::default(),
                    page_input: Default::default(),
                    delete_button: Default::default(),
                    cancel_button: Default::default(),
                    submit_button: Default::default(),
                    confirm_yes_button: Default::default(),
                    confirm_no_button: Default::default(),
                };
            }
            DocMessage::Cancel => {
                self.state = DocState::Idle {
                    edit_button: button::State::new(),
                    preview_button: button::State::new(),
                }
            }
            DocMessage::FinishEdition => {
                let basename = Path::new(&self.path).parent();
                let filename = format!(
                    "{}_{}_{}_{}.{}",
                    &self.date, &self.institution, &self.title, &self.page, &self.extension
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
                self.state = DocState::Idle {
                    edit_button: button::State::new(),
                    preview_button: button::State::new(),
                }
            }
            DocMessage::Delete => {
                if self.show_delete_confirmation {
                    self.show_delete_confirmation = false;
                } else {
                    self.show_delete_confirmation = true;
                }
            }
            DocMessage::ConfirmNo => {
                self.show_delete_confirmation = false;
            }
            DocMessage::DateEdited(s) => {
                self.date = s;
            }
            DocMessage::InstitutionEdited(s) => {
                self.institution = s;
            }
            DocMessage::PageEdited(s) => {
                self.page = s;
            }
            DocMessage::TitleEdited(s) => {
                self.title = s;
            }
            _ => {}
        }
    }

    fn view(&mut self, pane: &Pane) -> Element<DocMessage> {
        match &mut self.state {
            DocState::Idle {
                preview_button,
                edit_button,
            } => {
                let checkbox = Checkbox::new(self.selected, "", DocMessage::Selected);
                let preview = Button::new(preview_button, Text::new(&self.filename))
                    .on_press(DocMessage::OpenPreviewPane(self.path.clone(), *pane))
                    .style(style::Button::Doc)
                    .width(Length::Fill);
                Row::new()
                    .spacing(20)
                    .align_items(Align::Center)
                    .push(checkbox)
                    .push(preview)
                    .push(
                        Button::new(edit_button, edit_icon())
                            .on_press(DocMessage::Edit)
                            .padding(10)
                            .style(style::Button::Icon),
                    )
                    .into()
            }
            DocState::Editing {
                date_input,
                institution_input,
                title_input,
                page_input,
                delete_button,
                cancel_button,
                submit_button,
                confirm_no_button,
                confirm_yes_button,
            } => {
                Column::new()
                    .spacing(10)
                    .push(Text::new(&self.filename))
                    .push(
                        TextInput::new(date_input, "Date", &self.date, DocMessage::DateEdited)
                            .on_submit(DocMessage::FinishEdition)
                            .padding(10),
                    )
                    .push(
                        TextInput::new(
                            institution_input,
                            "Institution",
                            &self.institution,
                            DocMessage::InstitutionEdited,
                        )
                        .on_submit(DocMessage::FinishEdition)
                        .padding(10),
                    )
                    .push(
                        TextInput::new(title_input, "Title", &self.title, DocMessage::TitleEdited)
                            .on_submit(DocMessage::FinishEdition)
                            .padding(10),
                    )
                    .push(
                        TextInput::new(page_input, "Page", &self.page, DocMessage::PageEdited)
                            .on_submit(DocMessage::FinishEdition)
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
                                .on_press(DocMessage::FinishEdition)
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
                                .on_press(DocMessage::Delete)
                                .padding(10)
                                .style(style::Button::Destructive),
                            )
                            .push(if self.show_delete_confirmation {
                                Row::new()
                                    .push(
                                        Button::new(confirm_no_button, Text::new("No!"))
                                            .on_press(DocMessage::ConfirmNo)
                                            .style(style::Button::Cancel),
                                    )
                                    .push(
                                        Button::new(confirm_yes_button, Text::new("Yes?"))
                                            .on_press(DocMessage::ConfirmDelete)
                                            .style(style::Button::Destructive),
                                    )
                                    .padding(10)
                                    .spacing(10)
                                    .align_items(Align::Center)
                            } else {
                                Row::new()
                            })
                            // Cancel Button
                            .push(
                                Button::new(
                                    cancel_button,
                                    Row::new().spacing(10).push(Text::new("Cancel")),
                                )
                                .on_press(DocMessage::Cancel)
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
    fn view(&mut self, docs: &[Document], current_filter: Filter) -> Row<Message> {
        let Controls {
            all_button,
            active_button,
            completed_button,
        } = self;

        let filter_button = |state, label, filter: Filter, current_filter: Filter| {
            let label = Text::new(format!(
                "{}: {}",
                label,
                docs.iter().filter(|d| filter.matches(d)).count()
            ))
            .size(16);
            let button = Button::new(state, label).style(style::Button::Filter {
                selected: filter == current_filter,
            });

            button.on_press(Message::FilterChanged(filter)).padding(8)
        };

        Row::new().spacing(20).align_items(Align::Center).push(
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
            Filter::Normalized => utils::is_normalized(&doc.path),
            Filter::Unnormalized => !utils::is_normalized(&doc.path),
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
    target_dir: String,
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

    use iced::{button, container, Background, Color, Vector};

    pub struct Pane {}

    impl container::StyleSheet for Pane {
        fn style(&self) -> container::Style {
            container::Style {
                background: Some(Background::Color(Color::from_rgb(
                    0xf8 as f32 / 255.0,
                    0xed as f32 / 255.0,
                    0xeb as f32 / 255.0,
                ))),
                border_width: 1.0,
                border_radius: 5.0,
                border_color: Color::from([0.7, 0.7, 0.7]), // light grey
                ..Default::default()
            }
        }
    }

    pub enum Button {
        Filter { selected: bool },
        Icon,
        Destructive,
        Update,
        Cancel,
        Doc,
        Refresh,
    }

    impl button::StyleSheet for Button {
        fn active(&self) -> button::Style {
            match self {
                Button::Doc => button::Style {
                    text_color: Color::WHITE,
                    background: Some(Background::Color(Color::from_rgb(
                        0xe5 as f32 / 255.0,
                        0x6b as f32 / 255.0,
                        0x6f as f32 / 255.0,
                    ))), // dark pink
                    border_radius: 5.0,
                    ..Default::default()
                },
                Button::Filter { selected } => {
                    if *selected {
                        button::Style {
                            background: Some(Background::Color(Color::from_rgb(0.2, 0.2, 0.7))),
                            border_radius: 10.0,
                            text_color: Color::WHITE,
                            ..button::Style::default()
                        }
                    } else {
                        button::Style {
                            border_radius: 10.0,
                            ..button::Style::default()
                        }
                    }
                }
                Button::Icon => button::Style {
                    text_color: Color::from_rgb(0.5, 0.5, 0.5),
                    border_radius: 10.0,
                    border_color: Color::from_rgb(0.5, 0.5, 0.5),
                    ..button::Style::default()
                },
                Button::Refresh => button::Style {
                    background: Some(Background::Color(Color::from_rgb(
                        0x24 as f32 / 255.0,
                        0x7b as f32 / 255.0,
                        0xa0 as f32 / 255.0,
                    ))),
                    border_radius: 5.0,
                    text_color: Color::WHITE,
                    shadow_offset: Vector::new(1.0, 1.0),
                    ..button::Style::default()
                },
                Button::Destructive => button::Style {
                    background: Some(Background::Color(Color::from_rgb(
                        0xef as f32 / 255.0,
                        0x47 as f32 / 255.0,
                        0x6f as f32 / 255.0,
                    ))),
                    border_radius: 5.0,
                    text_color: Color::WHITE,
                    shadow_offset: Vector::new(1.0, 1.0),
                    ..button::Style::default()
                },
                Button::Update => button::Style {
                    background: Some(Background::Color(Color::from_rgb(
                        0x06 as f32 / 255.0,
                        0xd6 as f32 / 255.0,
                        0xa0 as f32 / 255.0,
                    ))),
                    border_radius: 5.0,
                    text_color: Color::WHITE,
                    shadow_offset: Vector::new(1.0, 1.0),
                    ..button::Style::default()
                },
                Button::Cancel => button::Style {
                    background: Some(Background::Color(Color::from_rgb(
                        0xff as f32 / 255.0,
                        0xd1 as f32 / 255.0,
                        0x66 as f32 / 255.0,
                    ))),
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
                border_width: 2.0,
                border_radius: self.active().border_radius,
                border_color: Color::from_rgb(0.5, 0.5, 0.5),
                // shadow_offset: active.shadow_offset + Vector::new(0.0, 2.0),
                ..active
            }
        }
    }
}
