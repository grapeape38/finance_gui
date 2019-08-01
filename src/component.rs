extern crate gio;
extern crate gtk;

extern crate hyper;
use crate::gui::{AppPtr};
use crate::datamodel::{poll_response};

use gtk::{prelude::*, Widget, Container, Button, Label};
use std::iter::FromIterator;
use std::rc::Rc;
use std::marker::PhantomData;

use std::collections::{HashMap};

struct Callback {
    //FnOnce<T: Widget>(T);
//    callback: Box<FnOnce()>
}

trait CallbackT {
    fn call<T: WidgetExt>(&self, widget: T);
}

fn get_callback_async<T: WidgetExt, C: CallbackT + 'static>(cb: C, app: &AppPtr) -> impl FnOnce(T) + 'static {
    let app_c = Rc::clone(app);
    move |widget| {
        cb.call(widget);
    }
}


/*impl Callback {
    fn new<T, F>(&mut self, f: F) where T: WidgetExt, F: FnOnce(T) -> () {
        self.callback = Box::new(f);
    }
}*/

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
    fn make_widget(&self) -> Widget {
        self.widget.new(&self.attributes)
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

pub type WidgetMap = HashMap<String, Widget>;

impl Component {
    fn new_leaf(factory: Box<WidgetFactory>) -> Component {
        Component::Leaf(MyWidget {
            attributes: HashMap::new(),
            callbacks: HashMap::new(),
            widget: factory
        })
    }
    pub fn new_node<T>(v: Vec<T>, state: AppPtr, container: Option<MyWidget>) -> Component
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

    fn hide_highest_widgets(&self, wmap: &WidgetMap) {
        match self {
            Component::Leaf(_) => { }
            Component::NonLeaf(node) => {
                node.children.iter().for_each(|(name, child)| {
                    match child {
                        Component::NonLeaf(child_node) => {
                            if child_node.widget.is_some() {
                                if let Some(ref child_widget) = wmap.get(name) {
                                    child_widget.hide();
                                }
                            }
                            else { 
                                child.hide_highest_widgets(wmap);
                            }
                        }
                        Component::Leaf(_) => {
                            if let Some(ref child_widget) = wmap.get(name) {
                                child_widget.hide();
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
                                let gtk_widget = wmap.remove(name).unwrap_or(child_widget.make_widget());
                                let new_cont = gtk_widget.downcast_ref::<Container>().unwrap();
                                child.add_or_show_widgets(new_cont, wmap);
                                gtk_widget.show();
                                wmap.insert(name.clone(), gtk_widget);
                            }
                            else { 
                                child.add_or_show_widgets(container, wmap);
                            }
                        }
                        Component::Leaf(child_widget) => {
                            if let Some(ref gtk_widget) = wmap.get(name) {
                                gtk_widget.show();
                            }
                            else {
                                wmap.insert(name.clone(), child_widget.make_widget());
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
                                        let gtk_widget = wmap.remove(name).unwrap_or(child_widget.make_widget());
                                        let new_cont = gtk_widget.downcast_ref::<Container>().unwrap();
                                        my_child.render_diff(Some(v), new_cont, wmap);
                                        wmap.insert(name.clone(), gtk_widget);
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
