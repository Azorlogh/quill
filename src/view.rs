use crate::{cx::Cx, tracking_scope::TrackingScope, NodeSpan};
use bevy::prelude::{Added, Component, Entity, World};
use std::sync::{Arc, Mutex};

#[allow(unused)]
/// An object which produces one or more display nodes.
pub trait View: Sync + Send + 'static {
    /// The external state for this View.
    type State: Send + Sync;

    /// Return the span of entities produced by this View.
    fn nodes(&self, state: &Self::State) -> NodeSpan;

    /// Construct and patch the tree of UiNodes produced by this view.
    /// This may also spawn child entities representing nested components.
    fn build(&self, cx: &mut Cx) -> Self::State;

    /// Update the internal state of this view, re-creating any UiNodes.
    /// Returns true if the output changed, that is, if `nodes()` would return a different value
    /// than it did before the rebuild.
    fn rebuild(&self, cx: &mut Cx, state: &mut Self::State) -> bool;

    /// Instructs the view to attach any child entities to the parent entity. This is called
    /// whenever we know that one or more child entities have changed.
    fn attach_children(&self, cx: &mut Cx, state: &mut Self::State) {}

    /// Recursively despawn any child entities that were created as a result of calling `.build()`.
    /// This calls `.raze()` for any nested views within the current view state.
    fn raze(&self, world: &mut World, state: &mut Self::State);
}

/// Combination of a [`View`] and it's built state.
pub struct ViewState<S, V: View<State = S>> {
    // TODO: These should not be public.
    pub(crate) view: V,
    pub(crate) state: Option<S>,
    // owner?
    // props?
}

/// Type-erased trait for a [`ViewState`].
pub trait AnyViewState: Sync + Send + 'static {
    /// Return the span of entities produced by this View.
    fn nodes(&self) -> NodeSpan;

    /// Update the internal state of this view, re-creating any UiNodes. Returns true if the output
    /// changed, that is, if `nodes()` would return a different value than it did before the
    /// rebuild.
    fn rebuild(&mut self, cx: &mut Cx) -> bool;

    /// Recursively despawn any child entities that were created as a result of calling `.build()`.
    /// This calls `.raze()` for any nested views within the current view state.
    fn raze(&mut self, world: &mut World);
}

impl<S: Send + Sync + 'static, V: View<State = S>> AnyViewState for ViewState<S, V> {
    fn nodes(&self) -> NodeSpan {
        match self.state {
            Some(ref state) => self.view.nodes(state),
            None => NodeSpan::Empty,
        }
    }

    fn rebuild(&mut self, cx: &mut Cx) -> bool {
        match self.state {
            Some(ref mut state) => self.view.rebuild(cx, state),
            None => {
                self.state = Some(self.view.build(cx));
                true
            }
        }
    }

    fn raze(&mut self, world: &mut World) {
        if let Some(state) = &mut self.state {
            self.view.raze(world, state);
            self.state = None;
        }
    }
}

/// An ECS component which holds a reference to the root of a view hierarchy.
#[derive(Component)]
pub struct ViewRoot(pub Arc<Mutex<dyn AnyViewState>>);

impl ViewRoot {
    pub fn new(view: impl View) -> Self {
        Self(Arc::new(Mutex::new(ViewState { view, state: None })))
    }
}

/// An ECS component which holds a reference to a view state.
#[derive(Component)]
pub struct ViewCell(pub Arc<Mutex<dyn AnyViewState>>);

// pub trait View: Send
// where
//     Self: Sized,
// {
//     /// Inserts a default instance of the specified component or bundle to the display entity.
//     /// This insertion occurs only once per output entity. The entity takes ownership of the
//     /// bundle.
//     ///
//     /// This method will panic if you call this on a view which produces more than one output
//     /// entity, since only one entity can take ownership.
//     fn insert<B: Bundle>(self, component: B) -> ViewInsertBundle<Self, B> {
//         ViewInsertBundle {
//             inner: self,
//             bundle: Cell::new(Some(component)),
//         }
//     }

//     /// Sets up a callback which is called for each output UiNode generated by this `View`.
//     /// Typically used to manipulate components on the entity. This is called each time the
//     /// view is rebuilt.
//     fn with<F: Fn(EntityWorldMut) + Send>(self, callback: F) -> ViewWith<Self, F> {
//         ViewWith {
//             inner: self,
//             callback,
//         }
//     }

//     /// Sets up a callback which is called for each output UiNode generated by this `View`.
//     /// Typically used to manipulate components on the entity. This callback is called when
//     /// the view is first created, and then called again if either (a) the output entity
//     /// changes, or (b) the value of the [`deps`] parameter is different than the previous
//     /// call.
//     fn with_memo<D: Clone + PartialEq + Send, F: Fn(EntityWorldMut) + Send>(
//         self,
//         callback: F,
//         deps: D,
//     ) -> ViewWithMemo<Self, D, F> {
//         ViewWithMemo {
//             inner: self,
//             callback,
//             deps,
//         }
//     }
// }

