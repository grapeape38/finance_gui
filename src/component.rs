extern crate gio;
extern crate gtk;

extern crate hyper;
use crate::gui::{AppPtr};
use crate::datamodel::{AsyncCallback, poll_response};

use gtk::{prelude::*, Widget, Container, Button, Label};
use std::iter::FromIterator;
use std::rc::Rc;
use hyper::rt::Future;
use std::marker::PhantomData;
use serde_json::Value;

use std::collections::{HashMap};

pub trait CallbackT {
    fn do_cb(&self, app: &AppPtr, widget: &Widget);
}

impl<F> CallbackT for AsyncCallback<F> 
where F: Future<Item=Value, Error=String> + Send + 'static
{
    fn do_cb(&self, app: &AppPtr, _: &Widget) {
        let app_c = Rc::clone(app);
        let req_type = self.req_type.clone();
        self.make_call_async(&app_c);
        timeout_add_seconds(1, move || {
            poll_response(Rc::clone(&app_c), &req_type)
        });
    }
}

fn call<T: gtk::Cast + gtk::IsA<Widget>>(cb: &Rc<CallbackT>, app: &AppPtr) -> Box<Fn(&T) + 'static>
{
    let app_2 = Rc::clone(app);
    let cb_2 = Rc::clone(cb);
    Box::new(move |w: &T| {
        let widget = w.upcast_ref::<Widget>();
        cb_2.do_cb(&app_2, widget);
    })
}

pub trait WidgetFactory {
    fn make(&self, info: &WidgetInfo, app: &AppPtr) -> Widget;
}

pub struct Factory<W: WidgetExt> {
    phantom: PhantomData<W> 
}

impl<W: WidgetExt> Factory<W> {
    pub fn new() -> Self { Factory { phantom: PhantomData } }
}

impl WidgetFactory for Factory<Button> {
    fn make(&self, info: &WidgetInfo, app: &AppPtr) -> Widget {
        let button = match info.attributes.get("label") {
            Some(label) => Button::new_with_label(label),
            None => Button::new()
        };
        if let Some(callback) = info.callbacks.get("clicked") {
            button.connect_clicked(call(callback, app));
        }
        button.upcast::<Widget>()
    }
}

impl WidgetFactory for Factory<Label> {
    fn make(&self, info: &WidgetInfo, _: &AppPtr) -> Widget {
        gtk::Label::new(info.attributes.get("mnemonic").map(|s| &**s)).upcast::<Widget>()
    }
}

pub struct WidgetInfo {
    attributes: HashMap<&'static str, &'static str>,
    callbacks: HashMap<&'static str, Rc<CallbackT>>,
    factory: Box<dyn WidgetFactory> 
}

impl WidgetInfo {
    fn with_attributes(mut self, attributes: HashMap<&'static str, &'static str>) -> Self {
        self.attributes = attributes;
        self
    }
    fn with_callback(mut self, cb_type: &'static str, cb: Rc<CallbackT>) -> Self {
        self.callbacks.insert(cb_type, cb);
        self
    }
    fn make_widget(&self, app: &AppPtr) -> Widget {
        self.factory.make(self, app)
    }
}

pub enum Component {
    NonLeaf(Node),
    Leaf(WidgetInfo)
}

pub struct Node {
    widget: Option<WidgetInfo>,
    children: HashMap<&'static str, Component>
}

impl Node {
    fn with_child(mut self, child: (&'static str, Component)) -> Self {
        self.children.insert(child.0, child.1);
        self
    }
}

pub type WidgetMap = HashMap<&'static str, Widget>;

fn add_parent_maybe(widget: &Widget, container: &Container) {
    if container.upcast_ref::<Widget>() != widget.get_parent().as_ref().unwrap_or(widget) {
        container.add(widget);
    }
}

impl Component {
    pub fn new_leaf(factory: Box<WidgetFactory>) -> Component {
        Component::Leaf(WidgetInfo {
            attributes: HashMap::new(),
            callbacks: HashMap::new(),
            factory 
        })
    }
    pub fn new_node<T>(v: Vec<T>, state: AppPtr, container: Option<WidgetInfo>) -> Component
            where T: ToComponent
    {
        let children = HashMap::from_iter(
            v.into_iter().map(|f| 
                (stringify!(f), f.to_component(Rc::clone(&state)))
            ));
        Component::NonLeaf(Node {
            widget: container,
            children
        })
    }

