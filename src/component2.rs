extern crate gio;
extern crate gtk;

use crate::xml_test::*;
use gtk::{prelude::*, Widget, Container, Builder};

use std::collections::{HashMap};
use std::iter::FromIterator;

macro_rules! class(
    { $type: ty, $class: literal } => {
        impl ComponentT for $type {
            fn class() -> &'static str { $class }
        }
    }
);

pub trait ComponentT {
    fn class() -> &'static str;
}

pub fn new_comp<T: ComponentT>(id: &'static str) -> Component {
    Component { class: T::class().to_string(), id: id.to_string(), ..Component::empty() }
}

class!(gtk::Box, "GtkBox");
class!(gtk::Button, "GtkButton");
class!(gtk::Frame, "GtkFrame");

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

pub fn add_child_maybe(widget: &Widget, container: &Container) {
    if container.upcast_ref::<Widget>() != widget.get_parent().as_ref().unwrap_or(widget) {
        container.add(widget);
    }
    if !widget.is_visible() {
        widget.show_all();
    }
}

struct CompIter<'a> {
    stack: Vec<&'a Component>
}

impl<'a> CompIter<'a> { 
    fn new(comp: &'a Component) -> CompIter<'a> { 
        CompIter { stack: vec![comp] } 
    } 
}

impl<'a> Iterator for CompIter<'a> {
    type Item = &'a Component;
    fn next(&mut self) -> Option<&'a Component> {
        match self.stack.pop() {
            None => None,
            Some(comp) => {
                for c in comp.children.v.iter() {
                    self.stack.push(&comp.children.m[c]);
                }
                Some(comp)
            }
        }
    }
}

impl Component {
    pub fn empty() -> Component {
        Component {
            class: String::new(),
            id: String::new(),
            properties: HashMap::new(),
            children: Children::new()
        }
    }
    
    fn iter(&self) -> CompIter {
        CompIter::new(self)
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

    pub fn build(&self, app: &AppPtr) {
        let xml_str = self.to_xml_string().expect("Error serializing to xml");
        let builder = Builder::new_from_string(&xml_str[..]);
        for c in self.iter() {
            app.widget_map.borrow_mut().insert(c.id.clone(), builder.get_object(&c.id).
                expect(&format!("Could not get widget {} from xml", c.id)[..]));
        }
    }

    pub fn remove_self_widget(&self, wmap: &mut WidgetMap) {
        println!("Removing widget {}", self.id);
        let widget = &wmap[&self.id];
        if let Some(parent) = widget.get_parent() {
            parent.downcast_ref::<Container>().unwrap().remove(widget);
        }
        for c in self.iter() {
            wmap.remove(&c.id);
        }
    }

    pub fn add_child_widget(&self, id: &String, app: &AppPtr) {
        self.children.m[id].build(app);
        let parent = &app.widget_map.borrow()[&self.id];
        let child = &app.widget_map.borrow()[id];
        add_child_maybe(&child, parent.downcast_ref::<Container>().unwrap());
    }

    pub fn render_diff(&self, comp_old: &Component, app: &AppPtr)
    {
        println!("Comparing {:?} to {:?}", self.id, comp_old.id);
        comp_old.children.v.iter().for_each(|old_id| {
            let old_child = &comp_old.children.m[old_id];
            if let Some(new_child) = self.children.m.get(old_id) {
                new_child.render_diff(old_child, app);
            }
            else {
                old_child.remove_self_widget(&mut app.widget_map.borrow_mut());
            }
        });
        self.children.v.iter().for_each(|id| {
            if !comp_old.children.m.contains_key(id) {
                self.add_child_widget(id, app);
            }
        });
    }
}



