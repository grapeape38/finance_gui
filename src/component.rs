extern crate gio;
extern crate gtk;

extern crate hyper;
use crate::datamodel;

use datamodel::{DataPtr, DataModel, Request, sign_in};
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

struct Callback {

}

trait MyWidgetExt {
    fn new(attributes: &HashMap<String, String>) -> Self where Self: Sized;
}

impl MyWidgetExt for gtk::Button {
    fn new(attributes: &HashMap<String, String>) -> gtk::Button {
        if let Some(ref label) = attributes.get("label") {
            gtk::Button::new_with_label(label)
        }
        else {
            gtk::Button::new()
        }
    }
}

impl MyWidgetExt for gtk::Label {
    fn new(attributes: &HashMap<String, String>) -> gtk::Label {
        gtk::Label::new_with_mnemonic(attributes.get("mnemonic").map(|s| &**s))
    }
}

/*struct MyWidget<C: MyWidgetExt> {
    attributes: HashMap<String, String>,
    callbacks: HashSet<String, Callback>
    widget: C
}
impl MyWidgetExt for MyWidget {

}*/

enum Component {
    NonLeaf(Node),
    Leaf(Box<dyn MyWidgetExt>)
}

struct Node {
    widget: Option<MyWidget>,
    children: HashMap<String, Component>
}

type WidgetMap = HashMap<String, Widget>;

impl Component {
    fn hide_highest_widgets(&self, wmap: &WidgetMap) {
        match self {
            Component::Leaf(_) => { }
            Component::NonLeaf(node) => {
                node.children.iter().for_each(|(name, child)| {
                    match child {
                        Component::NonLeaf(child_node) => {
                            if child_node.widget.is_some() {
                                wmap[name].hide();
                            }
                            else { 
                                child.hide_highest_widgets(wmap);
                            }
                        }
                        Component::Leaf(_) => {
                            wmap[name].hide();
                        }
                    } 
                });
            }
        }
    } 
    fn add_or_show_widgets(&self, wmap: &WidgetMap) {


    }
}
/*

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
} */