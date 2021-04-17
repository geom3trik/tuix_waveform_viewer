

#[derive(Debug, Clone, PartialEq)]
pub enum ChannelMode {
    Left,
    Right,
    Both,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnitsMode {
    Linear,
    Decibel,
}
#[derive(Debug, Clone, PartialEq)]
pub enum PlayState {
    Playing,
    Paused,
    Stopped,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ZoomMode {
    Cursor,
    Mouse,
}

// Events used to update the time display labels
#[derive(Debug, Clone, PartialEq)]
pub enum InfoEvent {
    SetTimeLabel(String),
    SetCursorLabel(String),
    SetSelectLabel(String),
    SetFileNameLabel(String),
    SetTooltip(String),
}
#[derive(Debug, Clone, PartialEq)]
pub enum AppEvent {
    OpenFileDialog,
    LoadAudioFile(String),
    SwicthChannel(ChannelMode),
    SwitchUnits(UnitsMode),
    SetZoomLevel(usize, ZoomMode),
    FollowPlayhead(bool),
    Loop(bool),
    Volume(f32),

    Mute(bool),
    IncZoom,
    DecZoom,

    Play,
    Pause,
    Stop,
    SeekLeft,
    SeekRight,
}