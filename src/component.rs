extern crate gio;
extern crate gtk;

extern crate hyper;
use crate::gui::{AppPtr};
use crate::datamodel::{AsyncCallback, poll_response};

use gtk::{prelude::*, Widget, Container, Button, Window, Label};
use std::iter::FromIterator;
use std::rc::Rc;
use hyper::rt::Future;
use std::marker::PhantomData;
use serde_json::Value;

use std::collections::{HashMap};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

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

pub struct MyWidgetInfo {
    widget: Option<Widget>,
    factory: Box<dyn WidgetFactory> 
}

impl MyWidgetInfo {
    pub fn new(factory: Box<dyn WidgetFactory>) -> MyWidgetInfo {
        MyWidgetInfo {
            widget: None,
            factory
        }
    }
    fn get_or_make(&mut self, info: &WidgetInfo, app: &AppPtr) -> &Widget {
        match self.widget {
            Some(ref w) => w,
            None => {
                self.widget = Some(self.factory.make(info, app));
                self.widget.as_ref().unwrap()
            }
        }
    }
    fn get(&self) -> Option<&Widget> {
        self.widget.as_ref()
    }
    pub fn set(&mut self, w: Widget) {
        self.widget = Some(w);
    }
    fn get_container(&self) -> Option<&Container> {
        self.widget.as_ref().map(|c| c.downcast_ref::<Container>().unwrap())
    }
}

#[derive(Hash, PartialEq, Eq, Clone)]
pub enum EWidget {
    SignInButton,
    SignInLabel,
    GetTransButton,
    SignedInLabel, 
    MainWindow
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

impl WidgetFactory for Factory<Window> {
    fn make(&self, _: &WidgetInfo, _: &AppPtr) -> Widget {
        /*window.set_title("First GTK+ Program");
        window.set_border_width(10);
        window.set_position(gtk::WindowPosition::Center);
        window.set_default_size(350, 70);*/
        Window::new(gtk::WindowType::Toplevel).upcast::<Widget>()
    }
}

pub struct WidgetInfo {
    attributes: HashMap<&'static str, &'static str>,
    callbacks: HashMap<&'static str, Rc<CallbackT>>,
    id: EWidget
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
}

pub enum Component {
    NonLeaf(Node),
    Leaf(WidgetInfo)
}

pub struct Node {
    widget: Option<WidgetInfo>,
    id: u64,
    children: HashMap<u64, Component>
}

pub type WidgetMap = HashMap<EWidget, MyWidgetInfo>;

fn add_parent_maybe(widget: &Widget, container: &Container) {
    if container.upcast_ref::<Widget>() != widget.get_parent().as_ref().unwrap_or(widget) {
        container.add(widget);
    }
}

impl Component {
    pub fn new_leaf(id: EWidget) -> Component {
        Component::Leaf(WidgetInfo {
            attributes: HashMap::new(),
            callbacks: HashMap::new(),
            id
        })
    }
    pub fn new_node<T>(v: Vec<T>, state: AppPtr, container: Option<WidgetInfo>, id: &'static str) -> Component
            where T: ToComponent
    {
        let children = HashMap::from_iter(
            v.into_iter().map(|f| {
                let comp = f.to_component(Rc::clone(&state));
                match comp {
                    Component::Leaf(ref widget_info) => (widget_info.id.clone() as u64, comp),
                    Component::NonLeaf(ref node) => (node.id, comp)
                }
            })
        );
        let mut s = DefaultHasher::new();
        id.hash(&mut s);
        Component::NonLeaf(Node {
            widget: container,
            id: s.finish(),
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
                        id: node.id,
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
                        id: node.id,
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
                node.children.iter().for_each(|(_, child)| {
                    match child {
                        Component::NonLeaf(child_node) => {
                            if let Some(ref widget_info) = child_node.widget {
                                wmap[&widget_info.id].get().unwrap().hide();
                            }
                            else { 
                                child.hide_highest_widgets(wmap);
                            }
                        }
                        Component::Leaf(widget_info) => {
                            wmap[&widget_info.id].get().unwrap().hide();
                        }
                    } 
                });
            }
        }
    } 

    fn add_or_show_widgets(&self, container_id: &EWidget, wmap: &mut WidgetMap, app: &AppPtr) {
        match self {
            Component::Leaf(_) => { }
            Component::NonLeaf(node) => {
                node.children.iter().for_each(|(_, child)| {
                    match child {
                        Component::NonLeaf(child_node) => {
                            if let Some(ref widget_info) = child_node.widget {
                                {
                                    let gtk_widget = wmap.get_mut(&widget_info.id).unwrap().get_or_make(widget_info, app); 
                                    let new_cont = gtk_widget.downcast_ref::<Container>().unwrap();
                                    add_parent_maybe(&gtk_widget, new_cont);
                                }
                                child.add_or_show_widgets(&widget_info.id, wmap, app);
                                wmap.get(&widget_info.id).unwrap().get().unwrap().show();
                            }
                            else { 
                                child.add_or_show_widgets(container_id, wmap, app);
                            }
                        }
                        Component::Leaf(widget_info) => {
                            let gtk_widget = wmap.get_mut(&widget_info.id).unwrap().get_or_make(widget_info, app); 
                            let cont = wmap[container_id].get_container().unwrap();
                            add_parent_maybe(&gtk_widget, cont);
                            gtk_widget.show();
                        }
                    } 
                });
            }
        }
        let cont = wmap[container_id].get_container().unwrap();
        if !cont.is_visible() {
            cont.show();
        }
    }

    pub fn render_diff(&self, comp_old: Option<&Component>, container_id: &EWidget, wmap: &mut WidgetMap, app: &AppPtr)
    {
        if let Some(comp_old) = comp_old {
            match comp_old {
                Component::NonLeaf(other_node) => {
                    match self {
                        Component::Leaf(_) => { //other is non leaf, you are leaf, remove all other's children
                            comp_old.hide_highest_widgets(wmap);
                            self.add_or_show_widgets(container_id, wmap, app);
                        }
                        Component::NonLeaf(my_node) => { //case both non leafs
                            let new_cont = my_node.widget.as_ref().map(|w| &w.id).unwrap_or(container_id);
                            other_node.children.iter().for_each(|(id, v)| {
                                if !my_node.children.contains_key(id) {
                                    v.hide_highest_widgets(wmap);
                                }
                                else { //common node, recurse
                                    let ref my_child = my_node.children[id];
                                    my_child.render_diff(Some(v), new_cont, wmap, app);
                                }
                            });
                            my_node.children.iter().for_each(|(id, v)| {
                                if !other_node.children.contains_key(id) { //add all new nodes
                                    v.add_or_show_widgets(container_id, wmap, app);
                                }
                            });
                        }
                    }
                }
                Component::Leaf(_) => {
                    match self {
                        Component::NonLeaf(_) => { //you are non leaf, other is leaf, remove all other's children
                            comp_old.hide_highest_widgets(wmap);
                            self.add_or_show_widgets(container_id, wmap, app);
                        }
                        _ => {} //will never compare two leaves
                    }
                }
            }
        }
        else { //empty previous state
            self.add_or_show_widgets(container_id, wmap, app);
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
