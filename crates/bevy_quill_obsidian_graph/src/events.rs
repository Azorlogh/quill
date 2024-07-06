use bevy::prelude::*;
use bevy_mod_picking::prelude::*;

/// For a connection drag, where we are dragging from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ConnectionAnchor {
    /// Drag from an input terminal
    InputTerminal(Entity),
    /// Drag from an output terminal
    OutputTerminal(Entity),
    /// Dragging the source end (connected to an output) of an existing edge.
    EdgeSource(Entity),
    /// Dragging the sink end (connected to an input) of an existing edge.
    EdgeSink(Entity),
}

/// For a connection drag, the current drop location.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ConnectionTarget {
    InputTerminal(Entity),
    OutputTerminal(Entity),
    None,
}

#[derive(Clone, Debug)]
pub enum Gesture {
    /// Drag one or more nodes (ones that are currently selected).
    /// The arguments are the drag vector, and whether this is the final drag value.
    Move(Vec2, bool),

    /// Drag a node onto the graph to create it.
    Create(Vec2),

    /// Start dragging a connection. The argument is the terminal.
    Connect(ConnectionAnchor),

    /// Drag the connection to a new location.
    ConnectDrag(Vec2),

    /// Called when the connection hovers over a target.
    ConnectHover(ConnectionTarget),

    /// Finish dragging the connection.
    ConnectFinish,

    /// Option-click to scroll the view.
    Scroll(Vec2),

    /// Select a rectangular region
    SelectRect(Rect),

    /// Select the given node. If the node is already selected, does nothing. If the node is
    /// not selected, clears the selection and selects only the given node.
    Select(Entity),

    /// Add a node to the selection, don't affect other nodes.
    SelectAdd(Entity),

    /// Remove a node from the selection.
    SelectRemove(Entity),

    /// Toggle the selection state of a node.
    SelectToggle(Entity),

    /// Remove all nodes from the selection.
    SelectClear,

    /// Cancel the current action.
    Cancel,
}

/// Mouse wheel entity event
#[derive(Clone, Event, EntityEvent, Debug)]
#[can_bubble]
pub struct GraphEvent {
    /// Event target
    #[target]
    pub target: Entity,
    /// The type of gesture.
    pub gesture: Gesture,
}

#[derive(Clone, Default, Debug, PartialEq)]
pub(crate) enum DragMode {
    #[default]
    None,
    Move,
    RectSelect,
    Connect,
}

#[derive(Resource, Default)]
pub(crate) struct GestureState {
    /// The type of gesture.
    pub(crate) mode: DragMode,
}