    pub fn with_attributes(self, attributes: HashMap<&'static str, &'static str>) -> Self {
        match self {
            Component::Leaf(widget_info) => {
                Component::Leaf(widget_info.with_attributes(attributes))
            }
            Component::NonLeaf(node) => {
                if let Some(widget_info) = node.widget {
                    Component::NonLeaf(Node {
                        widget: Some(widget_info.with_attributes(attributes)),
                        children: node.children
                    })
                }
                else {
                    Component::NonLeaf(node)
                }
            }
        }
    }

    pub fn with_callback(self, cb_type: &'static str, callback: Rc<CallbackT>) -> Self {
        match self {
            Component::Leaf(widget_info) => {
                Component::Leaf(widget_info.with_callback(cb_type, callback))
            }
            Component::NonLeaf(node) => {
                if let Some(widget_info) = node.widget {
                    Component::NonLeaf(Node {
                        widget: Some(widget_info.with_callback(cb_type, callback)),
                        children: node.children
                    })
                }
                else {
                    Component::NonLeaf(node)
                }
            }
        }
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

    fn add_or_show_widgets(&self, container: &Container, wmap: &mut WidgetMap, app: &AppPtr) {
        match self {
            Component::Leaf(_) => { }
            Component::NonLeaf(node) => {
                node.children.iter().for_each(|(name, child)| {
                    match child {
                        Component::NonLeaf(child_node) => {
                            if let Some(ref child_widget) = child_node.widget {
                                let gtk_widget = wmap.remove(name).unwrap_or(child_widget.make_widget(app));
                                add_parent_maybe(&gtk_widget, container);
                                let new_cont = gtk_widget.downcast_ref::<Container>().unwrap();
                                child.add_or_show_widgets(new_cont, wmap, app);
                                gtk_widget.show();
                                wmap.insert(name.clone(), gtk_widget);
                            }
                            else { 
                                child.add_or_show_widgets(container, wmap, app);
                            }
                        }
                        Component::Leaf(child_widget) => {
                            if let Some(ref gtk_widget) = wmap.get(name) {
                                add_parent_maybe(&gtk_widget, container);
                                gtk_widget.show();
                            }
                            else {
                                wmap.insert(name.clone(), child_widget.make_widget(app));
                            }
                        }
                    } 
                });
            }
        }
    }
    pub fn render_diff(&self, comp_old: Option<&Component>, container: &Container, wmap: &mut WidgetMap, app: &AppPtr)
    {
        if let Some(comp_old) = comp_old {
            match comp_old {
                Component::NonLeaf(other_node) => {
                    match self {
                        Component::Leaf(_) => { //other is non leaf, you are leaf, remove all other's children
                            comp_old.hide_highest_widgets(wmap);
                            self.add_or_show_widgets(container, wmap, app);
                        }
                        Component::NonLeaf(my_node) => { //case both non leafs
                            other_node.children.iter().for_each(|(name, v)| {
                                if !my_node.children.contains_key(name) {
                                    v.hide_highest_widgets(wmap);
                                }
                                else { //common node, recurse
                                    let ref my_child = my_node.children[name];
                                    if let Some(ref child_widget) = my_node.widget {
                                        let gtk_widget = wmap.remove(name).unwrap_or(child_widget.make_widget(app));
                                        let new_cont = gtk_widget.downcast_ref::<Container>().unwrap();
                                        my_child.render_diff(Some(v), new_cont, wmap, app);
                                        wmap.insert(name.clone(), gtk_widget);
                                    }
                                    else {
                                        my_child.render_diff(Some(v), container, wmap, app);
                                    }
                                }
                            });
                            my_node.children.iter().for_each(|(name, v)| {
                                if !other_node.children.contains_key(name) { //add all new nodes
                                    v.add_or_show_widgets(container, wmap, app);
                                }
                            });
                        }
                    }
                }
                Component::Leaf(_) => {
                    match self {
                        Component::NonLeaf(_) => { //you are non leaf, other is leaf, remove all other's children
                            comp_old.hide_highest_widgets(wmap);
                            self.add_or_show_widgets(container, wmap, app);
                        }
                        _ => {} //will never compare two leaves
                    }
                }
            }
        }
        else { //empty previous state
            self.add_or_show_widgets(container, wmap, app);
        }
    }
}


pub trait ToComponent {
    fn to_component(self, state: AppPtr) -> Component;
}

pub type ComponentFn = fn(AppPtr) -> Component;

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
