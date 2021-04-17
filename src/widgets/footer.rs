
use tuix::*;

use crate::SAMPLES_PER_PIXEL;
use super::app_event::{*, self};
use super::icons::*;

pub struct Footer {
    cursor_label: Entity,
    select_label: Entity,

    tooltip: Entity,
    file_name_label: Entity,

    zoom_levels_dropdown: Entity,

    zoom_0: Entity,
    zoom_1: Entity,
    zoom_2: Entity,
    zoom_3: Entity,
    zoom_4: Entity,
    zoom_5: Entity,
    zoom_6: Entity,
    zoom_7: Entity,
    zoom_8: Entity,
}

impl Footer {
    pub fn new() -> Self {
        Self {
            cursor_label: Entity::default(),
            select_label: Entity::default(),
            tooltip: Entity::default(),
            file_name_label: Entity::default(),
            zoom_levels_dropdown: Entity::default(),

            zoom_0: Entity::default(),
            zoom_1: Entity::default(),
            zoom_2: Entity::default(),
            zoom_3: Entity::default(),
            zoom_4: Entity::default(),
            zoom_5: Entity::default(),
            zoom_6: Entity::default(),
            zoom_7: Entity::default(),
            zoom_8: Entity::default(),
        }
    }
}

impl BuildHandler for Footer {
    type Ret = Entity;
    fn on_build(&mut self, state: &mut State, entity: Entity) -> Self::Ret {

        self.tooltip = Label::new("").build(state, entity, |builder| {
            builder
            .class("info")
            .set_margin(Length::Pixels(10.0))
        });

        self.file_name_label = Label::new("").build(state, entity, |builder| {
            builder.class("info").set_margin(Length::Pixels(10.0)).set_flex_grow(1.0).set_text_justify(Justify::Center)
        });

        // self.cursor_label = Label::new("Cursor:  00''00'00.0").build(state, entity, |builder| {
        //     builder.class("info").set_margin(Length::Pixels(20.0))
        // });

        // self.select_label =
        //     Label::new("Select End:  00''00'00.0").build(state, entity, |builder| {
        //         builder.class("info").set_margin(Length::Pixels(20.0))
        //     });

        Checkbox::new(false)
            .on_unchecked(Event::new(AppEvent::FollowPlayhead(false)).target(entity))
            .on_checked(Event::new(AppEvent::FollowPlayhead(true)).target(entity))
            .with_icon_checked(ICON_LOCK)
            .with_icon_unchecked(ICON_LOCK_OPEN)
            .build(state, entity, |builder| {
                builder.set_font("Icons").class("snap")
            });

        // Zoom Controls
        let zoom_controls =
            Element::new().build(state, entity, |builder| builder.class("zoom_controls"));

        Button::with_label(ICON_MINUS)
            .on_press(Event::new(AppEvent::DecZoom).target(entity))
            .build(state, zoom_controls, |builder| {
                builder.set_font("Icons").class("zoom").class("first")
            });

        let (zoom_levels_dropdown, _, zoom_levels_dropdown_container) =
            ZoomDropdown::new().build(state, zoom_controls, |builder| builder.class("zoom"));

        self.zoom_levels_dropdown = zoom_levels_dropdown;

        let zoom_levels_list =
            RadioList::new().build(state, zoom_levels_dropdown_container, |builder| {
                builder
                    .class("checklist")
                    .set_flex_direction(FlexDirection::Column)
            });

        self.zoom_8 = RadioButton::new()
            .on_checked(Event::new(AppEvent::SetZoomLevel(8, ZoomMode::Cursor)))
            .build(state, zoom_levels_list, |builder| {
                builder.set_text("147X").class("zoom")
            });

        self.zoom_7 = RadioButton::new()
            .on_checked(Event::new(AppEvent::SetZoomLevel(7, ZoomMode::Cursor)))
            .build(state, zoom_levels_list, |builder| {
                builder.set_text("49X").class("zoom")
            });

        self.zoom_6 = RadioButton::new()
            .on_checked(Event::new(AppEvent::SetZoomLevel(6, ZoomMode::Cursor)))
            .build(state, zoom_levels_list, |builder| {
                builder.set_text("21X").class("zoom")
            });

        self.zoom_5 = RadioButton::new()
            .on_checked(Event::new(AppEvent::SetZoomLevel(5, ZoomMode::Cursor)))
            .build(state, zoom_levels_list, |builder| {
                builder.set_text("9X").class("zoom")
            });

        self.zoom_4 = RadioButton::new()
            .on_checked(Event::new(AppEvent::SetZoomLevel(4, ZoomMode::Cursor)))
            .build(state, zoom_levels_list, |builder| {
                builder.set_text("3X").class("zoom")
            });

        self.zoom_3 = RadioButton::new()
            .on_checked(Event::new(AppEvent::SetZoomLevel(3, ZoomMode::Cursor)))
            .build(state, zoom_levels_list, |builder| {
                builder.set_text("1X").class("zoom")
            })
            .set_checked(state, true);

        self.zoom_2 = RadioButton::new()
            .on_checked(Event::new(AppEvent::SetZoomLevel(2, ZoomMode::Cursor)))
            .build(state, zoom_levels_list, |builder| {
                builder.set_text("0.5X").class("zoom")
            });

        self.zoom_1 = RadioButton::new()
            .on_checked(Event::new(AppEvent::SetZoomLevel(1, ZoomMode::Cursor)))
            .build(state, zoom_levels_list, |builder| {
                builder.set_text("0.25X").class("zoom")
            });

        self.zoom_0 = RadioButton::new()
            .on_checked(Event::new(AppEvent::SetZoomLevel(0, ZoomMode::Cursor)))
            .build(state, zoom_levels_list, |builder| {
                builder.set_text("0.1X").class("zoom")
            });

        // TODO
        // RadioButton::new()
        //     .on_checked(Event::new(AppEvent::SetZoomLevel(0)))
        //     .build(state, zoom_levels_list, |builder| {
        //         builder.set_text("FIT").class("zoom")
        //     });

        Button::with_label(ICON_PLUS)
            .on_press(Event::new(AppEvent::IncZoom).target(entity))
            .build(state, zoom_controls, |builder| {
                builder.set_font("Icons").class("zoom").class("last")
            });

        entity.set_element(state, "entity")
    }
}