// /// `ViewState` contains all of the data needed to re-render a presenter: The presenter function,
// /// its properties, its state, and the cached output nodes.
// ///
// /// This type is generic on the props and state for the presenter.
// pub struct ViewState<Marker: 'static, F: PresenterFn<Marker>> {
//     /// Reference to presenter function
//     presenter: F,

//     /// Props passed to the presenter
//     // props: F::Props,

//     /// View tree output by presenter
//     view: Option<F::View>,

//     /// Externalized state defined by view tree
//     state: Option<<F::View as View>::State>,

//     /// The UiNodes generated by this view state
//     nodes: NodeSpan,
// }

/// A reference to a [`View`] which can be passed around as a parameter.
pub struct ViewHandle(pub(crate) Arc<Mutex<dyn AnyViewState>>);

impl ViewHandle {
    /// Construct a new [`ViewRef`] from a [`View`].
    pub fn new(view: impl View) -> Self {
        Self(Arc::new(Mutex::new(ViewState { view, state: None })))
    }

    // /// Given a view template, construct a new view. This creates an entity to hold the view
    // /// and the view handle, and then calls [`View::build`] on the view. The resuling entity
    // /// is part of the template invocation hierarchy, it is not a display node.
    // pub fn spawn(view: &ViewHandle, parent: Entity, world: &mut World) -> Entity {
    //     todo!("spawn view");
    //     // let mut child_ent = world.spawn(ViewCell(view.0.clone()));
    //     // child_ent.set_parent(parent);
    //     // let id = child_ent.id();
    //     // view.0.lock().unwrap().build(child_ent.id(), world);
    //     // id
    // }

    /// Returns the display nodes produced by this `View`.
    pub fn nodes(&self) -> NodeSpan {
        self.0.lock().unwrap().nodes()
    }

    /// Destroy the view, including the display nodes, and all descendant views.
    pub fn raze(&self, world: &mut World) {
        self.0.lock().unwrap().raze(world);
    }
}

impl Clone for ViewHandle {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl Default for ViewHandle {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(EmptyView)))
    }
}

/// Trait that allows a type to be converted into a [`ViewRef`].
// pub trait IntoView {
//     /// Convert the type into a [`ViewRef`].
//     fn into_view(self) -> Arc<Mutex<dyn AnyViewState>>;
// }

// impl IntoView for Arc<Mutex<dyn AnyViewState>> {
//     fn into_view(self) -> Arc<Mutex<dyn AnyViewState>> {
//         self
//     }
// }

// impl IntoView for () {
//     fn into_view(self) -> Arc<Mutex<dyn AnyViewState>> {
//         Arc::new(Mutex::new(EmptyView))
//         // ViewHandle::new(EmptyView)
//     }
// }

// impl IntoView for &str {
//     fn into_view(self) -> Arc<Mutex<dyn AnyViewState>> {
//         Arc::new(Mutex::new(TextView::new(self.to_string())))
//     }
// }

// impl IntoView for Signal<String> {
//     fn into_view(self) -> ViewRef {
//         ViewRef::new(TextComputed::new(move |rcx| self.get_clone(rcx)))
//     }
// }

// impl IntoView for String {
//     fn into_view(self) -> Arc<Mutex<dyn AnyViewState>> {
//         Arc::new(Mutex::new(ViewState {
//             view: TextView::new(self),
//             state: None,
//         }))
//     }
// }

// impl<V: IntoView> IntoView for Option<V> {
//     fn into_view(self) -> Arc<Mutex<dyn AnyViewState>> {
//         match self {
//             Some(v) => v.into_view(),
//             None => Arc::new(Mutex::new(EmptyView)),
//         }
//     }
// }

/// View which renders nothing.
pub struct EmptyView;

#[allow(unused_variables)]
impl AnyViewState for EmptyView {
    fn nodes(&self) -> NodeSpan {
        NodeSpan::Empty
    }

    fn rebuild(&mut self, cx: &mut Cx) -> bool {
        false
    }
    fn raze(&mut self, world: &mut World) {}
}

pub(crate) fn create_views(world: &mut World) {
    let mut roots = world.query_filtered::<(Entity, &ViewRoot), Added<ViewRoot>>();
    let roots_copy: Vec<Entity> = roots.iter(world).map(|(e, _)| e).collect();
    let tick = world.change_tick();
    for root_entity in roots_copy.iter() {
        let Ok((_, root)) = roots.get(world, *root_entity) else {
            continue;
        };
        let inner = root.0.clone();
        let mut scope = TrackingScope::new(tick);
        let mut cx = Cx::new(world, *root_entity, &mut scope);
        inner.lock().unwrap().rebuild(&mut cx);
    }
}
