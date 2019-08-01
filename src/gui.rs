extern crate gio;
extern crate gtk;

extern crate hyper;
use crate::datamodel;

use datamodel::{DataPtr, DataModel, RequestStatus, sign_in};
use gio::prelude::*;
use gtk::{prelude::*, Widget, Container};
use std::env::args;
use std::iter::FromIterator;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

macro_rules! make_widget {
    ($widget: ident) => {
        Rc::new($widget.upcast::<gtk::Widget>())
    }
}

pub struct AppState {
    pub data: DataModel,
    pub async_request: Arc<Mutex<Request>>,
    ui_tree: Option<Component>,
    gui: GuiState
}

pub type AppPtr = Rc<RefCell<AppState>>;

struct GuiState {
    window: Rc<gtk::ApplicationWindow>,
    widgets: HashMap<String, Rc<Widget>>,
}

enum Component {
    NonLeaf(Node),
    Leaf(Rc<Widget>)
}

struct Node {
    container: Option<Rc<Container>>,
    children: HashMap<String, Component>
}

impl Component {
    fn hide_highest_widgets(&self/*, container: &Container*/) {
        match self {
            Component::Leaf(widget) => {
                //container.remove(*widget);
                widget.hide();
            }
            Component::NonLeaf(node) => {
                if let Some(ref my_container) = node.container {
                    //container.remove(my_container.upcast_ref::<Widget>());
                    my_container.hide();
                }
                else {
                    node.children.iter().for_each(|(_, v)| {
                        v.hide_highest_widgets();
                    });
                }
            }
        }
    } 

    fn render_diff<C>(&self, comp_old: Option<&Component>, container: &Rc<C>)
        where C: ContainerExt + IsA<Widget>
    {
        if let Some(comp_old) = comp_old {
            match comp_old {
                Component::NonLeaf(other_node) => {
                    match self {
                        Component::Leaf(_) => { //other is non leaf, you are leaf, remove all other's children
                            comp_old.hide_highest_widgets();
                            self.add_all(container);
                        }
                        Component::NonLeaf(my_node) => { //case both non leafs
                            other_node.children.iter().for_each(|(name, v)| {
                                if !my_node.children.contains_key(name) {
                                    v.hide_highest_widgets();
                                }
                                else { //common node, recurse
                                    let ref my_child = my_node.children[name];
                                    if let Some(ref new_container) = my_node.container {
                                        my_child.render_diff(Some(v), new_container);
                                    }
                                    else {
                                        my_child.render_diff(Some(v), container);
                                    }
                                }
                            });
                            my_node.children.iter().for_each(|(name, v)| {
                                if !other_node.children.contains_key(name) { //add all new nodes
                                    v.add_all(container);
                                }
                            });
                        }
                    }
                }
                Component::Leaf(_) => {
                    match self {
                        Component::NonLeaf(_) => { //you are non leaf, other is leaf, remove all other's children
                            comp_old.hide_highest_widgets();
                            self.add_all(container);
                        }
                        _ => {} //will never compare two leaves
                    }
                }
            }
        }
        else { //empty previous state
            self.add_all(container);
        }
    }

    fn add_all<C>(&self, container: &Rc<C>) where C: ContainerExt + IsA<Widget> {
        match self {
            Component::Leaf(widget) => {
                if !widget.is_ancestor(&**container) {
                    container.add(&**widget);
                }
                widget.show();
            }
            Component::NonLeaf(node) => {
                if let Some(ref new_container) = node.container {
                    node.children.iter().for_each(|(_, v)| {
                        v.add_all(new_container);
                    });
                    new_container.upcast_ref::<Widget>().show();
                }
                else {
                    node.children.iter().for_each(|(_, v)| {
                        v.add_all(container);
                    });
                }
            }
        }
    }
}

trait ToComponent {
    fn to_component(self, state: AppPtr) -> Component;
}

type ComponentFn = fn(AppPtr) -> Component;

impl ToComponent for ComponentFn {
    fn to_component(self, state: AppPtr) -> Component {
        self(state)
    }
}

impl ToComponent for Component {
    fn to_component(self, _: AppPtr) -> Component {
       self 
    }
}

fn create_tree<'a, T>(v: Vec<T>, state: AppPtr, container: Option<Rc<Container>>) -> Component
        where T: ToComponent
{
    let children = HashMap::from_iter(
        v.into_iter().map(|f| 
            (stringify!(f).to_string(), f.to_component(Rc::clone(&state)))
        ));
    Component::NonLeaf(Node {
        container,
        children
    })
} 

fn sign_in_page(state: AppPtr) -> Component {
    let button = Rc::clone(&state.borrow().gui.widgets["sign_in_button"]);
    button.downcast_ref::<gtk::Button>().unwrap().connect_clicked(move |_| {
        println!("Clicked!");
        sign_in(Rc::clone(&state));
    });
    Component::Leaf(button)
}

fn user_page(state: AppPtr) -> Component {
    let label = Rc::clone(&state.borrow().gui.widgets["signed_in_label"]);
    let button = Rc::clone(&state.borrow().gui.widgets["get_trans_button"]);
    let state_c = Rc::clone(&state);
    button.downcast_ref::<gtk::Button>().unwrap().connect_clicked(move |_| {
        println!("Clicked!");
        sign_in(Rc::clone(&state_c));
    });
    let v = vec![Component::Leaf(label), Component::Leaf(button)];
    create_tree(v, state, None)
}

fn main_app(state: AppPtr) -> Component {
    let mut v = Vec::new();
    if state.borrow().data.signed_in {
        v.push(sign_in_page as ComponentFn);
    }
    else {
        v.push(user_page as ComponentFn);
    }
    create_tree(v, state, None)
}

fn create_widgets() -> HashMap<String, Rc<Widget>> {
    let mut widgets = HashMap::new();
    let button = gtk::Button::new_with_label("Sign in");
    widgets.insert("sign_in_button".to_string(), make_widget!(button));
    let label = gtk::Label::new_with_mnemonic(Some("You are now signed in!"));
    widgets.insert("signed_in_label".to_string(), make_widget!(label));
    let get_trans_button = gtk::Button::new_with_label("Get transactions!");
    widgets.insert("get_trans_button".to_string(), make_widget!(get_trans_button));
    widgets
}

impl GuiState {
    fn new(app: &gtk::Application) -> GuiState {
        let window = gtk::ApplicationWindow::new(app);
        window.set_title("First GTK+ Program");
        window.set_border_width(10);
        window.set_position(gtk::WindowPosition::Center);
        window.set_default_size(350, 70);
        let widgets = create_widgets();
        GuiState {
            window: Rc::new(window),
            widgets
        }
    }
}

pub fn build_ui(state: AppPtr) {
    let app_tree = main_app(Rc::clone(&state));
    app_tree.render_diff(state.borrow().ui_tree.as_ref(), &state.borrow().gui.window);
    state.borrow_mut().ui_tree = Some(app_tree);
}

pub fn run_app() {
    let application =
        gtk::Application::new(Some("com.github.gtk-rs.examples.basic"), Default::default())
            .expect("Initialization failed...");
    application.connect_activate(move |app| {
        let app_state = AppState {
            data: DataModel::new(),
            async_request: Request::arc_none(),
            ui_tree: None,
            gui: GuiState::new(app)
        };
        let app_ptr = Rc::new(RefCell::new(app_state));
        build_ui(app_ptr);
    });

    application.run(&args().collect::<Vec<_>>());
}