impl EventHandler for Footer {
    fn on_event(&mut self, state: &mut State, entity: Entity, event: &mut Event) {
        if let Some(app_event) = event.message.downcast::<AppEvent>() {
            match app_event {
                AppEvent::SetZoomLevel(val, zoom_mode) => {
                     // This is terrible and I hate myself
                     let zoom_entity = match val {
                        0 => self.zoom_0,
                        1 => self.zoom_1,
                        2 => self.zoom_2,
                        3 => self.zoom_3,
                        4 => self.zoom_4,
                        5 => self.zoom_5,
                        6 => self.zoom_6,
                        7 => self.zoom_7,
                        8 => self.zoom_8,
                        _ => Entity::root(),
                    };

                    // Send an event that will be intercepted by the radio list to change the zoom selection
                    state.insert_event(Event::new(CheckboxEvent::Check).target(zoom_entity));
                    // Let the dropdown know it should change
                    state.insert_event(Event::new(AppEvent::SetZoomLevel(*val,zoom_mode.clone())).target(self.zoom_levels_dropdown).propagate(Propagation::Direct));
                }

                _=> {}
            }
        }

        if let Some(info_event) = event.message.downcast::<InfoEvent>() {
            match info_event {
                // InfoEvent::SetCursorLabel(text) => {
                //     self.cursor_label.set_text(state, text);
                // }

                // InfoEvent::SetSelectLabel(text) => {
                //     self.select_label.set_text(state, text);
                // }

                InfoEvent::SetTooltip(text) => {
                    self.tooltip.set_text(state, text);
                }

                InfoEvent::SetFileNameLabel(text) => {
                    self.file_name_label.set_text(state, text);
                }

                _=> {}
            }
        }
    }
}


// A dropdown container for zoom controls (inherited from a dropdown container)
pub struct ZoomDropdown {
    dropdown: Dropdown,
}

impl ZoomDropdown {
    pub fn new() -> Self {
        Self {
            dropdown: Dropdown::new("1X"),
        }
    }
}

impl BuildHandler for ZoomDropdown {
    type Ret = (Entity, Entity, Entity);
    fn on_build(&mut self, state: &mut State, entity: Entity) -> Self::Ret {
        self.dropdown.on_build(state, entity)
    }
}

impl EventHandler for ZoomDropdown {
    fn on_event(&mut self, state: &mut State, entity: Entity, event: &mut Event) {
        self.dropdown.on_event(state, entity, event);

        if let Some(app_event) = event.message.downcast::<AppEvent>() {
            match app_event {
                AppEvent::SetZoomLevel(val, _) => {
                    self.dropdown.label.set_text(
                        state,
                        &((441.0 / SAMPLES_PER_PIXEL[*val] as f32).to_string() + "X"),
                    );
                }

                _ => {}
            }
        }
    }
}
