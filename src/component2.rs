extern crate gio;
extern crate gtk;

use crate::xml_parse::*;

use gtk::{prelude::*, Widget, Container, Builder};
use std::ops::{Deref};
use std::rc::Rc;
use std::cell::RefCell;

use std::collections::{HashMap};
use std::iter::FromIterator;

macro_rules! class(
    { $type: ty, $class: literal } => {
        impl ComponentT for $type {
            fn class() -> &'static str { $class }
        }
    }
);

pub struct AppState {
    widget_map: RefCell<WidgetMap>,
    ui_tree: RefCell<Component>
}

pub type AppPtr = Rc<AppState>;

pub trait ComponentT {
    fn class() -> &'static str;
}

pub fn new_comp<T: ComponentT>(id: &'static str) -> Component {
    Component { class: T::class().to_string(), id: id.to_string(), ..Component::empty() }
}

class!(gtk::Box, "GtkBox");
class!(gtk::Button, "GtkButton");

pub struct Component {
    pub class: String,
    pub id: String,
    pub properties: HashMap<String, String>,
    pub children: Children 
}

pub struct Children {
    pub v: Vec<String>,
    pub m: HashMap<String, Component>
}

impl Children {
    fn new() -> Children {
        Children { v: Vec::new(), m: HashMap::new() }
    }
}

fn add_child_maybe(widget: &Widget, container: &Container) {
    if container.upcast_ref::<Widget>() != widget.get_parent().as_ref().unwrap_or(widget) {
        container.add(widget);
    }
}

/*fn remove_child_maybe(child: &Widget, container: &Container) {
    if container.upcast_ref::<Widget>() == child.get_parent().as_ref().unwrap_or(child)  {
        container.remove(child);
    }
}*/

fn remove_widget_maybe(id: &String, app: &AppPtr) {
    {
        let child = &app.widget_map.borrow()[id];
        if let Some(parent) = child.get_parent() {
            parent.downcast_ref::<Container>().unwrap().remove(child);
        }
    }
    app.widget_map.borrow_mut().remove(id);
}

type WidgetMap = HashMap<String, Widget>;

impl Component {
    pub fn empty() -> Component {
        Component {
            class: String::new(),
            id: String::new(),
            properties: HashMap::new(),
            children: Children::new()
        }
    }

    pub fn with_props(mut self, props: HashMap<&str, &str>) -> Component {
        self.properties = HashMap::from_iter(props.iter().map(|(k,v)| (k.to_string(), v.to_string())));
        self
    }

    pub fn with_children(mut self, children: Vec<Component>) -> Component {
        children.into_iter().for_each(|c| {
            self.children.v.push(c.id.clone());
            self.children.m.insert(c.id.clone(), c);
        });
        self
    }

    pub fn build(&self) -> Widget {
        let xml_str = self.to_xml_string().expect("Error serializing to xml");
        let builder = Builder::new_from_string(&xml_str[..]);
        builder.get_object(&self.id).expect("Error getting root object")
    }

    pub fn add_child_widget(&self, id: &String, app: &AppPtr) {
        let child = self.children.m[id].build();
        {
            let parent = &app.widget_map.borrow()[&self.id];
            add_child_maybe(&child, parent.downcast_ref::<Container>().unwrap());
        }
        app.widget_map.borrow_mut().insert(id.clone(), child);
    }

    pub fn render_diff(&self, comp_old: &Component, app: &AppPtr)
    {
        println!("Comparing {:?} to {:?}", self.id, comp_old.id);
        if comp_old.id != self.id {
            remove_widget_maybe(&comp_old.id, app);
        }
        else {
            comp_old.children.v.iter().for_each(|old_id| {
                let old_child = &comp_old.children.m[old_id];
                if let Some(new_child) = self.children.m.get(old_id) {
                    new_child.render_diff(old_child, app);
                }
                else {
                    remove_widget_maybe(&old_child.id, app);
                }
            });
            self.children.m.iter().for_each(|(id, child)| {
                if !comp_old.children.m.contains_key(id) {
                    child.add_child_widget(id, app);
                }
            });
        }
    }
}

