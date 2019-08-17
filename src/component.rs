extern crate gio;
extern crate gtk;

extern crate hyper;
use crate::gui::{AppPtr};
use crate::datamodel::{CallbackFn};
use crate::ewidget::*;

use gtk::{prelude::*, Widget, Container};
use std::ops::{Deref};
use std::rc::Rc;
use std::cell::RefCell;

use std::collections::{HashMap};

pub fn widget_call<T: gtk::Cast + gtk::IsA<Widget>>(cb: &Rc<CallbackFn>, app: &AppPtr) -> Box<Fn(&T) + 'static>
{
    let app_2 = Rc::clone(app);
    let cb_2 = Rc::clone(cb);
    Box::new(move |_: &T| {
        /*app_2.data.borrow_mut().signed_in = true;
        build_ui(Rc::clone(&app_2));*/
        //let widget = w.upcast_ref::<Widget>();
        cb_2(Rc::clone(&app_2));
    })
}

pub struct MyWidgetInfo {
    wmap: RefCell<HashMap<String, Widget>>,
    factory: Box<dyn WidgetFactory> 
}

impl MyWidgetInfo {
    pub fn new(factory: Box<dyn WidgetFactory>) -> MyWidgetInfo {
        MyWidgetInfo {
            wmap: RefCell::new(HashMap::new()),
            factory
        }
    }
    fn get_or_make<'a>(&'a self, id: &'a str, info: &WidgetInfo, app: &AppPtr) -> WidgetGuard {
        let widget = match self.wmap.borrow_mut().remove(id) {
                Some(w) => Some(w),
                None => {
                    Some(self.factory.make(info, app))
                }
        };
        WidgetGuard { widget_info: self, widget, id}
    }
    fn get<'a>(&'a self, id: &'a str) -> WidgetGuard {
        WidgetGuard{ widget_info: self, widget: self.wmap.borrow_mut().remove(id), id}
    }
    pub fn set(&mut self, w: Widget, id: String) {
        self.wmap.borrow_mut().insert(id, w);
    }
}

struct WidgetGuard<'a> {
    widget_info: &'a MyWidgetInfo,
    widget: Option<Widget>,
    id: &'a str 
}

impl<'a> Drop for WidgetGuard<'a> {
    fn drop(&mut self) {
        self.widget_info.wmap.borrow_mut().insert(self.id.to_string(), self.widget.take().unwrap());
    }
}

impl <'a> WidgetGuard <'a> {
    fn to_container(&self) -> &Container {
        self.widget.as_ref().map(|c| c.downcast_ref::<Container>().unwrap()).unwrap()
    }
}

impl <'a> Deref for WidgetGuard<'a> {
    type Target = Widget;
    fn deref(&self) -> &Widget {
        &self.widget.as_ref().unwrap()
    }
}

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
pub enum ComponentID {
    WidgetID(WidgetKey),
    NodeID(&'static str)
}


impl<K: ToKey> From<K> for ComponentID {
    fn from(k: K) -> Self {
        ComponentID::WidgetID(k.to_key())
    }
}

impl From<&'static str> for ComponentID {
    fn from(s: &'static str) -> Self {
        ComponentID::NodeID(s)
    }
}

pub struct WidgetInfo {
    pub attributes: HashMap<&'static str, String>,
    pub callbacks: HashMap<&'static str, Rc<CallbackFn>>,
}

impl WidgetInfo {
    fn new() -> Self {
        WidgetInfo {
            attributes: HashMap::new(),
            callbacks: HashMap::new(),
        }
    }
    fn with_attributes(mut self, attributes: HashMap<&'static str, String>) -> Self {
        self.attributes = attributes;
        self
    }
    fn with_callback(mut self, cb_type: &'static str, cb: Rc<CallbackFn>) -> Self {
        self.callbacks.insert(cb_type, cb);
        self
    }
}

pub struct Component {
    widget: Option<WidgetInfo>,
    id: ComponentID,
    children: Children 
}

struct Children {
    m: HashMap<ComponentID, Component>,
    v: Vec<ComponentID>
}

impl Children {
    fn new() -> Self {
        Children { m: HashMap::new(), v: Vec::new() }
    }
}

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

impl Component {
    pub fn empty(id: &'static str) -> Component {
        Component {
            widget: None,
            id: id.into(),
            children: Children::new()
        }
    }
    pub fn with_attributes(self, attributes: HashMap<&'static str, String>) -> Self {
        Component {
            widget: self.widget.map(|w| w.with_attributes(attributes)),
            ..self
        }
    }

