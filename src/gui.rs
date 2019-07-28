extern crate gio;
extern crate gtk;

extern crate hyper;
use crate::datamodel;

use datamodel::{DataPtr, create_model, sign_in};
use gio::prelude::*;
use gtk::{prelude::*, Widget, Container};
use std::env::args;
use std::iter::FromIterator;
use std::rc::Rc;
use std::collections::{HashMap, HashSet};

pub struct AppState<'a> {
    data: DataPtr,
    ui_tree: Option<Component<'a>>,
    gui: &'a GuiState
}

struct GuiState {
    window: gtk::ApplicationWindow,
    widgets: HashMap<String, Widget>,
}

enum Component<'a> {
    NonLeaf(Node<'a>),
    Leaf(&'a Widget)
}

struct Node<'a> {
    container: Option<&'a Container>,
    children: HashMap<String, Component<'a>>
}

impl<'a> Component<'a> {
    fn empty() -> Component<'a> {
        Component::NonLeaf(Node {
            container: None,
            children: HashMap::new()
        })
    }
    fn hide_highest_widgets(&self/*, container: &Container*/) {
        match self {
            Component::Leaf(widget) => {
                //container.remove(*widget);
                widget.hide();
            }
            Component::NonLeaf(node) => {
                if let Some(my_container) = node.container {
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

    fn render_diff(&self, comp_old: Option<&Component<'a>>, container: &Container) {
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
                                    let new_container = my_node.container.unwrap_or(container); 
                                    let ref my_child = my_node.children[name];
                                    my_child.render_diff(Some(v), new_container);
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

    fn add_all(&self, container: &Container) {
        match self {
            Component::Leaf(widget) => {
                if !widget.is_ancestor(container) {
                    container.add(*widget);
                }
                widget.show();
            }
            Component::NonLeaf(node) => {
                let new_container = node.container.unwrap_or(container); 
                node.children.iter().for_each(|(_, v)| {
                    v.add_all(new_container);
                });
                if node.container.is_some() { //now show container
                    new_container.upcast_ref::<Widget>().show();
                }
            }
        }
    }
}

trait ToComponent<'a, 'b> {
    fn to_component(self, state: &'a AppState<'b>) -> Component<'b>;
}

type ComponentFn = for<'a, 'b> fn(&'a AppState<'b>) -> Component<'b>;

impl<'a, 'b> ToComponent<'a, 'b> for ComponentFn {
    fn to_component(self, state: &'a AppState<'b>) -> Component<'b> {
        self(state)
    }
}

impl<'a, 'b> ToComponent<'a, 'b> for Component<'b> {
    fn to_component(self, _: &'a AppState<'b>) -> Component<'b> {
       self 
    }
}

fn create_tree<'a, 'b, T>(v: Vec<T>, state: &'a AppState<'b>, container: Option<&'b Container>) -> Component<'b>
        where T: ToComponent<'a, 'b>
{
    let children = HashMap::from_iter(
        v.into_iter().map(|f| 
            (stringify!(f).to_string(), f.to_component(state))
        ));
    Component::NonLeaf(Node {
        container,
        children
    })
} 

fn sign_in_page<'a, 'b>(state: &'a AppState<'b>) -> Component<'b> {
    let ref button = state.gui.widgets["sign_in_button"];
    let data = Rc::clone(&state.data);
    button.downcast_ref::<gtk::Button>().unwrap().connect_clicked(move |_| {
        println!("Clicked!");
        sign_in(Rc::clone(&data));
    });
    Component::Leaf(button)
}

fn user_page<'a, 'b>(state: &'a AppState<'b>) -> Component<'b> {
    let ref label = state.gui.widgets["signed_in_label"];
    let ref button = state.gui.widgets["get_trans_button"];
    let data = Rc::clone(&state.data);
    button.downcast_ref::<gtk::Button>().unwrap().connect_clicked(move |_| {
        println!("Clicked!");
        sign_in(Rc::clone(&data));
    });
    let v = vec![Component::Leaf(label), Component::Leaf(button)];
    create_tree(v, state, None)
}

fn main_app<'a, 'b>(state: &'a AppState<'b>) -> Component<'b> {
    let mut v = Vec::new();
    if state.data.borrow().signed_in {
        v.push(sign_in_page as ComponentFn);
    }
    else {
        v.push(user_page as ComponentFn);
    }
    create_tree(v, state, None)
}

fn create_widgets() -> HashMap<String, Widget> {
    let mut widgets = HashMap::new();
    let button = gtk::Button::new_with_label("Sign in");
    widgets.insert("sign_in_button".to_string(), button.upcast::<gtk::Widget>());
    let label = gtk::Label::new_with_mnemonic(Some("You are now signed in!"));
    widgets.insert("signed_in_label".to_string(), label.upcast::<gtk::Widget>());
    let get_trans_button = gtk::Button::new_with_label("Get transactions!");
    widgets.insert("get_trans_button".to_string(), get_trans_button.upcast::<gtk::Widget>());
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
            window,
            widgets
        }
    }
}

impl<'a, 'b> AppState<'b> {
    pub fn build_ui(&'a mut self) {
        //let app = self.main_app();
        let app = main_app(self);
        //app.render_diff(self.ui_tree.as_ref(), self.window.upcast_ref::<gtk::Container>());
        self.ui_tree = Some(app);
    }
}

pub fn run_app() {
    let application =
        gtk::Application::new(Some("com.github.gtk-rs.examples.basic"), Default::default())
            .expect("Initialization failed...");

    let data_ptr = create_model(); 
    application.connect_activate(move |app| {
        let mut app_state = AppState {
            data: Rc::clone(&data_ptr),
            ui_tree: None,
            gui: &GuiState::new(app)
        };
        app_state.build_ui();
    });

    application.run(&args().collect::<Vec<_>>());
}