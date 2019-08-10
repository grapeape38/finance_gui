extern crate gio;
extern crate gtk;

extern crate hyper;
use crate::gui::{AppPtr};
use crate::datamodel::{CallbackFn};

use gtk::{prelude::*, Widget, Container, Button, Window, Label};
use std::iter::FromIterator;
use std::ops::Deref;
use std::rc::Rc;
use std::cell::RefCell;
use std::marker::PhantomData;

use std::collections::{HashMap};

fn call<T: gtk::Cast + gtk::IsA<Widget>>(cb: &Rc<CallbackFn>, app: &AppPtr) -> Box<Fn(&T) + 'static>
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
                Some(w) => Some(w),
                None => {
                    Some(self.factory.make(info, app))
                }
        };
        WidgetGuard { widget_info: self, widget }
    }
    fn get(&self) -> WidgetGuard {
        WidgetGuard{ widget_info: self, widget: self.widget.borrow_mut().take() }
    }
    pub fn set(&mut self, w: Widget) {
        *self.widget.borrow_mut() = Some(w);
    }
}

struct WidgetGuard<'a> {
    widget_info: &'a MyWidgetInfo,
    widget: Option<Widget>
}

impl<'a> Drop for WidgetGuard<'a> {
    fn drop(&mut self) {
        *self.widget_info.widget.borrow_mut() = self.widget.take();
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
pub enum EWidget {
    SignInButton,
    SignInLoading,
    ErrorPage,
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

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
pub enum ComponentID {
    WidgetID(EWidget),
    NodeID(&'static str)
}

pub struct WidgetInfo {
    attributes: HashMap<&'static str, String>,
    callbacks: HashMap<&'static str, Rc<CallbackFn>>,
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
    children: HashMap<ComponentID, Component>
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

impl Component {
    

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

    fn remove_highest_widgets(&self, container_id: &EWidget, wmap: &WidgetMap) {
        if let ComponentID::WidgetID(ref id) = self.id {
            if container_id != id {
                let parent = wmap[container_id].get();
                let child = wmap[id].get();
                remove_child_maybe(&(*child), parent.to_container());
            }
        }
        else {
            self.children.iter().for_each(|(_, child)| {
                child.remove_highest_widgets(container_id, wmap);
            });
        }
    } 

    fn hide_highest_widgets(&self, wmap: &WidgetMap) {
        if let ComponentID::WidgetID(ref id) = self.id {
            wmap[id].get().hide();
        }
        else {
            self.children.iter().for_each(|(_, child)| {
                child.hide_highest_widgets(wmap);
            });
        }
    }
        
    fn add_or_show_widgets(&self, container_id: &EWidget, wmap: &WidgetMap, app: &AppPtr) {
        println!("On component: {:?}, adding to container: {:?}", self.id, container_id);
        let mut new_cont_id = container_id;
        if let ComponentID::WidgetID(ref id) = self.id {
            if container_id != id {
                new_cont_id = id;
                println!("Adding child {:?} to container {:?}", id, container_id);
                if let Some(ref info) = self.widget {
                    let gtk_widget = wmap[id].get_or_make(info, app);
                    let parent_guard = wmap[container_id].get();
                    add_child_maybe(&(*gtk_widget), parent_guard.to_container());
                    gtk_widget.show();
                }
            }
        }
        let mut i = 0;
        self.children.iter().for_each(|(_, child)| {
            i+=1;
            println!("Child: {}", i);
            child.add_or_show_widgets(new_cont_id, wmap, app);
        });
        let guard = wmap[container_id].get();
        let cont = guard.to_container();
        if !cont.is_visible() {
            cont.show();
        }
    }

    pub fn render_diff(&self, comp_old: Option<&Component>, container_id: &EWidget, wmap: &WidgetMap, app: &AppPtr)
    {
        if let Some(comp_old) = comp_old {
            let mut new_cont_id = container_id;
            if let ComponentID::WidgetID(ref id) = self.id {
                new_cont_id = id;
            }
            println!("Comparing {:?} to {:?}", self.id, comp_old.id);
            if comp_old.id != self.id {
                comp_old.remove_highest_widgets(container_id, wmap);
            }
            else {
                comp_old.children.iter().for_each(|(id, old_child)| {
                    if let Some(new_child) = self.children.get(id) {
                        new_child.render_diff(Some(old_child), new_cont_id, wmap, app);
                    }
                    else {
                        old_child.remove_highest_widgets(new_cont_id, wmap);
                    }
                });
                self.children.iter().for_each(|(id, child)| {
                    if !comp_old.children.contains_key(id) {
                        child.add_or_show_widgets(new_cont_id, wmap, app);
                    }
                });
            }
        }
        else { //empty previous state
            self.add_or_show_widgets(container_id, wmap, app);
        }
    }
}

pub fn new_leaf(id: EWidget) -> Component {
    Component {
        widget: Some(WidgetInfo::new()),
        children: HashMap::new(),
        id: ComponentID::WidgetID(id)
    }
}

pub fn new_node(v: Vec<Component>, id: ComponentID) -> Component
{
    let children = HashMap::from_iter(
        v.into_iter().map(|c| {
            (c.id.clone(), c)
        })
    );
    let widget = match id {
        ComponentID::WidgetID(_) => Some(WidgetInfo::new()),
        ComponentID::NodeID(_) => None
    };
    Component {
        widget,
        id,
        children
    }
}

