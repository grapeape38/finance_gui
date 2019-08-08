extern crate gio;
extern crate gtk;

extern crate hyper;
use crate::gui::{AppPtr, build_ui};
use crate::datamodel::{AsyncCallback, poll_response};

use gtk::{prelude::*, Widget, Container, Button, Window, Label};
use std::iter::FromIterator;
use std::ops::Deref;
use std::fmt::Debug;
use std::rc::Rc;
use std::mem;
use std::cell::RefCell;
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
        /*app_2.data.borrow_mut().signed_in = true;
        build_ui(Rc::clone(&app_2));*/
        let widget = w.upcast_ref::<Widget>();
        cb_2.do_cb(&app_2, widget);
    })
}

pub struct MyWidgetInfo {
    widget: RefCell<Option<Widget>>,
    factory: Box<dyn WidgetFactory> 
}

impl MyWidgetInfo {
    pub fn new(factory: Box<dyn WidgetFactory>) -> MyWidgetInfo {
        MyWidgetInfo {
            widget: RefCell::new(None),
            factory
        }
    }
    fn get_or_make(&self, info: &WidgetInfo, app: &AppPtr) -> WidgetGuard {
        let widget = match self.widget.borrow_mut().take() {
                Some(w) => w,
                None => {
                    self.factory.make(info, app)
                }
        };
        WidgetGuard { widget_info: self, widget }
    }
    fn get(&self) -> WidgetGuard {
        WidgetGuard{ widget_info: self, widget: self.widget.borrow_mut().take().unwrap() }
    }
    pub fn set(&mut self, w: Widget) {
        *self.widget.borrow_mut() = Some(w);
    }
}

struct WidgetGuard<'a> {
    widget_info: &'a MyWidgetInfo,
    widget: Widget
}

impl<'a> Drop for WidgetGuard<'a> {
    fn drop(&mut self) {
        let widget = mem::replace(&mut self.widget, Label::new(None).upcast::<Widget>());
        *self.widget_info.widget.borrow_mut() = Some(widget);
    }
}

impl <'a> WidgetGuard <'a> {
    fn to_container(&self) -> &Container {
        self.widget.downcast_ref::<Container>().unwrap()
    }
}

impl <'a> Deref for WidgetGuard<'a> {
    type Target = Widget;
    fn deref(&self) -> &Widget {
        &self.widget
    }
}

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
pub enum EWidget {
    SignInButton,
    SignInLabel,
    GetTransButton,
    SignedInFrame, 
    MainBox,
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
        gtk::Label::new(info.attributes.get("mnemonic").map(|s| &s[..])).upcast::<Widget>()
    }
}

impl WidgetFactory for Factory<Window> {
    fn make(&self, _: &WidgetInfo, _: &AppPtr) -> Widget {
        Window::new(gtk::WindowType::Toplevel).upcast::<Widget>()
    }
}

impl WidgetFactory for Factory<gtk::Box> {
    fn make(&self, _: &WidgetInfo, _: &AppPtr) -> Widget {
        gtk::Box::new(gtk::Orientation::Vertical, 10).upcast::<Widget>()
    }
}

impl WidgetFactory for Factory<gtk::Frame> {
    fn make(&self, info: &WidgetInfo, _: &AppPtr) -> Widget {
        gtk::Frame::new(info.attributes.get("label").map(|s| &s[..])).upcast::<Widget>()
    }
}

pub struct WidgetInfo {
    attributes: HashMap<&'static str, String>,
    callbacks: HashMap<&'static str, Rc<CallbackT>>,
    id: EWidget
}

