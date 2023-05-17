use {
    crate::{arena::Id, buf::EditKind, move_ops, Arena, Buf, CursorSet, Diff, Event, Pos, Text},
    std::{
        cell::{RefCell, RefMut},
        collections::HashSet,
    },
};

#[derive(Debug, Default)]
pub struct State {
    views: Arena<RefCell<View>>,
    models: Arena<Model>,
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create_view(&mut self) -> ViewId {
        let model = self.models.insert(Model {
            view_ids: HashSet::new(),
            buf: Buf::new(include_str!("arena.rs").into()),
        });
        let view_id = self.views.insert(RefCell::new(View {
            model_id: model,
            cursors: CursorSet::new(),
        }));
        self.models[model].view_ids.insert(view_id);
        ViewId(view_id)
    }

    pub fn destroy_view(&mut self, ViewId(view_id): ViewId) {
        let model_id = self.views[view_id].borrow().model_id;
        self.models[model_id].view_ids.remove(&view_id);
        if self.models[model_id].view_ids.is_empty() {
            self.models.remove(model_id);
        }
        self.views.remove(view_id);
    }

    pub fn draw(&self, ViewId(view_id): ViewId, f: impl FnOnce(&Text, &CursorSet)) {
        let model_id = self.views[view_id].borrow().model_id;
        f(
            &self.models[model_id].buf.text(),
            &self.views[view_id].borrow().cursors,
        );
    }

    pub fn handle_event(&mut self, ViewId(view_id): ViewId, event: Event) {
        let model_id = self.views[view_id].borrow().model_id;
        let sibling_views: Vec<_> = self.models[model_id]
            .view_ids
            .iter()
            .filter_map(|&sibling_view_id| {
                if sibling_view_id == view_id {
                    return None;
                }
                Some(self.views[sibling_view_id].borrow_mut())
            })
            .collect();
        HandleEventContext {
            view: self.views[view_id].borrow_mut(),
            sibling_views,
            model: &mut self.models[model_id],
        }
        .handle_event(event);
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ViewId(Id<RefCell<View>>);

#[derive(Debug)]
struct View {
    model_id: Id<Model>,
    cursors: CursorSet,
}

impl View {
    fn update(&mut self, cursors: Option<CursorSet>, diff: &Diff, local: bool) {
        if let Some(cursors) = cursors {
            self.cursors = cursors
        } else {
            self.cursors.apply_diff(diff, local);
        }
    }
}

#[derive(Debug)]
struct Model {
    view_ids: HashSet<Id<RefCell<View>>>,
    buf: Buf,
}

#[derive(Debug)]
struct HandleEventContext<'a> {
    view: RefMut<'a, View>,
    sibling_views: Vec<RefMut<'a, View>>,
    model: &'a mut Model,
}

impl<'a> HandleEventContext<'a> {
    fn handle_event(&mut self, event: Event) {
        use crate::{edit_ops, event::*};

        match event {
            Event::Key(KeyEvent {
                code: KeyCode::Backspace,
                ..
            }) => {
                self.edit(
                    EditKind::Delete,
                    edit_ops::delete(self.model.buf.text(), &self.view.cursors),
                );
            }
            Event::Key(KeyEvent {
                code: KeyCode::Enter,
                ..
            }) => {
                self.edit(
                    EditKind::Insert,
                    edit_ops::insert(
                        self.model.buf.text(),
                        &self.view.cursors,
                        &Text::from(["".to_string(), "".to_string()]),
                    ),
                );
            }
            Event::Key(KeyEvent {
                modifiers: KeyModifiers { shift, .. },
                code: KeyCode::Left,
            }) => {
                self.do_move(shift, |text, pos, _| (move_ops::move_left(text, pos), None));
            }
            Event::Key(KeyEvent {
                modifiers: KeyModifiers { shift, .. },
                code: KeyCode::Up,
            }) => {
                self.do_move(shift, |text, pos, column| {
                    move_ops::move_up(text, pos, column)
                });
            }
            Event::Key(KeyEvent {
                modifiers: KeyModifiers { shift, .. },
                code: KeyCode::Right,
            }) => {
                self.do_move(shift, |text, pos, _| {
                    (move_ops::move_right(text, pos), None)
                });
            }
            Event::Key(KeyEvent {
                modifiers: KeyModifiers { shift, .. },
                code: KeyCode::Down,
            }) => {
                self.do_move(shift, |text, pos, column| {
                    move_ops::move_down(text, pos, column)
                });
            }
            Event::Key(KeyEvent {
                modifiers:
                    KeyModifiers {
                        command: true,
                        shift: false,
                    },
                code: KeyCode::Z,
            }) => {
                self.undo();
            }
            Event::Key(KeyEvent {
                modifiers:
                    KeyModifiers {
                        command: true,
                        shift: true,
                    },
                code: KeyCode::Z,
            }) => {
                self.redo();
            }
            Event::Text(TextEvent { string }) => {
                self.edit(
                    EditKind::Insert,
                    edit_ops::insert(
                        self.model.buf.text(),
                        self.view.cursors.iter(),
                        &Text::from(string),
                    ),
                );
            }
            _ => {}
        }
    }

    fn do_move(
        &mut self,
        select: bool,
        mut f: impl FnMut(&Text, Pos, Option<usize>) -> (Pos, Option<usize>),
    ) {
        self.view.cursors.update_all(|cursor| {
            cursor.do_move(select, |pos, column| f(self.model.buf.text(), pos, column))
        });
        self.model.buf.flush();
    }

    fn edit(&mut self, kind: EditKind, diff: Diff) {
        self.model.buf.edit(kind, &self.view.cursors, diff.clone());
        self.update_views(None, &diff);
    }

    fn undo(&mut self) {
        if let Some((cursors, diff)) = self.model.buf.undo() {
            self.update_views(Some(cursors), &diff);
        }
    }

    fn redo(&mut self) {
        if let Some((cursors, diff)) = self.model.buf.redo() {
            self.update_views(Some(cursors), &diff);
        }
    }

    fn update_views(&mut self, cursors: Option<CursorSet>, diff: &Diff) {
        self.view.update(cursors, diff, true);
        for sibling_view in &mut self.sibling_views {
            sibling_view.update(None, diff, false);
        }
    }
}