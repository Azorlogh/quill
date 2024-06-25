# Quill

**Quill** is a UI framework for the Bevy game engine. It's meant to provide a simple API for
constructing reactive user interfaces, similar to frameworks like React and Solid, but built on a
foundation of Bevy ECS state management.

Quill is an experimental library which borrows ideas from a number of popular UI frameworks,
including React.js, Solid.js, Dioxus, and Xilem. However, the way these ideas are implemented is
quite different, owing to the need to build on the foundations of Bevy ECS.

Quill is relatively unopinionated about styling, layout or ECS hierarchies - you can use it to
build a traditional 2D game UI, gizmo-like overlays in 3D, or reactive scenes. However, Quill
comes with a separate crate, `quill_obsidian` which provides a set of opininated widgets for
building game editors.

## Getting started

> :warning: Quill currently requires the unstable Rust feature `impl_trait_in_assoc_type`. This requirement will go away once the feature has been stabilized.

> :warning: Quill is currently in early development, and is likely to change as it evolves.

For now, you can run the examples. The "complex" example shows off multiple features of the
library:

```sh
cargo run --example complex
```

## Aspirations / guiding principles:

- Allows easy composition and re-use of hierarchical widgets.
- Built on top of existing Bevy UI components.
- No special syntax required, it's just Rust.
- Allows reactive hooks such as `use_resource()` that hook into Bevy's change detection framework.
- State management built on top of Bevy ECS, rather than maintaining its own separate UI "world".
- Any data type (String, int, color, etc.) can be displayed in the UI so long as it implements
  the `View` trait.
- Efficient rendering approach with minimal memory allocations. Uses a hybrid approach that borrows
  from both React and Solid to handle incremental modifications of the UI node graph.
- Supports dynamic styling and animation effects.

