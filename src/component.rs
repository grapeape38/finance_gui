extern crate gio;
extern crate gtk;

extern crate hyper;
use crate::gui::{AppPtr};

use gtk::{prelude::*, Widget, Container, Button, Label};
use std::iter::FromIterator;
use std::rc::Rc;
use std::marker::PhantomData;

use std::collections::{HashMap, HashSet};

struct Callback {

}

trait WidgetFactory {
    fn new(&self, attributes: &HashMap<String, String>) -> Widget;
}

pub struct GTKWidget<T: WidgetExt> { 
    phantom: PhantomData<T>
}

impl<T: WidgetExt> GTKWidget<T> {
    fn new() -> Self {
        GTKWidget { phantom: PhantomData }
    }
}

impl WidgetFactory for GTKWidget<Button> {
    fn new(&self, attributes: &HashMap<String, String>) -> Widget {
        if let Some(ref label) = attributes.get("label") {
            gtk::Button::new_with_label(label).upcast::<Widget>()
        }
        else {
            gtk::Button::new().upcast::<Widget>()
        }
    }
}

impl WidgetFactory for GTKWidget<Label> {
    fn new(&self, attributes: &HashMap<String, String>) -> Widget {
        gtk::Label::new(attributes.get("mnemonic").map(|s| &**s)).upcast::<Widget>()
    }
}

pub struct MyWidget {
    attributes: HashMap<String, String>,
    callbacks: HashMap<String, Callback>,
    widget: Box<dyn WidgetFactory> 
}

impl MyWidget {
    fn with_attributes(mut self, attributes: HashMap<String, String>) -> Self {
        self.attributes = attributes;
        self
    }
    fn new(&self) -> Widget {
        self.widget.new(&self.attributes)
    }
    fn get_or_make(&self, name: &String, wmap: &mut WidgetMap) -> Widget {
        wmap.get_mut(name).unwrap_or(&mut Some(self.new())).take().unwrap()
    }
}

pub enum Component {
    NonLeaf(Node),
    Leaf(MyWidget)
}

struct Node {
    widget: Option<MyWidget>,
    children: HashMap<String, Component>
}

impl Node {
    fn with_child(mut self, child: (String, Component)) -> Self {
        self.children.insert(child.0, child.1);
        self
    }
}

pub type WidgetMap = HashMap<String, Option<Widget>>;

impl Component {
    fn new_leaf(factory: Box<WidgetFactory>) -> Component {
        Component::Leaf(MyWidget {
            attributes: HashMap::new(),
            callbacks: HashMap::new(),
            widget: factory
        })
    }

    fn hide_highest_widgets(&self, wmap: &WidgetMap) {
        match self {
            Component::Leaf(_) => { }
            Component::NonLeaf(node) => {
                node.children.iter().for_each(|(name, child)| {
                    match child {
                        Component::NonLeaf(child_node) => {
                            if child_node.widget.is_some() {
                                if let Some(ref child_widget) = wmap.get(name) {
                                    child_widget.as_ref().unwrap().hide();
                                }
                            }
                            else { 
                                child.hide_highest_widgets(wmap);
                            }
                        }
                        Component::Leaf(_) => {
                            if let Some(ref child_widget) = wmap.get(name) {
                                child_widget.as_ref().unwrap().hide();
                            }
                        }
                    } 
                });
            }
        }
    } 

    fn add_or_show_widgets(&self, container: &Container, wmap: &mut WidgetMap) {
        match self {
            Component::Leaf(_) => { }
            Component::NonLeaf(node) => {
                node.children.iter().for_each(|(name, child)| {
                    match child {
                        Component::NonLeaf(child_node) => {
                            if let Some(ref child_widget) = child_node.widget {
                                let gtk_widget = child_widget.get_or_make(name, wmap); 
                                let new_cont = gtk_widget.downcast_ref::<Container>().unwrap();
                                child.add_or_show_widgets(new_cont, wmap);
                                gtk_widget.show();
                                wmap.insert(name.clone(), Some(gtk_widget));
                            }
                            else { 
                                child.add_or_show_widgets(container, wmap);
                            }
                        }
                        Component::Leaf(child_widget) => {
                            if let Some(ref gtk_widget) = wmap.get(name) {
                                gtk_widget.as_ref().unwrap().show();
                            }
                            else {
                                wmap.insert(name.clone(), Some(child_widget.new()));
                            }
                        }
                    } 
                });
            }
        }
    }
     fn render_diff(&self, comp_old: Option<&Component>, container: &Container, wmap: &mut WidgetMap)
    {
        if let Some(comp_old) = comp_old {
            match comp_old {
                Component::NonLeaf(other_node) => {
                    match self {
                        Component::Leaf(_) => { //other is non leaf, you are leaf, remove all other's children
                            comp_old.hide_highest_widgets(wmap);
                            self.add_or_show_widgets(container, wmap);
                        }
                        Component::NonLeaf(my_node) => { //case both non leafs
                            other_node.children.iter().for_each(|(name, v)| {
                                if !my_node.children.contains_key(name) {
                                    v.hide_highest_widgets(wmap);
                                }
                                else { //common node, recurse
                                    let ref my_child = my_node.children[name];
                                    if let Some(ref child_widget) = my_node.widget {
                                        let gtk_widget = child_widget.get_or_make(name, wmap); 
                                        let new_cont = gtk_widget.downcast_ref::<Container>().unwrap();
                                        my_child.render_diff(Some(v), new_cont, wmap);
                                        wmap.insert(name.clone(), Some(gtk_widget));
                                    }
                                    else {
                                        my_child.render_diff(Some(v), container, wmap);
                                    }
                                }
                            });
                            my_node.children.iter().for_each(|(name, v)| {
                                if !other_node.children.contains_key(name) { //add all new nodes
                                    v.add_or_show_widgets(container, wmap);
                                }
                            });
                        }
                    }
                }
                Component::Leaf(_) => {
                    match self {
                        Component::NonLeaf(_) => { //you are non leaf, other is leaf, remove all other's children
                            comp_old.hide_highest_widgets(wmap);
                            self.add_or_show_widgets(container, wmap);
                        }
                        _ => {} //will never compare two leaves
                    }
                }
            }
        }
        else { //empty previous state
            self.add_or_show_widgets(container, wmap);
        }
    }
}


pub trait ToComponent {
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

pub fn create_tree<T>(v: Vec<T>, state: AppPtr, container: Option<MyWidget>) -> Component
        where T: ToComponent
{
    let children = HashMap::from_iter(
        v.into_iter().map(|f| 
            (stringify!(f).to_string(), f.to_component(Rc::clone(&state)))
        ));
    Component::NonLeaf(Node {
        widget: container,
        children
    })
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
}*/