impl WidgetInfo {
    fn new(id: EWidget) -> Self {
        WidgetInfo {
            attributes: HashMap::new(),
            callbacks: HashMap::new(),
            id
        }
    }
    fn with_attributes(mut self, attributes: HashMap<&'static str, String>) -> Self {
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

fn add_child_maybe(widget: &Widget, container: &Container) {
    if container.upcast_ref::<Widget>() != widget.get_parent().as_ref().unwrap_or(widget) {
        container.add(widget);
    }
}

fn remove_child_maybe(child: &Widget, container: &Container) {
    if container.upcast_ref::<Widget>() == child.get_parent().as_ref().unwrap_or(child)  {
        container.remove(child);
    }
}

fn remove_child_maybe_id(child_id: &EWidget, parent_id: &EWidget, wmap: &WidgetMap) {
    if parent_id != child_id {
        let parent = wmap[parent_id].get();
        let child = wmap[child_id].get();
        remove_child_maybe(&(*child), parent.to_container());
    }
}

impl Component {
    pub fn new_leaf(id: EWidget) -> Component {
        Component::Leaf(WidgetInfo::new(id))
    }
    pub fn new_node<T>(v: Vec<T>, state: AppPtr, container_id: Option<EWidget>, id: &'static str) -> Component
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
        let container = container_id.map(|c| WidgetInfo::new(c));
        Component::NonLeaf(Node {
            widget: container,
            id: s.finish(),
            children
        })
    }

    pub fn with_attributes(self, attributes: HashMap<&'static str, String>) -> Self {
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
    fn remove_highest_widgets(&self, container_id: &EWidget, wmap: &WidgetMap) {
        match self {
            Component::Leaf(widget_info) => { 
                println!("Removing leaf widget {:?}", widget_info.id);
                remove_child_maybe_id(&widget_info.id, container_id, wmap);
            }
            Component::NonLeaf(node) => {
                node.children.iter().for_each(|(_, child)| {
                    match child {
                        Component::NonLeaf(child_node) => {
                            if let Some(ref widget_info) = child_node.widget {
                                println!("Removing container {:?}", widget_info.id);
                                remove_child_maybe_id(&widget_info.id, container_id, wmap);
                            }
                            else { 
                                child.remove_highest_widgets(container_id, wmap);
                            }
                        }
                        Component::Leaf(widget_info) => {
                            println!("Removing leaf widget {:?}", widget_info.id);
                            remove_child_maybe_id(&widget_info.id, container_id, wmap);
                        }
                    } 
                });
            }
        }
    } 

    fn hide_highest_widgets(&self, wmap: &WidgetMap) {
        match self {
            Component::Leaf(widget_info) => { wmap[&widget_info.id].get().hide() }
            Component::NonLeaf(node) => {
                node.children.iter().for_each(|(_, child)| {
                    match child {
                        Component::NonLeaf(child_node) => {
                            if let Some(ref widget_info) = child_node.widget {
                                println!("Hiding container {:?}", widget_info.id);
                                wmap[&widget_info.id].get().hide();
                            }
                            else { 
                                child.hide_highest_widgets(wmap);
                            }
                        }
                        Component::Leaf(widget_info) => {
                            println!("Hiding leaf widget {:?}", widget_info.id);
                            wmap[&widget_info.id].get().hide();
                        }
                    } 
                });
            }
        }
    } 

    fn add_or_show_widgets(&self, container_id: &EWidget, wmap: &WidgetMap, app: &AppPtr) {
        println!("Adding to {:?}", container_id);
        match self {
            Component::Leaf(widget_info) => { 
                println!("Adding child {:?} to container {:?}", widget_info.id, container_id);
                let gtk_widget = wmap[&widget_info.id].get_or_make(widget_info, app); 
                let parent_guard = wmap[container_id].get();
                add_child_maybe(&gtk_widget, parent_guard.to_container());
                gtk_widget.show();
            }
            Component::NonLeaf(node) => {
                let mut i = 0;
                node.children.iter().for_each(|(_, child)| {
                    i+=1;
                    println!("Child: {}", i);
                    match child {
                        Component::NonLeaf(child_node) => {
                            if let Some(ref widget_info) = child_node.widget {
                                {
                                    println!("Adding child {:?} to container {:?}", widget_info.id, container_id);
                                    let gtk_widget = wmap[&widget_info.id].get_or_make(widget_info, app); 
                                    let parent_guard =  wmap[container_id].get();
                                    add_child_maybe(&gtk_widget, parent_guard.to_container());
                                }
                                child.add_or_show_widgets(&widget_info.id, wmap, app);
                                wmap[&widget_info.id].get().show();
                            }
                            else { 
                                child.add_or_show_widgets(container_id, wmap, app);
                            }
                        }
                        Component::Leaf(_) => {
                            child.add_or_show_widgets(container_id, wmap, app);
                        }
                    } 
                });
            }
        }
        let guard = wmap[container_id].get();
        let cont = guard.to_container();
        if !cont.is_visible() {
            cont.show();
        }
    }

    pub fn render_diff(&self, comp_old: Option<&Component>, container_id: &EWidget, wmap: &WidgetMap, app: &AppPtr)
    {
        if let Some(comp_old) = comp_old {
            match comp_old {
                Component::NonLeaf(other_node) => {
                    match self {
                        Component::Leaf(_) => { //other is non leaf, you are leaf, remove all other's children
                            comp_old.remove_highest_widgets(container_id, wmap);
                            //comp_old.hide_highest_widgets(wmap);
                            self.add_or_show_widgets(container_id, wmap, app);
                        }
                        Component::NonLeaf(my_node) => { //case both non leafs
                            other_node.children.iter().for_each(|(id, v)| {
                                println!("Comparing other child {:?} to my children", id);
                                if !my_node.children.contains_key(id) {
                                    //v.hide_highest_widgets(wmap);
                                    v.remove_highest_widgets(container_id, wmap);
                                }
                                else { //common node, recurse
                                    println!("Common child");
                                    let my_child = &my_node.children[id];
                                    let new_cont = match my_child {
                                        Component::Leaf(_) => container_id,
                                        Component::NonLeaf(child_node) => child_node.widget.as_ref().map(|w| &w.id).unwrap_or(container_id)
                                    };
                                    my_child.render_diff(Some(v), new_cont, wmap, app);
                                }
                            });
                            my_node.children.iter().for_each(|(id, v)| {
                                println!("Comparing my child {:?} to other children", id);
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
                            //comp_old.hide_highest_widgets(wmap);
                            comp_old.remove_highest_widgets(container_id, wmap);
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
