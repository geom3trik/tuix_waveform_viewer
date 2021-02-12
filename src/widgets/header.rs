
use super::app_event::{*, self};
use super::icons::*;
use tuix::*;

#[derive(Debug, Clone, PartialEq)]
pub enum InfoEvent {
    SetTimeLabel(String),
}

pub struct Header {
    play_button: Entity,
    playhead_label: Entity,
}

impl Header {
    pub fn new() -> Self {
        Self {
            play_button: Entity::default(),
            playhead_label: Entity::default(),
        }
    }
}

impl BuildHandler for Header {
    type Ret = Entity;
    fn on_build(&mut self, state: &mut State, entity: Entity) -> Self::Ret {
        // Open file button
        Button::new()
            .on_release(Event::new(AppEvent::OpenFileDialog))
            .build(state, entity, |builder| {
                builder
                    .set_text("Open")
                    .set_margin(Length::Pixels(10.0))
                    .class("open")
            });

        // Transpoort controls
        let transport = Element::new().build(state, entity, |builder| builder.class("transport"));

        // To start button
        Button::new()
            .on_press(Event::new(AppEvent::SeekLeft))
            .build(state, transport, |builder| {
                builder
                    .set_text(ICON_TO_START)
                    .set_font("Icons")
                    .class("first")
            });

        // Play/Pause Checkbox
        self.play_button = Checkbox::new(true)
            .on_unchecked(Event::new(AppEvent::Play))
            .on_checked(Event::new(AppEvent::Pause))
            .with_icon_checked(ICON_PLAY)
            .with_icon_unchecked(ICON_PAUSE)
            .build(state, transport, |builder| {
                builder.set_text(ICON_PLAY).set_font("Icons").class("play")
            });

        // Stop button
        Button::new()
            .on_press(Event::new(AppEvent::Stop))
            .build(state, transport, |builder| {
                builder.set_text(ICON_STOP).set_font("Icons")
            });

        // To end button
        Button::new().build(state, transport, |builder| {
            builder.set_text(ICON_TO_END).set_font("Icons")
        });

        // Loop checkbox
        let loop_button = Checkbox::new(true)
            .on_unchecked(Event::new(AppEvent::Loop(false)).target(entity))
            .on_checked(Event::new(AppEvent::Loop(true)).target(entity))
            .with_icon_checked(ICON_LOOP)
            .with_icon_unchecked(ICON_LOOP)
            .build(state, transport, |builder| {
                builder.set_font("Icons").class("loop").class("last")
            });

        self.playhead_label = Label::new("00''00'00.0").build(state, entity, |builder| {
            builder.class("info").set_margin(Length::Pixels(10.0))
        });

        // Mute/Unmute checkbox
        Checkbox::new(true)
            .on_unchecked(Event::new(AppEvent::Mute(false)))
            .on_checked(Event::new(AppEvent::Mute(true)))
            .with_icon_checked(ICON_SOUND)
            .with_icon_unchecked(ICON_MUTE)
            .build(state, entity, |builder| {
                builder.set_font("Icons").class("snap")
            });

        // Volume slider
        Slider::new()
            .on_change(move |value| Event::new(AppEvent::Volume(value)))
            .build(state, entity, |builder| builder.class("volume"));

        // Channels selector
        let channels = RadioList::new().build(state, entity, |builder| builder.class("checklist"));

        // Left channel Button
        RadioButton::new()
            .on_checked(Event::new(AppEvent::SwicthChannel(ChannelMode::Left)).target(entity))
            .build(state, channels, |builder| {
                builder.set_text("L").class("first")
            })
            .set_checked(state, true);

        // Right channel button
        RadioButton::new()
            .on_checked(Event::new(AppEvent::SwicthChannel(ChannelMode::Right)).target(entity))
            .build(state, channels, |builder| builder.set_text("R"));

        // Both channels button
        RadioButton::new()
            .on_checked(Event::new(AppEvent::SwicthChannel(ChannelMode::Both)).target(entity))
            .build(state, channels, |builder| {
                builder
                    .set_text("L + R")
                    .class("last")
                    .set_width(Length::Pixels(60.0))
            });

        // Units selector
        let units = RadioList::new().build(state, entity, |builder| builder.class("checklist"));

        // Linear units button
        RadioButton::new()
            .on_checked(Event::new(AppEvent::SwitchUnits(UnitsMode::Linear)).target(entity))
            .build(state, units, |builder| {
                builder.set_text("Mag").class("first")
            })
            .set_checked(state, true);

        // Decibel units button
        RadioButton::new()
            .on_checked(Event::new(AppEvent::SwitchUnits(UnitsMode::Decibel)).target(entity))
            .build(state, units, |builder| builder.set_text("dB").class("last"));

        entity.set_element(state, "header")
    }
}

impl EventHandler for Header {
    fn on_event(&mut self, state: &mut State, entity: Entity, event: &mut Event) {
        
        // Respond to App events
        if let Some(app_event) = event.message.downcast::<AppEvent>() {
            match app_event {
                AppEvent::Play => {
                    state.insert_event(Event::new(CheckboxEvent::Uncheck).target(self.play_button).propagate(Propagation::Direct));
                }

                AppEvent::Pause => {
                    state.insert_event(Event::new(CheckboxEvent::Check).target(self.play_button).propagate(Propagation::Direct));
                }

                AppEvent::Stop => {
                    state.insert_event(Event::new(CheckboxEvent::Check).target(self.play_button).propagate(Propagation::Direct));
                }

                _=> {}
            }
        }

        // Respond to Info events
        if let Some(info_event) = event.message.downcast::<InfoEvent>() {
            match info_event {
                InfoEvent::SetTimeLabel(val) => {
                    self.playhead_label.set_text(state, &val);
                }

                _=> {}
            }
        }
    }
}