<!-- Check out the demo video [here](https://youtu.be/NXabt3NrKMg). -->

## A Basic Example

To create a basic widget, start by creating a struct which implements the `ViewTemplate` trait.
This trait has one method, `create()`, which takes a context (`Cx`) and returns a `View`.

```rust
/// A view template
struct MyWidget;

impl ViewTempate for MyWidget {
    type View = impl View;

    fn create(cx: &mut Cx) -> Self::View {
        // Access data in a resource
        let counter = cx.use_resource::<Counter>();
        Element::<NodeBundle>::new().children((
            format!("The count is: {}", counter.count),
        ))
    }
}
```

To actually display this widget, you'll need to set up a few things:

- Add `QuillPlugin` in your app's plugins.
- Initialize the `Counter` resource.
- Spawn a view root.

The view root is an ECS entity which represents the root of the UI hierarchy. Note that there are
actually two hierarchies, the "view hierarchy" which is the tree of templates and reactive objects,
and the "display hierarchy", which are the actual Bevy entities that get rendered on the screen.
You only need to worry about the former, since the display hierarchy is automatically constructed.

To spawn a root for the `MyWidget` template, use Bevy `commands` in a setup system:

```rust
commands.spawn(MyWidget.to_root());
```

## View Structure and Lifecycle

Quill manages views and templates on three different levels:

- `ViewTemplates` are application-level components that "react" when their dependencies change.
  If you have every used `React.js`, a `ViewTemplate` is the equivalent of a React `Component`.
- `Views` are lower-level constructs that generate the basic ECS building blocks of a UI,
  such as entities and components. Views understand incremental updates: how to patch an ECS
  hierarchy to make modifications without re-constructing the whole tree.
- "display nodes" are the actual renderable ECS entities created by the views.

Every `ViewTemplate` has a method called `create` which is called when the template is first
spawned, and which is called again each time the display needs to update. It's important to
understand how `create()` is called, as this is the key to working with Quill:

- `create()` is typically called many times, which means that any code inside of `create` needs to
  be written in a way that is repeatable. Fortunately, the `Cx` object has lots of methods to help
  with this: for example, if you want to write some code that only runs once, or only runs under
  certain situations, you can call `cx.create_effect()`.
- `create()` is reactive, meaning that it will be run again whenever one of its dependnencies change.
  For example, when you access a Bevy resource or component, it automatically add that resource or
  component to a tracking list. If some other function later modifies that resource or component,
  it will trigger a reaction, which will cause `create` to run again.

It's important to write the `create()` method in a way that doesn't leak memory or resources.
For example, it would be a mistake to write a `create()` method that calls `material_assets.add()`
directly, since this would add a new material every update, returning a new handle. Instead, you can
wrap the material initialization in a call to `cx.create_memo()`, which will allow you to preserve
the material handle between invocations.

As a general rule, you should write your templates in a "mostly functional" style, minimizing
side effects. When you do need side effects, there are special methods available like `.insert()`
or `.create_mutable()` to help you out.

The return value from `create` is an object that implements the `View` trait. `Views` have a more
complicated lifecycle that involves methods such as `build()`, `rebuild()` and `raze()`, but most
of the time you won't need to worry about these.

### Elements

Typically you won't need to write your own implementations of `View`, as these have already
been provided. The most important of these is the `Element` type, which creates a single display
node entity. Elements have three aspects:

- A bundle type: the type of bundle that will be inserted into the entity upon creation.
- Zero or more children.
- Zero or more "effects".

The children of an element are also views, defined using `View` objects. These are constructed by
the parent and inserted as children of the display node using the standard Bevy parent/child
relations.

"Effects" are anything that add or modify an entity's ECS components, such as:

- Adding `bevy_mod_picking` event handlers.
- Adding custom materials.
- Adding animations.
- Adding ARIA nodes for accessibility.
- Applying styles.

Children and effects are added using a builder pattern, as in the following example:

```rust
Element::<NodeBundle>::new()
    .style(style_panel)
    .children((
        "Hello, ",
        "world!",
    ))
```

The `children()` method accepts either a single view or a tuple of views. In this case, we're passing
plain text strings. Because the `View` trait has an implementation for `&str`, these strings can
be displayed as views, and will construct the appropriate bevy_ui `Text` nodes.

Note that `ViewTemplates` also implement `View`, so you can freely mix both templates and views
when defining children.

In the above example, the `.style()` method adds a "style effect" - an effect which initializes the
style components of the entity.

The `.style()` method is an example of a _static effect_, an effect which is only applied once
when the element is first created. There are also _dynamic effects_ which can be applied multiple
times. Dynamic effects typically take a list of dependencies, and only re-run the effect when the
dependencies change. For example, here's a style effect which displays a "focus rectangle" around
a checkbox, but only when the "focus" flag (as determined by the accessiibility `Focus` resource)
is set to true:

```rust
.style_dyn(
    // This closure only runs when 'focused' changes.
    |focused, sb| {
        if focused {
            sb.outline_color(colors::FOCUS)
                .outline_offset(1.0)
                .width(2.0);
        } else {
            sb.outline_color(Option::<Color>::None);
        }
    },
    focused,
)
```

Another frequently-used effect is `.insert()` and it's dynamic counterpart, `.insert_dyn()`, which
inserts an ECS component. There's also a variation, `.insert_if()` which inserts a component when
the condition is true, and removes it when it's false - handy for marker components like `Disabled`.

The `.insert()` method is frequently used for inserting `bevy_mod_picking` event handlers.

## More Examples

### Conditional rendering with `Cond`

The `Cond` (short for "conditional") view takes a conditional expression, and two child views, one
which is rendered when the condition is true, the other when the condition is false.

```rust
/// Widget that displays whether the count is even or odd.
struct EvenOrOdd;

impl ViewTempate for EvenOrOdd {
    type View = impl View;

    fn create(cx: &mut Cx) -> Self::View {
        let counter = cx.use_resource::<Counter>();
        Element::new().children((
            "The count is: ",
            Cond::new(counter.count & 1 == 0, "even", "odd"),
        ))
    }
}
```

Note: It's perfectly OK to use a regular `if` statement or `match` for conditional views, however
there is a limitation, which is that the result type has to be the same for all branches. `Cond`
doesn't have this limitation, the `true` and `false` branches can be different types. Internally,
`Cond` tears down the previous branch and initializes the new branch whenever the condition variable
changes.

Often the "false" branch of a `Cond` will be the empty view, `()`, which renders nothing and
creates no entities.

### Rendering multiple items with `For`

`For::each()` takes a list of items, and a callback which renders a `View` for each item:

```rust
struct EventLog;

impl ViewTempate for EventLog {
    type View = impl View;

    fn create(cx: &mut Cx) -> Self::View {
      let log = cx.use_resource::<ClickLog>();
      Element::new()
          .children(For::each(&log.0, |item| {
              Element::new()
                  .styled(STYLE_LOG_ENTRY.clone())
                  .children((item.to_owned(), "00:00:00"))
          }).with_fallback("No items")),
    }
}
```

During updates, the `For` view compares the list of items with the previous list and computes
a diff. Only items which have actually changed (insertions, deletions and mutations) are
rebuilt. There are three different variations of the `For` construct, which differ in how they
handle comparisons between items:

- `For::each()` requires that the array elements implement `PartialEq`.
- `For::each_cmp()` takes an additional comparator argument which is used to compare the items.
- `For::indexed()` doesn't compare items, but instead uses the array index as a key. This version
  is less efficient, since an item insertion or deletion will require re-building all of the
  child views.

There is also `For::index()` and `For::keyed()`.

### Returning multiple nodes

Normally a `ViewTemplate` returns a single view. If you want to return multiple views, use a `Fragment`:

```rust
Fragment::new((
    "Hello, ",
    "World!"
))
```

The children of the `Fragment` will be inserted inline in place of the `Fragment` node.

## Mutables: Local state

It's common in UI code where a parent widget will have to keep track of some local state.
Often this state needs to be accessible by both the code that creates the UI and the event
handlers. "Mutables" are a way to manage local state in a reactive way.

A `Mutable<T>` is a handle which refers to a piece of mutable data stored within the Bevy World.
Since the Mutable itself is just an id, it supports Clone/Copy, which means you can pass it around
to child views or other functions.

Creating a new `Mutable` is performed via the `create_mutable::<T>(value)` method which is implemented
for both `Cx` and `World`. Note that mutables created through `Cx` are owned by the current view,
and are automatically despawned when the view is despawned. Handles created on the world are not;
you are responsible for deleting them.

There are several ways to access data in a mutable, but all of them require some kind of context
so that the data can be retrieved. This context can be either a `Cx` context object or a `World`.

- `mutable.get(cx)`
- `mutable.get(world)`
- `mutable.set(cx, new_value)`
- `mutable.set(world, new_value)`

Because `Mutables` are components, they are also reactive: calling `mutable.get(cx)` automatically
adds the mutable to the tracking set for that context. This does not happen if you pass in a `World`
however.

The `.get()`, `.set()` methods given above assume that the data in the mutable implements `Copy`.
There are also `.get_clone()` and `.set_clone()` methods, which works with data types that
implement `Clone`.

You can also update Mutables in place via `.update()`, which takes a callback that is passed
a reference to the mutable data.

## Element::from_entity() and explicit entity ids

Elements normally spawn a new Entity when they are built. However, there are cases where you want
to specify the entity id of an entity that has already been spawned. For this, you can use
`Element::for_entity(id)` instead of `Element::new()`.

An example of where this is useful is hovering: we want to highlight the color of an element when
the mouse is over it. To do this, we want to pass in an "is_hovered" flag as a parameter when
constructing the element so that the proper styles can be computed. But computing "is_hovered"
requires knowing the element's entity id, which doesn't exist until the element has been created.

In the Obsidian library, there's a hook (an extension to `Cx`) named `is_hovering(id)` which returns
true if a given element is currently being hovered over by the mouse. We can set up our widget
with the following steps:

- create a new entity id using `id = cx.create_entity()`.
- check for hovering using `cx.is_hovering(id)`.
- create an `Element` using the created id with `Element::for_entity(id)`.
- in the builder methods for the `Element`, use `is_hovered` to conditionally apply styles.

<!-- ### Advanced hooks

There are several advanced hooks in the examples directory. These hooks are not part of Quill,
but will likely be included in "Egret" which is the planned headless widget library based on Quill.

#### `use_enter_exit()`

The `.use_enter_exit()` hook is useful for elements such as dialog boxes which do both an open
and close animation. The idea is that when a dialog box closes you don't want to destroy the display
graph until the closing animation is complete:

```rust
use super::enter_exit::EnterExitApi;

let state = cx.use_enter_exit(open, 0.3);
```

In this example, `open` is a boolean flag which is true when we want the dialog to be open, and
false when we want it to be closed. Changing this flag starts a timer which drives a state machine;
the second argument is the delay time, in seconds, for the animations.

The output, `state`, is an enum which can have one of six values:

```rust
#[derive(Default, Clone, PartialEq)]
pub enum EnterExitState {
    EnterStart,
    Entering,
    Entered,
    ExitStart,
    Exiting,

    #[default]
    Exited,
}
```

These values can be converted into strings by calling `.as_class_name()` on them. The resulting
value can be put directly on the element as a class name, and the class names can be used in
dynamic stylesheet selectors.

The calling presenter should, in most cases, render the item whenever the state is not `Exited`.

```rust
let state = cx.use_enter_exit(open, 0.3);
If::new(
    state != EnterExitState::Exited,
    Element::new().class_name(state.as_class_name()), // Dialog content, etc.
    ()
```

#### `use_element_rect()`

The `use_element_rect()` hook returns the rectangular bounds of a ui node as a reactive data source,
meaning that it will trigger an update whenever the position or size of the element changes.
The input is the entity id of the element we wish to measure:

```rust
let rect = cx.use_element_rect(id_inner);
``` -->

## Styling

There are several different ways to approach styling in Bevy. One is "imperative styles", meaning
that you explicitly create style components such as `BackgroundColor` and `Outline` in the template
and attach them to the display node.

A disadvantage of this approach is that you have limited ability to compose styles from different
sources. Rust has one mechanism for inheriting struct values from another struct, which is the
`..` syntax; this supposes that both of the struct values are known at the point of declaration.
Ideally, what we want is a general mechanism that allows to take a "base" style and then add
customizations on top of it, but in a way that doesn't require exposing the inner details of the
base style to the world.

Quill takes a more functional approach to styles, using the `bevy_mod_stylebuilder` crate. This
crate provides a `StyleBuilder` interface that lets you define Bevy styles using a fluent builder
pattern. Moreover, it supports composition: you can pass in a tuple of styles
`(style1, style2, style3)` and each of the three styles will be applied in the given order.

"Styles" in this system are just functions that take a `StyleBuilder` parameter. For example, the
Obsidian `Button` widget uses the following style:

```rust
fn style_button(ss: &mut StyleBuilder) {
    ss.border(1)
        .display(ui::Display::Flex)
        .flex_direction(ui::FlexDirection::Row)
        .justify_content(ui::JustifyContent::Center)
        .align_items(ui::AlignItems::Center)
        .align_content(ui::AlignContent::Center)
        .padding((12, 0))
        .border(0)
        .color(colors::FOREGROUND)
        .cursor(CursorIcon::Pointer);
}
```

Within a view template, multiple styles can be added to the element:

```rust
element.style((
    // Default text styles
    typography::text_default,
    // Standard Button styles
    style_button,
    // Custom style overrides
    self.custom_styles.clone()
))
```

For convenience, the `StyleBuilder` API supports both "long-form" and "shortcut" syntax variations.
For example, the following are all equivalent:

- `.border(ui::UiRect::all(ui::Val::Px(10.)))` -- a border of 10px on all sides
- `.border(ui::Val::Px(10.))` -- Scalar is automatically converted to a rect
- `.border(10.)` -- `Px` is assumed to be the default unit
- `.border(10)` -- Integers are automatically converted to f32 type.

Similarly, it offers many automatic conversions for types such as colors, rectangles, and asset paths.

For those familiar with CSS, there will be some familiarity, however `StyleBuilder` differs from
CSS in a number of important ways:

- There is no prioritization or cascade, as this tends to be a source of confusion for web
  developers (Even CSS itself is moving away from this with the new "CSS layers" feature.) Instead
  styles are merged strictly in the order that they appear on the element.
- Styles can only affect the element they are assigned to, not their children.

<!-- ### Deep Dive: For-loops

`For` views are views that, given an array of data items, render a variable number of children.
There are three different flavors of `For` loops. The simplest, and least efficient, is the
`index()` loop. This loop simply renders each item at its index position in the array. The reason
this is inefficient is that the array may have insertions and deletions since the previous render
cycle. Thus, if element #2 becomes element #3, then the for loop will just blindly overwrite any
existing display nodes at position #3, destroying any nodes that don't match and building new
nodes in their place.

The next type is `.keyed()`, which is a bit smarter: it takes an additional function closure which
produces a unique key for each array element. The keys can be any data type, so long as they are
clonable and equals-comparable. The algorithm then attempts to match the old array nodes with the
new ones using an LCS (Longest Common Substring) matching algorithm. This means that as array
elements shift around, it will re-use the display nodes from the previous render, minimizing the
amount of churn. Any insertions or deletions will be detected, and the nodes in those positions
built or razed as appropriate.

Finally, there is `.each()`, which treats the actual array data as the key. This doesn't require
the extra closure argument, but requires that your array data implement `Clone` and `PartialEq`. -->

<!-- ### Deep-Dive: NodeSpans

Even though the view state graph is frequently reconstructed, it's "shape" is relatively stable,
unlike the display graph. For example, a `For` element may generate varying numbers of children
in the display graph, but each new iteration of the view state graph will have a `For` node in
the same relative location.

A helper class which is used by views is `NodeSpan`, which is kind of like a "rope" for Bevy
Entities. The `.build()` method of each `View` produces exactly one `NodeSpan`, however that span
may contain zero, one, or a varying number of entities that represent child nodes in the display
tree. `NodeSpan`s are also stored along with the view State in ECS components. This list of entities
is flattened before it is attached to the parent entity.

To illustrate how this works, consider the following example: Say a presenter produces a sequence
of three elements, where the second element is a "For" element. This means that the output of
`.build()` will produce three `NodeSpans`, but the middle `NodeSpan` will contain a varying number
of entities based on the data passed to the `For`. For a list of n items passed to `For`, the total
number of entities for the presenter will be n + 2. As the for loop reacts to changes in the length
of the array, it will always know where in the flat list of entities those changes will go. -->

# Bibliography

- [Xilem: an architecture for UI in Rust](https://raphlinus.github.io/rust/gui/2022/05/07/ui-architecture.html)
- [Building a reactive library from scratch](https://dev.to/ryansolid/building-a-reactive-library-from-scratch-1i0p)