    pub fn with_callback(self, cb_type: &'static str, callback: Rc<CallbackFn>) -> Self {
        Component {
            widget: self.widget.map(|w| w.with_callback(cb_type, callback)),
            ..self
        }
    }

    fn remove_highest_widgets(&self, container_id: &WidgetKey, app: &AppPtr) {
        let wmap = &app.widgets;
        if let ComponentID::WidgetID(ref id) = self.id {
            if container_id != id {
                let parent = wmap[&container_id.0].get(&container_id.1);
                let child = wmap[&id.0].get(&id.1);
                println!("Removing child {:?} from container {:?}", id, container_id);
                remove_child_maybe(&(*child), parent.to_container());
            }
        }
        else {
            self.children.m.iter().for_each(|(id, child)| {
                println!("Removing child {:?} from container {:?}", id, container_id);
                child.remove_highest_widgets(container_id, app);
            });
        }
    } 

    fn hide_highest_widgets(&self, wmap: &WidgetMap) {
        if let ComponentID::WidgetID(ref id) = self.id {
            wmap[&id.0].get(&id.1).hide();
        }
        else {
            self.children.m.iter().for_each(|(_, child)| {
                child.hide_highest_widgets(wmap);
            });
        }
    }
        
    fn add_or_show_widgets(&self, container_id: &WidgetKey, app: &AppPtr) {
        println!("On component: {:?}, adding to container: {:?}", self.id, container_id);
        let wmap = &app.widgets;
        let mut new_cont_id = container_id;
        if let ComponentID::WidgetID(ref id) = self.id {
            if container_id != id {
                new_cont_id = id;
                println!("Adding child {:?} to container {:?}", id, container_id);
                if let Some(ref info) = self.widget {
                    let gtk_widget = wmap[&id.0].get_or_make(&id.1, info, app);
                    let parent_guard = wmap[&container_id.0].get(&container_id.1);
                    add_child_maybe(&(*gtk_widget), parent_guard.to_container());
                    gtk_widget.show();
                }
            }
        }
        self.children.v.iter().for_each(|id| {
            self.children.m[id].add_or_show_widgets(new_cont_id, app);
        });
        let guard = wmap[&container_id.0].get(&container_id.1);
        let cont = guard.to_container();
        if !cont.is_visible() {
            cont.show();
        }
    }

    pub fn render_diff(&self, comp_old: Option<&Component>, container_id: &WidgetKey, app: &AppPtr)
    {
        if let Some(comp_old) = comp_old {
            let mut new_cont_id = container_id;
            if let ComponentID::WidgetID(ref id) = self.id {
                new_cont_id = id;
            }
            println!("Comparing {:?} to {:?}", self.id, comp_old.id);
            if comp_old.id != self.id {
                comp_old.remove_highest_widgets(container_id, app);
            }
            else {
                comp_old.children.v.iter().for_each(|old_id| {
                    let old_child = &comp_old.children.m[old_id];
                    if let Some(new_child) = self.children.m.get(old_id) {
                        new_child.render_diff(Some(old_child), new_cont_id, app);
                    }
                    else {
                        old_child.remove_highest_widgets(new_cont_id, app);
                    }
                });
                self.children.m.iter().for_each(|(id, child)| {
                    if !comp_old.children.m.contains_key(id) {
                        child.add_or_show_widgets(new_cont_id, app);
                    }
                });
            }
        }
        else { //empty previous state
            self.add_or_show_widgets(container_id, app);
        }
    }
}

pub fn new_leaf<K: ToKey>(id: K) -> Component {
    Component {
        widget: Some(WidgetInfo::new()),
        children: Children::new(),
        id: ComponentID::WidgetID(id.to_key())
    }
}

pub fn new_node<K: Into<ComponentID> + Clone>(v: Vec<Component>, id: K) -> Component
{
    let mut children = Children::new();
    v.into_iter().for_each(|comp| {
        let id = comp.id.clone();
        children.m.insert(comp.id.clone(), comp);
        children.v.push(id)
    });
    let widget = match id.clone().into() {
        ComponentID::WidgetID(_) => Some(WidgetInfo::new()),
        ComponentID::NodeID(_) => None
    };
    Component {
        widget,
        id: id.into(),
        children
    }
}


